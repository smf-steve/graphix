use super::{compiler::compile, wrap_error, Cached};
use crate::{
    expr::{Expr, ExprId, ModPath},
    typ::Type,
    wrap, BindId, Event, ExecCtx, Node, Refs, Rt, Update, UserEvent,
};
use anyhow::{anyhow, bail, Result};
use arcstr::{literal, ArcStr};
use netidx_value::{Typ, Value};
use std::{fmt, sync::LazyLock};
use triomphe::Arc;

macro_rules! compare_op {
    ($name:ident, $op:tt) => {
        #[derive(Debug)]
        pub(crate) struct $name<R: Rt, E: UserEvent> {
            spec: Expr,
            typ: Type,
            lhs: Cached<R, E>,
            rhs: Cached<R, E>,
        }

        impl<R: Rt, E: UserEvent> $name<R, E> {
            pub(crate) fn compile(
                ctx: &mut ExecCtx<R, E>,
                spec: Expr,
                scope: &ModPath,
                top_id: ExprId,
                lhs: &Expr,
                rhs: &Expr
            ) -> Result<Node<R, E>> {
                let lhs = Cached::new(compile(ctx, lhs.clone(), scope, top_id)?);
                let rhs = Cached::new(compile(ctx, rhs.clone(), scope, top_id)?);
                let typ = Type::Primitive(Typ::Bool.into());
                Ok(Box::new(Self { spec, typ, lhs, rhs }))
            }
        }

        impl<R: Rt, E: UserEvent> Update<R, E> for $name<R, E> {
            fn update(
                &mut self,
                ctx: &mut ExecCtx<R, E>,
                event: &mut Event<E>,
            ) -> Option<Value> {
                let lhs_up = self.lhs.update(ctx, event);
                let rhs_up = self.rhs.update(ctx, event);
                if lhs_up || rhs_up {
                    return self.lhs.cached.as_ref().and_then(|lhs| {
                        self.rhs.cached.as_ref().map(|rhs| (lhs $op rhs).into())
                    })
                }
                None
            }

            fn spec(&self) -> &Expr {
                &self.spec
            }

            fn typ(&self) -> &Type {
                &self.typ
            }

            fn refs(&self, refs: &mut Refs) {
                self.lhs.node.refs(refs);
                self.rhs.node.refs(refs);
            }

            fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
                self.lhs.node.delete(ctx);
                self.rhs.node.delete(ctx)
            }

            fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
                self.lhs.node.sleep(ctx);
                self.rhs.node.sleep(ctx)
            }

            fn typecheck(&mut self, ctx: &mut ExecCtx<R, E>) -> Result<()> {
                wrap!(self.lhs.node, self.lhs.node.typecheck(ctx))?;
                wrap!(self.rhs.node, self.rhs.node.typecheck(ctx))?;
                wrap!(
                    self,
                    self.lhs.node.typ().check_contains(&ctx.env, &self.rhs.node.typ())
                )?;
                wrap!(self, self.typ.check_contains(&ctx.env, &Type::boolean()))
            }
        }
    };
}

compare_op!(Eq, ==);
compare_op!(Ne, !=);
compare_op!(Lt, <);
compare_op!(Gt, >);
compare_op!(Lte, <=);
compare_op!(Gte, >=);

macro_rules! bool_op {
    ($name:ident, $op:tt) => {
        #[derive(Debug)]
        pub(crate) struct $name<R: Rt, E: UserEvent> {
            spec: Expr,
            typ: Type,
            lhs: Cached<R, E>,
            rhs: Cached<R, E>,
        }

        impl<R: Rt, E: UserEvent> $name<R, E> {
            pub(crate) fn compile(
                ctx: &mut ExecCtx<R, E>,
                spec: Expr,
                scope: &ModPath,
                top_id: ExprId,
                lhs: &Expr,
                rhs: &Expr
            ) -> Result<Node<R, E>> {
                let lhs = Cached::new(compile(ctx, lhs.clone(), scope, top_id)?);
                let rhs = Cached::new(compile(ctx, rhs.clone(), scope, top_id)?);
                let typ = Type::Primitive(Typ::Bool.into());
                Ok(Box::new(Self { spec, typ, lhs, rhs }))
            }
        }

        impl<R: Rt, E: UserEvent> Update<R, E> for $name<R, E> {
            fn update(
                &mut self,
                ctx: &mut ExecCtx<R, E>,
                event: &mut Event<E>,
            ) -> Option<Value> {
                let lhs_up = self.lhs.update(ctx, event);
                let rhs_up = self.rhs.update(ctx, event);
                if lhs_up || rhs_up {
                    return match (self.lhs.cached.as_ref(), self.rhs.cached.as_ref()) {
                        (Some(Value::Bool(b0)), Some(Value::Bool(b1))) => Some(Value::Bool(*b0 $op *b1)),
                        (_, _) => None
                    }
                }
                None
            }

            fn spec(&self) -> &Expr {
                &self.spec
            }

            fn typ(&self) -> &Type {
                &self.typ
            }

            fn refs(&self, refs: &mut Refs) {
                self.lhs.node.refs(refs);
                self.rhs.node.refs(refs);
            }

            fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
                self.lhs.node.delete(ctx);
                self.rhs.node.delete(ctx)
            }

            fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
                self.lhs.sleep(ctx);
                self.rhs.sleep(ctx)
            }

            fn typecheck(&mut self, ctx: &mut ExecCtx<R, E>) -> Result<()> {
                wrap!(self.lhs.node, self.lhs.node.typecheck(ctx))?;
                wrap!(self.rhs.node, self.rhs.node.typecheck(ctx))?;
                let bt = Type::Primitive(Typ::Bool.into());
                wrap!(self.lhs.node, bt.check_contains(&ctx.env, self.lhs.node.typ()))?;
                wrap!(self.rhs.node, bt.check_contains(&ctx.env, self.rhs.node.typ()))?;
                wrap!(self, self.typ.check_contains(&ctx.env, &Type::boolean()))
            }
        }
    };
}

bool_op!(And, &&);
bool_op!(Or, ||);

#[derive(Debug)]
pub(crate) struct Not<R: Rt, E: UserEvent> {
    spec: Expr,
    typ: Type,
    n: Node<R, E>,
}

impl<R: Rt, E: UserEvent> Not<R, E> {
    pub(crate) fn compile(
        ctx: &mut ExecCtx<R, E>,
        spec: Expr,
        scope: &ModPath,
        top_id: ExprId,
        n: &Expr,
    ) -> Result<Node<R, E>> {
        let n = compile(ctx, n.clone(), scope, top_id)?;
        let typ = Type::Primitive(Typ::Bool.into());
        Ok(Box::new(Self { spec, typ, n }))
    }
}

impl<R: Rt, E: UserEvent> Update<R, E> for Not<R, E> {
    fn update(&mut self, ctx: &mut ExecCtx<R, E>, event: &mut Event<E>) -> Option<Value> {
        self.n.update(ctx, event).and_then(|v| match v {
            Value::Bool(b) => Some(Value::Bool(!b)),
            _ => None,
        })
    }

    fn spec(&self) -> &Expr {
        &self.spec
    }

    fn typ(&self) -> &Type {
        &self.typ
    }

    fn refs(&self, refs: &mut Refs) {
        self.n.refs(refs);
    }

    fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.n.delete(ctx);
    }

    fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.n.sleep(ctx);
    }

    fn typecheck(&mut self, ctx: &mut ExecCtx<R, E>) -> Result<()> {
        wrap!(self.n, self.n.typecheck(ctx))?;
        let bt = Type::Primitive(Typ::Bool.into());
        wrap!(self.n, bt.check_contains(&ctx.env, self.n.typ()))?;
        wrap!(self, self.typ.check_contains(&ctx.env, &Type::boolean()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Op::Add => write!(f, "+"),
            Op::Sub => write!(f, "-"),
            Op::Mul => write!(f, "*"),
            Op::Div => write!(f, "/"),
            Op::Mod => write!(f, "%"),
        }
    }
}

static ARITH_ERR_TAG: ArcStr = literal!("ArithError");
static ARITH_ERR: LazyLock<Type> = LazyLock::new(|| {
    Type::Variant(
        ARITH_ERR_TAG.clone(),
        Arc::from_iter([Type::Primitive(Typ::String.into())]),
    )
});

macro_rules! arith_op {
    ($name:ident, $opn:expr, $op:tt) => {
        #[derive(Debug)]
        pub(crate) struct $name<R: Rt, E: UserEvent> {
            spec: Expr,
            typ: Type,
            id: BindId,
            lhs: Cached<R, E>,
            rhs: Cached<R, E>
        }

        impl<R: Rt, E: UserEvent> $name<R, E> {
            pub(crate) fn compile(
                ctx: &mut ExecCtx<R, E>,
                spec: Expr,
                scope: &ModPath,
                top_id: ExprId,
                lhs: &Expr,
                rhs: &Expr
            ) -> Result<Node<R, E>> {
                let lhs = Cached::new(compile(ctx, lhs.clone(), scope, top_id)?);
                let rhs = Cached::new(compile(ctx, rhs.clone(), scope, top_id)?);
                let typ = Type::empty_tvar();
                let id = ctx.env.lookup_catch(scope)?;
                Ok(Box::new(Self { spec, id, typ, lhs, rhs }))
            }
        }

        impl<R: Rt, E: UserEvent> Update<R, E> for $name<R, E> {
            fn update(&mut self, ctx: &mut ExecCtx<R, E>, event: &mut Event<E>) -> Option<Value> {
                let lhs_up = self.lhs.update(ctx, event);
                let rhs_up = self.rhs.update(ctx, event);
                let lhs = self.lhs.cached.as_ref()?;
                let rhs = self.rhs.cached.as_ref()?;
                if lhs_up || rhs_up {
                    match lhs.clone() $op rhs.clone() {
                        Value::Error(e) => {
                            let e: Value = (ARITH_ERR_TAG.clone(), (*e).clone()).into();
                            let e = wrap_error(&ctx.env, &self.spec, e);
                            ctx.set_var(self.id, Value::Error(Arc::new(e)));
                            None
                        }
                        v => Some(v)
                    }
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

            fn refs(&self, refs: &mut Refs) {
                self.lhs.node.refs(refs);
                self.rhs.node.refs(refs);
            }

            fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
                self.lhs.node.delete(ctx);
                self.rhs.node.delete(ctx);
            }

            fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
                self.lhs.sleep(ctx);
                self.rhs.sleep(ctx);
            }

            fn typecheck(&mut self, ctx: &mut ExecCtx<R, E>) -> Result<()> {
                wrap!(self.lhs.node, self.lhs.node.typecheck(ctx))?;
                wrap!(self.rhs.node, self.rhs.node.typecheck(ctx))?;
                let lhs = self.lhs.node.typ();
                let rhs = self.rhs.node.typ();
                match (lhs.with_deref(|t| t.cloned()), rhs.with_deref(|t| t.cloned())) {
                    (None, None) | (Some(_), Some(_)) => (),
                    (Some(t), None) => { let _ = rhs.contains(&ctx.env, &t); }
                    (None, Some(t)) => { let _ = lhs.contains(&ctx.env, &t); },
                }
                // init types that aren't known by now to Number
                let typ = Type::Primitive(Typ::number());
                wrap!(self.lhs.node, typ.contains(&ctx.env, lhs))?;
                wrap!(self.rhs.node, typ.contains(&ctx.env, rhs))?;
                // Duration and DateTime can be involved in some arith operations however
                let typ = Type::Primitive(Typ::number() | Typ::Duration | Typ::DateTime);
                wrap!(self.lhs.node, typ.check_contains(&ctx.env, lhs))?;
                wrap!(self.rhs.node, typ.check_contains(&ctx.env, rhs))?;
                let ut = match (lhs.with_deref(|t| t.cloned()), rhs.with_deref(|t| t.cloned())) {
                    (None, _) | (_, None) => bail!("type must be known"),
                    (Some(lhs@ Type::Primitive(p0)), Some(rhs@ Type::Primitive(p1))) => {
                        if p0.contains(Typ::DateTime) {
                            if p1 == Typ::Duration && ($opn == Op::Add || $opn == Op::Sub) {
                                Type::Primitive(Typ::DateTime.into())
                            } else {
                                bail!("can't perform {lhs} {} {rhs}", $opn)
                            }
                        } else if p1.contains(Typ::DateTime) {
                            if p0 == Typ::Duration && $opn == Op::Add {
                                Type::Primitive(Typ::DateTime.into())
                            } else {
                                bail!("can't perform {lhs} {} {rhs}", $opn)
                            }
                        } else if p0.contains(Typ::Duration) {
                            if p1 == Typ::Duration && ($opn == Op::Add || $opn == Op::Sub) {
                                Type::Primitive(Typ::Duration.into())
                            } else if (Typ::integer() | Typ::F32 | Typ::F64).contains(p1) && ($opn == Op::Mul || $opn == Op::Div) {
                                Type::Primitive(Typ::Duration.into())
                            } else {
                                bail!("can't perform {lhs} {} {rhs}", $opn)
                            }
                        } else if p1.contains(Typ::Duration) {
                            if (Typ::integer() | Typ::F32 | Typ::F64).contains(p0) && $opn == Op::Mul {
                                Type::Primitive(Typ::Duration.into())
                            } else {
                                bail!("can't perform {lhs} {} {rhs}", $opn)
                            }
                        } else {
                            wrap!(self, lhs.union(&ctx.env, &rhs))?
                        }
                    }
                    (Some(_), Some(_)) => wrap!(self, lhs.union(&ctx.env, rhs))?
                };
                wrap!(self, self.typ.check_contains(&ctx.env, &ut))?;
                let bind = ctx.env.by_id.get(&self.id).ok_or_else(|| anyhow!("BUG: missing catch id"))?;
                wrap!(self, bind.typ.check_contains(&ctx.env, &ARITH_ERR))
            }
        }
    }
}

arith_op!(Add, Op::Add, +);
arith_op!(Sub, Op::Sub, -);
arith_op!(Mul, Op::Mul, *);
arith_op!(Div, Op::Div, /);
arith_op!(Mod, Op::Mod, %);
