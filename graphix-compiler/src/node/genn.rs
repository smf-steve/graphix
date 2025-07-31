use super::{callsite::CallSite, Constant, Nop, Ref, NOP};
use crate::{
    expr::{ExprId, ModPath},
    typ::{FnType, Type},
    BindId, ExecCtx, Node, Rt, UserEvent,
};
use netidx::publisher::{Typ, Value};
use std::collections::HashMap;
use triomphe::Arc;

/// generate a no op with the specific type
pub fn nop<R: Rt, E: UserEvent>(typ: Type) -> Node<R, E> {
    Nop::new(typ)
}

/// bind a variable and return a node referencing it
pub fn bind<R: Rt, E: UserEvent>(
    ctx: &mut ExecCtx<R, E>,
    scope: &ModPath,
    name: &str,
    typ: Type,
    top_id: ExprId,
) -> (BindId, Node<R, E>) {
    let id = ctx.env.bind_variable(scope, name, typ.clone()).id;
    ctx.rt.ref_var(id, top_id);
    (id, Box::new(Ref { spec: NOP.clone(), typ, id, top_id }))
}

/// generate a reference to a bind id
pub fn reference<R: Rt, E: UserEvent>(
    ctx: &mut ExecCtx<R, E>,
    id: BindId,
    typ: Type,
    top_id: ExprId,
) -> Node<R, E> {
    ctx.rt.ref_var(id, top_id);
    Box::new(Ref { spec: NOP.clone(), typ, id, top_id })
}

pub fn constant<R: Rt, E: UserEvent>(v: Value) -> Node<R, E> {
    Box::new(Constant {
        spec: NOP.clone(),
        typ: Type::Primitive(Typ::get(&v).into()),
        value: v,
    })
}

/// generate and return an apply node for the given lambda
pub fn apply<R: Rt, E: UserEvent>(
    fnode: Node<R, E>,
    args: Vec<Node<R, E>>,
    typ: Arc<FnType>,
    top_id: ExprId,
) -> Node<R, E> {
    Box::new(CallSite {
        spec: NOP.clone(),
        ftype: typ.clone(),
        args,
        arg_spec: HashMap::default(),
        fnode,
        function: None,
        top_id,
    })
}
