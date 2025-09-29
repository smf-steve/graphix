use anyhow::{bail, Result};
use arcstr::ArcStr;
use compact_str::format_compact;
use enumflags2::{bitflags, BitFlags};
use fxhash::FxHashMap;
use graphix_compiler::{
    expr::{ExprId, ModuleResolver},
    node::genn,
    typ::{FnType, Type},
    Apply, BindId, BuiltIn, BuiltInInitFn, Event, ExecCtx, LambdaId, Node, Refs, Rt,
    Scope, UserEvent,
};
use netidx::{path::Path, subscriber::Value};
use netidx_core::utils::Either;
use std::{
    collections::hash_map::Entry,
    fmt::Debug,
    iter,
    sync::{Arc, LazyLock},
};
use triomphe::Arc as TArc;

mod array;
mod core;
mod net;
mod rand;
mod re;
mod str;
#[cfg(test)]
mod test;
mod time;

#[macro_export]
macro_rules! deftype {
    ($scope:literal, $s:literal) => {
        const TYP: ::std::sync::LazyLock<graphix_compiler::typ::FnType> =
            ::std::sync::LazyLock::new(|| {
                let scope =
                    graphix_compiler::expr::ModPath(::netidx::path::Path::from($scope));
                graphix_compiler::expr::parser::parse_fn_type($s)
                    .expect("failed to parse fn type {s}")
                    .scope_refs(&scope)
            });
    };
}

#[macro_export]
macro_rules! arity1 {
    ($from:expr, $updates:expr) => {
        match (&*$from, &*$updates) {
            ([arg], [arg_up]) => (arg, arg_up),
            (_, _) => unreachable!(),
        }
    };
}

#[macro_export]
macro_rules! arity2 {
    ($from:expr, $updates:expr) => {
        match (&*$from, &*$updates) {
            ([arg0, arg1], [arg0_up, arg1_up]) => ((arg0, arg1), (arg0_up, arg1_up)),
            (_, _) => unreachable!(),
        }
    };
}

#[derive(Debug)]
pub struct CachedVals(pub Box<[Option<Value>]>);

impl CachedVals {
    pub fn new<R: Rt, E: UserEvent>(from: &[Node<R, E>]) -> CachedVals {
        CachedVals(from.into_iter().map(|_| None).collect())
    }

    pub fn clear(&mut self) {
        for v in &mut self.0 {
            *v = None
        }
    }

    pub fn update<R: Rt, E: UserEvent>(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> bool {
        from.into_iter().enumerate().fold(false, |res, (i, src)| {
            match src.update(ctx, event) {
                None => res,
                v @ Some(_) => {
                    self.0[i] = v;
                    true
                }
            }
        })
    }

    /// Like update, but return the indexes of the nodes that updated
    /// instead of a consolidated bool
    pub fn update_diff<R: Rt, E: UserEvent>(
        &mut self,
        up: &mut [bool],
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) {
        for (i, n) in from.iter_mut().enumerate() {
            match n.update(ctx, event) {
                None => (),
                v => {
                    self.0[i] = v;
                    up[i] = true
                }
            }
        }
    }

    pub fn flat_iter<'a>(&'a self) -> impl Iterator<Item = Option<Value>> + 'a {
        self.0.iter().flat_map(|v| match v {
            None => Either::Left(iter::once(None)),
            Some(v) => Either::Right(v.clone().flatten().map(Some)),
        })
    }
}

pub trait EvalCached: Debug + Default + Send + Sync + 'static {
    const NAME: &str;
    const TYP: LazyLock<FnType>;

    fn eval(&mut self, from: &CachedVals) -> Option<Value>;
}

#[derive(Debug)]
pub struct CachedArgs<T: EvalCached> {
    cached: CachedVals,
    t: T,
}

impl<R: Rt, E: UserEvent, T: EvalCached> BuiltIn<R, E> for CachedArgs<T> {
    const NAME: &str = T::NAME;
    const TYP: LazyLock<FnType> = T::TYP;

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|_, _, _, from, _| {
            let t = CachedArgs::<T> { cached: CachedVals::new(from), t: T::default() };
            Ok(Box::new(t))
        })
    }
}

impl<R: Rt, E: UserEvent, T: EvalCached> Apply<R, E> for CachedArgs<T> {
    fn update(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> Option<Value> {
        if self.cached.update(ctx, from, event) {
            self.t.eval(&self.cached)
        } else {
            None
        }
    }

    fn sleep(&mut self, _ctx: &mut ExecCtx<R, E>) {
        self.cached.clear()
    }
}

pub trait MapCollection: Debug + Clone + Default + Send + Sync + 'static {
    /// return the length of the collection
    fn len(&self) -> usize;

    /// iterate the collection elements as values
    fn iter_values(&self) -> impl Iterator<Item = Value>;

    /// given a value, return Some if the value is the collection type
    /// we are mapping.
    fn select(v: Value) -> Option<Self>;

    /// given a collection wrap it in a value
    fn project(self) -> Value;

    /// return the element type given the function type
    fn etyp(ft: &FnType) -> Result<Type>;
}

pub trait MapFn<R: Rt, E: UserEvent>: Debug + Default + Send + Sync + 'static {
    type Collection: MapCollection;

    const NAME: &str;
    const TYP: LazyLock<FnType>;

    /// finish will be called when every lambda instance has produced
    /// a value for the updated array. Out contains the output of the
    /// predicate lambda for each index i, and a is the array. out and
    /// a are guaranteed to have the same length. out[i].cur is
    /// guaranteed to be Some.
    fn finish(&mut self, slots: &[Slot<R, E>], a: &Self::Collection) -> Option<Value>;
}

#[derive(Debug)]
pub struct Slot<R: Rt, E: UserEvent> {
    id: BindId,
    pred: Node<R, E>,
    pub cur: Option<Value>,
}

impl<R: Rt, E: UserEvent> Slot<R, E> {
    fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.pred.delete(ctx);
        ctx.cached.remove(&self.id);
        ctx.env.unbind_variable(self.id);
    }
}

#[derive(Debug)]
pub struct MapQ<R: Rt, E: UserEvent, T: MapFn<R, E>> {
    scope: Scope,
    predid: BindId,
    top_id: ExprId,
    mftyp: TArc<FnType>,
    etyp: Type,
    slots: Vec<Slot<R, E>>,
    cur: T::Collection,
    t: T,
}

impl<R: Rt, E: UserEvent, T: MapFn<R, E>> BuiltIn<R, E> for MapQ<R, E, T> {
    const NAME: &str = T::NAME;
    const TYP: LazyLock<FnType> = T::TYP;

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|_ctx, typ, scope, from, top_id| match from {
            [_, _] => Ok(Box::new(Self {
                scope: scope.append(&format_compact!("fn{}", LambdaId::new().inner())),
                predid: BindId::new(),
                top_id,
                etyp: T::Collection::etyp(typ)?,
                mftyp: match &typ.args[1].typ {
                    Type::Fn(ft) => ft.clone(),
                    t => bail!("expected a function not {t}"),
                },
                slots: vec![],
                cur: Default::default(),
                t: T::default(),
            })),
            _ => bail!("expected two arguments"),
        })
    }
}

impl<R: Rt, E: UserEvent, T: MapFn<R, E>> Apply<R, E> for MapQ<R, E, T> {
    fn update(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> Option<Value> {
        let slen = self.slots.len();
        if let Some(v) = from[1].update(ctx, event) {
            ctx.cached.insert(self.predid, v.clone());
            event.variables.insert(self.predid, v);
        }
        let (up, resized) =
            match from[0].update(ctx, event).and_then(|v| T::Collection::select(v)) {
                Some(a) if a.len() == slen => (Some(a), false),
                Some(a) if a.len() < slen => {
                    while self.slots.len() > a.len() {
                        if let Some(mut s) = self.slots.pop() {
                            s.delete(ctx)
                        }
                    }
                    (Some(a), true)
                }
                Some(a) => {
                    while self.slots.len() < a.len() {
                        let (id, node) = genn::bind(
                            ctx,
                            &self.scope.lexical,
                            "x",
                            self.etyp.clone(),
                            self.top_id,
                        );
                        let fargs = vec![node];
                        let fnode = genn::reference(
                            ctx,
                            self.predid,
                            Type::Fn(self.mftyp.clone()),
                            self.top_id,
                        );
                        let pred = genn::apply(
                            fnode,
                            self.scope.clone(),
                            fargs,
                            &self.mftyp,
                            self.top_id,
                        );
                        self.slots.push(Slot { id, pred, cur: None });
                    }
                    (Some(a), true)
                }
                None => (None, false),
            };
        if let Some(a) = up {
            for (s, v) in self.slots.iter().zip(a.iter_values()) {
                ctx.cached.insert(s.id, v.clone());
                event.variables.insert(s.id, v);
            }
            self.cur = a.clone();
            if a.len() == 0 {
                return Some(T::Collection::project(a));
            }
        }
        let init = event.init;
        let mut up = resized;
        for (i, s) in self.slots.iter_mut().enumerate() {
            if i == slen {
                // new nodes were added starting here
                event.init = true;
                if let Entry::Vacant(e) = event.variables.entry(self.predid)
                    && let Some(v) = ctx.cached.get(&self.predid)
                {
                    e.insert(v.clone());
                }
            }
            if let Some(v) = s.pred.update(ctx, event) {
                s.cur = Some(v);
                up = true;
            }
        }
        event.init = init;
        if up && self.slots.iter().all(|s| s.cur.is_some()) {
            self.t.finish(&mut &self.slots, &self.cur)
        } else {
            None
        }
    }

    fn typecheck(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        _from: &mut [Node<R, E>],
    ) -> anyhow::Result<()> {
        let (_, node) =
            genn::bind(ctx, &self.scope.lexical, "x", self.etyp.clone(), self.top_id);
        let fargs = vec![node];
        let ft = self.mftyp.clone();
        let fnode = genn::reference(ctx, self.predid, Type::Fn(ft.clone()), self.top_id);
        let mut node = genn::apply(fnode, self.scope.clone(), fargs, &ft, self.top_id);
        let r = node.typecheck(ctx);
        node.delete(ctx);
        r
    }

    fn refs(&self, refs: &mut Refs) {
        for s in &self.slots {
            s.pred.refs(refs)
        }
    }

    fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
        ctx.cached.remove(&self.predid);
        for sl in &mut self.slots {
            sl.delete(ctx)
        }
    }

    fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.cur = Default::default();
        for sl in &mut self.slots {
            sl.cur = None;
            sl.pred.sleep(ctx);
        }
    }
}

#[bitflags]
#[derive(Clone, Copy)]
#[repr(u64)]
pub enum Module {
    Array,
    NetAndTime,
    Rand,
    Re,
    Str,
}

/// Register selected modules of the standard graphix library
///
/// and return a root module that will load them along with a module resolver
/// that contains the necessary code. You need both of these for the `rt`
/// module.
///
/// Note, core is always included and registered, all the other
/// modules are optional
///
/// # Example
///
/// ```no_run
/// use netidx::{publisher::Publisher, subscriber::Subscriber};
/// use anyhow::Result;
/// use poolshark::global::GPooled;
/// use graphix_compiler::ExecCtx;
/// use graphix_rt::{GXRt, GXConfigBuilder, GXHandle, GXEvent, NoExt};
/// use tokio::sync::mpsc;
/// use enumflags2::BitFlags;
///
/// async fn start_runtime(
///     publisher: Publisher,
///     subscriber: Subscriber,
///     sub: mpsc::Sender<GPooled<Vec<GXEvent<NoExt>>>>
/// ) -> Result<GXHandle<NoExt>> {
///     let mut ctx = ExecCtx::new(GXRt::<NoExt>::new(publisher, subscriber));
///     let (root, mods) = graphix_stdlib::register(&mut ctx, BitFlags::all())?;
///     GXConfigBuilder::default()
///        .ctx(ctx)
///        .root(root)
///        .resolvers(vec![mods])
///        .sub(sub)
///        .build()?
///        .start()
///        .await
/// }
/// ```
pub fn register<R: Rt, E: UserEvent>(
    ctx: &mut ExecCtx<R, E>,
    modules: BitFlags<Module>,
) -> Result<(ArcStr, ModuleResolver)> {
    let mut tbl = FxHashMap::default();
    tbl.insert(Path::from("/core"), core::register(ctx)?);
    let mut root = String::from("pub mod core;\nuse core;\n");
    for module in modules {
        match module {
            Module::Array => {
                root.push_str("pub mod array;\n");
                tbl.insert(Path::from("/array"), array::register(ctx)?);
            }
            Module::NetAndTime => {
                root.push_str("pub mod time;\n");
                tbl.insert(Path::from("/time"), time::register(ctx)?);
                root.push_str("pub mod net;\n");
                tbl.insert(Path::from("/net"), net::register(ctx)?);
            }
            Module::Rand => {
                root.push_str("pub mod rand;\n");
                tbl.insert(Path::from("/rand"), rand::register(ctx)?);
            }
            Module::Re => {
                root.push_str("pub mod re;\n");
                tbl.insert(Path::from("/re"), re::register(ctx)?);
            }
            Module::Str => {
                root.push_str("pub mod str;\n");
                tbl.insert(Path::from("/str"), str::register(ctx)?);
            }
        }
    }
    root.pop();
    root.pop();
    Ok((ArcStr::from(root), ModuleResolver::VFS(tbl)))
}
