use crate::{
    deftype, CachedArgs, CachedVals, EvalCached, MapCollection, MapFn, MapQ, Slot,
};
use anyhow::{bail, Result};
use arcstr::{literal, ArcStr};
use graphix_compiler::{
    expr::ExprId,
    typ::{FnType, Type},
    Apply, BindId, BuiltIn, BuiltInInitFn, Event, ExecCtx, Node, Rt, UserEvent,
};
use immutable_chunkmap::map::Map as CMap;
use netidx::subscriber::Value;
use netidx_value::ValArray;
use std::{fmt::Debug, sync::Arc};
use triomphe::Arc as TArc;

impl MapCollection for CMap<Value, Value, 32> {
    fn iter_values(&self) -> impl Iterator<Item = Value> {
        self.into_iter().map(|(k, v)| {
            Value::Array(ValArray::from_iter_exact([k.clone(), v.clone()].into_iter()))
        })
    }

    fn len(&self) -> usize {
        CMap::len(self)
    }

    fn select(v: Value) -> Option<Self> {
        match v {
            Value::Map(m) => Some(m.clone()),
            _ => None,
        }
    }

    fn project(self) -> Value {
        Value::Map(self)
    }

    fn etyp(ft: &FnType) -> Result<Type> {
        match &ft.args[0].typ {
            Type::Map { key, value } => {
                Ok(Type::Tuple(TArc::from_iter([(**key).clone(), (**value).clone()])))
            }
            _ => bail!("expected Map"),
        }
    }
}

#[derive(Debug, Default)]
struct MapImpl;

impl<R: Rt, E: UserEvent> MapFn<R, E> for MapImpl {
    type Collection = CMap<Value, Value, 32>;

    const NAME: &str = "map_map";
    deftype!(
        "core::map",
        "fn(Map<'a, 'b>, fn(('a, 'b)) -> ('c, 'd) throws 'e) -> Map<'c, 'd> throws 'e"
    );

    fn finish(&mut self, slots: &[Slot<R, E>], _: &Self::Collection) -> Option<Value> {
        Some(Value::Map(CMap::from_iter(
            slots
                .iter()
                .map(|s| s.cur.clone().unwrap().cast_to::<(Value, Value)>().unwrap()),
        )))
    }
}

type Map<R, E> = MapQ<R, E, MapImpl>;

#[derive(Debug, Default)]
struct FilterImpl;

impl<R: Rt, E: UserEvent> MapFn<R, E> for FilterImpl {
    type Collection = CMap<Value, Value, 32>;

    const NAME: &str = "map_filter";
    deftype!(
        "core::map",
        "fn(Map<'a, 'b>, fn(('a, 'b)) -> bool throws 'e) -> Map<'a, 'b> throws 'e"
    );

    fn finish(
        &mut self,
        slots: &[Slot<R, E>],
        m: &CMap<Value, Value, 32>,
    ) -> Option<Value> {
        Some(Value::Map(CMap::from_iter(slots.iter().zip(m.into_iter()).filter_map(
            |(p, (k, v))| match p.cur {
                Some(Value::Bool(true)) => Some((k.clone(), v.clone())),
                _ => None,
            },
        ))))
    }
}

type Filter<R, E> = MapQ<R, E, FilterImpl>;

#[derive(Debug, Default)]
struct FilterMapImpl;

impl<R: Rt, E: UserEvent> MapFn<R, E> for FilterMapImpl {
    type Collection = CMap<Value, Value, 32>;

    const NAME: &str = "map_filter_map";
    deftype!(
        "core::map",
        "fn(Map<'a, 'b>, fn(('a, 'b)) -> Option<('c, 'd)> throws 'e) -> Map<'c, 'd> throws 'e"
    );

    fn finish(
        &mut self,
        slots: &[Slot<R, E>],
        _: &CMap<Value, Value, 32>,
    ) -> Option<Value> {
        Some(Value::Map(CMap::from_iter(slots.iter().filter_map(|s| {
            match s.cur.as_ref().unwrap() {
                Value::Null => None,
                v => Some(v.clone().cast_to::<(Value, Value)>().unwrap()),
            }
        }))))
    }
}

type FilterMap<R, E> = MapQ<R, E, FilterMapImpl>;

#[derive(Debug, Default)]
struct LenEv;

impl EvalCached for LenEv {
    const NAME: &str = "map_len";
    deftype!("core::map", "fn(Map<'a, 'b>) -> i64");

    fn eval(&mut self, from: &CachedVals) -> Option<Value> {
        match &from.0[0] {
            Some(Value::Map(m)) => Some(Value::I64(m.len() as i64)),
            Some(_) | None => None,
        }
    }
}

type Len = CachedArgs<LenEv>;

#[derive(Debug, Default)]
struct GetEv;

impl EvalCached for GetEv {
    const NAME: &str = "map_get";
    deftype!("core::map", "fn(Map<'a, 'b>, 'a) -> Option<'b>");

    fn eval(&mut self, from: &CachedVals) -> Option<Value> {
        match (&from.0[0], &from.0[1]) {
            (Some(Value::Map(m)), Some(key)) => {
                Some(m.get(key).cloned().unwrap_or(Value::Null))
            }
            _ => None,
        }
    }
}

type Get = CachedArgs<GetEv>;

#[derive(Debug)]
pub(super) struct Iter {
    id: BindId,
    top_id: ExprId,
}

impl<R: Rt, E: UserEvent> BuiltIn<R, E> for Iter {
    const NAME: &str = "map_iter";
    deftype!("core::map", "fn(Map<'a, 'b>) -> ('a, 'b)");

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|ctx, _, _, _, top_id| {
            let id = BindId::new();
            ctx.rt.ref_var(id, top_id);
            Ok(Box::new(Self { id, top_id }))
        })
    }
}

impl<R: Rt, E: UserEvent> Apply<R, E> for Iter {
    fn update(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> Option<Value> {
        if let Some(Value::Map(m)) = from[0].update(ctx, event) {
            for (k, v) in m.into_iter() {
                let pair = Value::Array(ValArray::from_iter_exact(
                    [k.clone(), v.clone()].into_iter(),
                ));
                ctx.rt.set_var(self.id, pair);
            }
        }
        event.variables.get(&self.id).map(|v| v.clone())
    }

    fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
        ctx.rt.unref_var(self.id, self.top_id)
    }

    fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
        ctx.rt.unref_var(self.id, self.top_id);
        self.id = BindId::new();
        ctx.rt.ref_var(self.id, self.top_id);
    }
}

pub(super) fn register<R: Rt, E: UserEvent>(ctx: &mut ExecCtx<R, E>) -> Result<ArcStr> {
    ctx.register_builtin::<Map<R, E>>()?;
    ctx.register_builtin::<Filter<R, E>>()?;
    ctx.register_builtin::<FilterMap<R, E>>()?;
    ctx.register_builtin::<Len>()?;
    ctx.register_builtin::<Get>()?;
    ctx.register_builtin::<Iter>()?;
    Ok(literal!(include_str!("map.gx")))
}
