use crate::{
    expr::{Expr, ExprId},
    node::{compiler::compile, Cached},
    typ::Type,
    update_args, wrap, CFlag, Event, ExecCtx, Node, Refs, Rt, Scope, Update, UserEvent,
};
use anyhow::Result;
use enumflags2::BitFlags;
use immutable_chunkmap::map::Map as CMap;
use netidx_value::Value;
use triomphe::Arc;

#[derive(Debug)]
pub(crate) struct Map<R: Rt, E: UserEvent> {
    spec: Expr,
    typ: Type,
    keys: Box<[Cached<R, E>]>,
    vals: Box<[Cached<R, E>]>,
}

impl<R: Rt, E: UserEvent> Map<R, E> {
    pub(crate) fn compile(
        ctx: &mut ExecCtx<R, E>,
        flags: BitFlags<CFlag>,
        spec: Expr,
        scope: &Scope,
        top_id: ExprId,
        args: &Arc<[(Expr, Expr)]>,
    ) -> Result<Node<R, E>> {
        let keys = args
            .iter()
            .map(|(k, _)| Ok(Cached::new(compile(ctx, flags, k.clone(), scope, top_id)?)))
            .collect::<Result<_>>()?;
        let vals = args
            .iter()
            .map(|(_, v)| Ok(Cached::new(compile(ctx, flags, v.clone(), scope, top_id)?)))
            .collect::<Result<_>>()?;
        let typ = Type::Map {
            key: Arc::new(Type::empty_tvar()),
            value: Arc::new(Type::empty_tvar()),
        };
        Ok(Box::new(Self { spec, typ, keys, vals }))
    }
}

impl<R: Rt, E: UserEvent> Update<R, E> for Map<R, E> {
    fn update(&mut self, ctx: &mut ExecCtx<R, E>, event: &mut Event<E>) -> Option<Value> {
        if self.keys.is_empty() && event.init {
            return Some(Value::Map(CMap::new()));
        }
        let (kupdated, kdetermined) = update_args!(self.keys, ctx, event);
        let (vupdated, vdetermined) = update_args!(self.vals, ctx, event);
        let (updated, determined) = (kupdated || vupdated, kdetermined && vdetermined);
        if updated && determined {
            let mut m = CMap::new();
            for (k, v) in self.keys.iter().zip(self.vals.iter()) {
                m.insert_cow(
                    k.cached.as_ref().cloned().unwrap(),
                    v.cached.as_ref().cloned().unwrap(),
                );
            }
            Some(Value::Map(m))
        } else {
            None
        }
    }

    fn spec(&self) -> &Expr {
        &self.spec
    }

    fn typ(&self) -> &Type {
        &self.typ
    }

    fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.keys.iter_mut().for_each(|n| n.node.delete(ctx));
        self.vals.iter_mut().for_each(|n| n.node.delete(ctx))
    }

    fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.keys.iter_mut().for_each(|n| n.sleep(ctx));
        self.vals.iter_mut().for_each(|n| n.sleep(ctx))
    }

    fn refs(&self, refs: &mut Refs) {
        self.keys.iter().for_each(|n| n.node.refs(refs));
        self.vals.iter().for_each(|n| n.node.refs(refs))
    }

    fn typecheck(&mut self, ctx: &mut ExecCtx<R, E>) -> Result<()> {
        for n in self.keys.iter_mut().chain(self.vals.iter_mut()) {
            wrap!(n.node, n.node.typecheck(ctx))?
        }
        let ktype = self
            .keys
            .iter()
            .fold(Ok(Type::Bottom), |acc, n| n.node.typ().union(&ctx.env, &acc?));
        let ktype = wrap!(self, ktype)?;
        let vtype = self
            .vals
            .iter()
            .fold(Ok(Type::Bottom), |acc, n| n.node.typ().union(&ctx.env, &acc?));
        let vtype = wrap!(self, vtype)?;
        let rtype = Type::Map { key: Arc::new(ktype), value: Arc::new(vtype) };
        Ok(self.typ.check_contains(&ctx.env, &rtype)?)
    }
}
