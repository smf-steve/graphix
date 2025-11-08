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

#[derive(Debug)]
struct IOCompletionQueue {
    q: VecDeque<(BindId, Option<Value>)>,
    id: BindId,
    top_id: ExprId,
}

impl IOCompletionQueue {
    fn new(id: BindId, top_id: ExprId) -> Self {
        Self { q: VecDeque::default(), id, top_id }
    }

    /// push a new outstanding item on the completion queue
    fn push_back(&mut self, id: BindId) {
        self.q.push_back((id, None));
    }

    /// Process IO completions in `event`
    ///
    /// record completion of IO items in the queue in whatever order it happens,
    /// but hold out of order items back so they can be reported in FIFO order.
    ///
    /// report ready completions via `set_var` to `id`
    fn process_event<R: Rt, E: UserEvent>(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        event: &mut Event<E>,
    ) {
        let mut saw_none = false;
        self.q.retain_mut(|(id, v)| {
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
        })
    }
}

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
    outstanding: IOCompletionQueue,
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
                outstanding: IOCompletionQueue::new(id, top_id),
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
            self.outstanding.push_back(id);
            self.reader.schedule_read(ctx, id, path)
        }
        self.outstanding.process_event(ctx, event);
        event.variables.remove(&self.id)
    }

    fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
        ctx.rt.unref_var(self.id, self.top_id);
        for (id, _) in self.outstanding.q.drain(..) {
            ctx.rt.unref_var(id, self.top_id);
        }
    }

    fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.delete(ctx);
        self.id = BindId::new();
        ctx.rt.ref_var(self.id, self.top_id);
        self.outstanding = IOCompletionQueue::new(self.id, self.top_id);
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

pub(super) trait WriteOp: Debug + Default + Send + Sync + 'static {
    const NAME: &str;
    const TYP: LazyLock<FnType>;

    fn schedule_write<R: Rt, E: UserEvent>(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        id: BindId,
        path: ArcStr,
        v: Value,
    );
}

#[derive(Debug)]
pub(super) struct WriteAllGen<T: WriteOp> {
    id: BindId,
    top_id: ExprId,
    outstanding: IOCompletionQueue,
    path: Option<ArcStr>,
    data: Option<Value>,
    writer: T,
}

impl<R: Rt, E: UserEvent, T: WriteOp> BuiltIn<R, E> for WriteAllGen<T> {
    const NAME: &str = T::NAME;
    const TYP: LazyLock<FnType> = T::TYP;

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|ctx, _, _, _, top_id| {
            let id = BindId::new();
            ctx.rt.ref_var(id, top_id);
            Ok(Box::new(WriteAllGen {
                id,
                top_id,
                outstanding: IOCompletionQueue::new(id, top_id),
                path: None,
                data: None,
                writer: T::default(),
            }))
        })
    }
}

impl<R: Rt, E: UserEvent, T: WriteOp> Apply<R, E> for WriteAllGen<T> {
    fn update(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> Option<Value> {
        let mut up = false;
        // the cast should never fail due to type checker constraints
        if let Some(Ok(path)) = from[0].update(ctx, event).map(|v| v.cast_to::<ArcStr>())
        {
            up = true;
            self.path = Some(path);
        }
        if let Some(data) = from[1].update(ctx, event) {
            up = true;
            self.data = Some(data)
        }
        if up
            && let Some(path) = &self.path
            && let Some(data) = &self.data
        {
            let id = BindId::new();
            ctx.rt.ref_var(id, self.top_id);
            self.outstanding.push_back(id);
            self.writer.schedule_write(ctx, id, path.clone(), data.clone());
        }
        self.outstanding.process_event(ctx, event);
        event.variables.remove(&self.id)
    }

    fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.path = None;
        self.data = None;
        ctx.rt.unref_var(self.id, self.top_id);
        for (id, _) in self.outstanding.q.drain(..) {
            ctx.rt.unref_var(id, self.top_id);
        }
    }

    fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.delete(ctx);
        self.id = BindId::new();
        ctx.rt.ref_var(self.id, self.top_id);
        self.outstanding = IOCompletionQueue::new(self.id, self.top_id);
    }
}

#[derive(Debug, Default)]
pub(super) struct WriteAllOp;

impl WriteOp for WriteAllOp {
    const NAME: &str = "fs_write_all";
    deftype!("fs", "fn(#path:string, string) -> Result<null, `IOError(string)>");

    fn schedule_write<R: Rt, E: UserEvent>(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        id: BindId,
        path: ArcStr,
        v: Value,
    ) {
        use tokio::fs;
        ctx.rt.spawn_var(async move {
            let res = match v {
                Value::String(s) => match fs::write(&*path, &*s).await {
                    Ok(()) => Value::Null,
                    Err(e) => errf!("IOError", "could not write {path}, {e:?}"),
                },
                v => errf!("IOError", "COMPILER BUG! Expected string not {v}"),
            };
            (id, res)
        });
    }
}

pub(super) type WriteAll = WriteAllGen<WriteAllOp>;

#[derive(Debug, Default)]
pub(super) struct WriteAllBinOp;

impl WriteOp for WriteAllBinOp {
    const NAME: &str = "fs_write_all_bin";
    deftype!("fs", "fn(#path:string, bytes) -> Result<null, `IOError(string)>");

    fn schedule_write<R: Rt, E: UserEvent>(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        id: BindId,
        path: ArcStr,
        v: Value,
    ) {
        use tokio::fs;
        ctx.rt.spawn_var(async move {
            let res = match v {
                Value::Bytes(s) => match fs::write(&*path, &*s).await {
                    Ok(()) => Value::Null,
                    Err(e) => errf!("IOError", "could not write {path}, {e:?}"),
                },
                v => errf!("IOError", "COMPILER BUG! Expected bytes not {v}"),
            };
            (id, res)
        });
    }
}

pub(super) type WriteAllBin = WriteAllGen<WriteAllBinOp>;
