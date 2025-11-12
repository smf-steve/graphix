use crate::{deftype, CachedArgs, CachedVals, EvalCached};
use anyhow::Result;
use arcstr::{literal, ArcStr};
use compact_str::CompactString;
use graphix_compiler::{
    errf, Apply, BuiltIn, BuiltInInitFn, Event, ExecCtx, Node, Rt, UserEvent,
};
use netidx_value::Value;
use poolshark::local::LPooled;
use std::{path::PathBuf, sync::Arc};
use tempfile::TempDir;

mod file;
mod watch;

#[derive(Debug)]
struct GxTempDir {
    current: Option<TempDir>,
}

impl<R: Rt, E: UserEvent> BuiltIn<R, E> for GxTempDir {
    const NAME: &str = "fs_tempdir";
    deftype!("fs", "fn(Any) -> Result<string, `IOError(string)>");

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|_, _, _, _, _| Ok(Box::new(GxTempDir { current: None })))
    }
}

impl<R: Rt, E: UserEvent> Apply<R, E> for GxTempDir {
    fn update(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> Option<Value> {
        if from[0].update(ctx, event).is_some() {
            match TempDir::new() {
                Err(e) => Some(errf!("IOError", "failed to create temp dir {e:?}")),
                Ok(td) => {
                    use std::fmt::Write;
                    let mut buf = CompactString::new("");
                    write!(buf, "{}", td.path().display()).unwrap();
                    self.current = Some(td);
                    Some(Value::String(ArcStr::from(buf.as_str())))
                }
            }
        } else {
            None
        }
    }

    fn sleep(&mut self, _ctx: &mut ExecCtx<R, E>) {
        self.current = None
    }
}

#[derive(Debug, Default)]
struct JoinPathEv;

impl EvalCached for JoinPathEv {
    const NAME: &str = "fs_join_path";
    deftype!("fs", "fn(string, @args: [string, Array<string>]) -> string");

    fn eval(&mut self, from: &CachedVals) -> Option<Value> {
        let mut parts: LPooled<Vec<ArcStr>> = LPooled::take();
        for part in from.0.iter() {
            match part {
                None => return None,
                Some(Value::String(s)) => parts.push(s.clone()),
                Some(Value::Array(a)) => {
                    for part in a.iter() {
                        match part {
                            Value::String(s) => parts.push(s.clone()),
                            _ => return None,
                        }
                    }
                }
                _ => return None,
            }
        }
        let mut path = PathBuf::new();
        for part in parts.drain(..) {
            path.push(&*part)
        }
        let mut buf = CompactString::new("");
        use std::fmt::Write;
        write!(buf, "{}", path.display()).unwrap();
        Some(Value::String(ArcStr::from(buf.as_str())))
    }
}

type JoinPath = CachedArgs<JoinPathEv>;

pub(super) fn register<R: Rt, E: UserEvent>(ctx: &mut ExecCtx<R, E>) -> Result<ArcStr> {
    ctx.register_builtin::<GxTempDir>()?;
    ctx.register_builtin::<JoinPath>()?;
    ctx.register_builtin::<watch::SetGlobals>()?;
    ctx.register_builtin::<watch::WatchBuiltIn>()?;
    ctx.register_builtin::<watch::WatchFullBuiltIn>()?;
    ctx.register_builtin::<file::ReadAll>()?;
    ctx.register_builtin::<file::ReadAllBin>()?;
    ctx.register_builtin::<file::WriteAll>()?;
    ctx.register_builtin::<file::WriteAllBin>()?;
    Ok(literal!(include_str!("fs.gx")))
}
