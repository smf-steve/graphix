use crate::deftype;
use arcstr::ArcStr;
use bytes::Bytes;
use graphix_compiler::{
    errf, expr::ExprId, typ::FnType, Apply, BindId, BuiltIn, BuiltInInitFn, Event,
    ExecCtx, Node, Rt, UserEvent,
};
use netidx_value::Value;
use std::{
    collections::VecDeque,
    fmt::Debug,
    sync::{Arc, LazyLock},
};

pub(super) trait ReadOp: Debug + Default + Send + Sync + 'static {
    const NAME: &str;
    const TYP: LazyLock<FnType>;

    fn schedule_read<R: Rt, E: UserEvent>(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        id: BindId,
        path: ArcStr,
    );
}

#[derive(Debug)]
pub(super) struct ReadAllGen<T: ReadOp> {
    id: BindId,
    top_id: ExprId,
    outstanding: VecDeque<(BindId, Option<Value>)>,
    reader: T,
}

impl<R: Rt, E: UserEvent, T: ReadOp> BuiltIn<R, E> for ReadAllGen<T> {
    const NAME: &str = T::NAME;
    const TYP: LazyLock<FnType> = T::TYP;

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|ctx, _, _, _, top_id| {
            let id = BindId::new();
            ctx.rt.ref_var(id, top_id);
            Ok(Box::new(ReadAllGen {
                id,
                top_id,
                outstanding: VecDeque::new(),
                reader: T::default(),
            }))
        })
    }
}

impl<R: Rt, E: UserEvent, T: ReadOp> Apply<R, E> for ReadAllGen<T> {
    fn update(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> Option<Value> {
        // the cast should never fail due to type checker constraints
        if let Some(Ok(path)) = from[0].update(ctx, event).map(|v| v.cast_to::<ArcStr>())
        {
            let id = BindId::new();
            ctx.rt.ref_var(id, self.top_id);
            self.outstanding.push_back((id, None));
            self.reader.schedule_read(ctx, id, path)
        }
        // We stop extracting as soon as we hit the first incomplete read to
        // preserve FIFO ordering. All reads after that point remain queued even
        // if they've completed.
        let mut saw_none = false;
        self.outstanding.retain_mut(|(id, v)| {
            if v.is_none()
                && let Some(res) = event.variables.remove(id)
            {
                ctx.rt.unref_var(*id, self.top_id);
                *v = Some(res);
            }
            saw_none
                || match v.take() {
                    None => {
                        saw_none = true;
                        true
                    }
                    Some(v) => {
                        ctx.rt.set_var(self.id, v);
                        false
                    }
                }
        });
        event.variables.remove(&self.id)
    }

    fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
        ctx.rt.unref_var(self.id, self.top_id);
        for (id, _) in self.outstanding.drain(..) {
            ctx.rt.unref_var(id, self.top_id);
        }
    }

    fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.delete(ctx);
        self.id = BindId::new();
        ctx.rt.ref_var(self.id, self.top_id);
    }
}

#[derive(Debug, Default)]
pub(super) struct ReadAllOp;

impl ReadOp for ReadAllOp {
    const NAME: &str = "fs_read_all";
    deftype!("fs", "fn(string) -> Result<string, `IOError(string)>");

    fn schedule_read<R: Rt, E: UserEvent>(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        id: BindId,
        path: ArcStr,
    ) {
        ctx.rt.spawn_var(async move {
            match tokio::fs::read_to_string(&*path).await {
                Ok(s) => (id, Value::from(s)),
                Err(e) => (id, errf!("IOError", "could not read {path}, {e:?}")),
            }
        });
    }
}

pub(super) type ReadAll = ReadAllGen<ReadAllOp>;

#[derive(Debug, Default)]
pub(super) struct ReadAllBinOp;

impl ReadOp for ReadAllBinOp {
    const NAME: &str = "fs_read_all_bin";
    deftype!("fs", "fn(string) -> Result<bytes, `IOError(string)>");

    fn schedule_read<R: Rt, E: UserEvent>(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        id: BindId,
        path: ArcStr,
    ) {
        ctx.rt.spawn_var(async move {
            match tokio::fs::read(&*path).await {
                Ok(s) => (id, Value::from(Bytes::from(s))),
                Err(e) => (id, errf!("IOError", "could not read {path}, {e:?}")),
            }
        });
    }
}

pub(super) type ReadAllBin = ReadAllGen<ReadAllBinOp>;
