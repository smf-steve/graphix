use anyhow::Result;
use arcstr::{literal, ArcStr};
use graphix_compiler::{ExecCtx, Rt, UserEvent};

mod watch;

pub(super) fn register<R: Rt, E: UserEvent>(ctx: &mut ExecCtx<R, E>) -> Result<ArcStr> {
    ctx.register_builtin::<watch::WatchBuiltIn>()?;
    Ok(literal!(include_str!("fs.gx")))
}
