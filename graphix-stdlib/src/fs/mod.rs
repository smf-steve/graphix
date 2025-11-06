use anyhow::Result;
use arcstr::{literal, ArcStr};
use graphix_compiler::{ExecCtx, Rt, UserEvent};

mod file;
mod watch;

pub(super) fn register<R: Rt, E: UserEvent>(ctx: &mut ExecCtx<R, E>) -> Result<ArcStr> {
    ctx.register_builtin::<watch::WatchBuiltIn>()?;
    ctx.register_builtin::<watch::WatchFullBuiltIn>()?;
    ctx.register_builtin::<file::ReadAll>()?;
    ctx.register_builtin::<file::ReadAllBin>()?;
    Ok(literal!(include_str!("fs.gx")))
}
