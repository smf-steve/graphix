use anyhow::Result;
use enumflags2::BitFlags;
use graphix_compiler::ExecCtx;
use graphix_rt::{GXConfig, GXEvent, GXHandle, GXRt, NoExt};
use netidx::pool::Pooled;
use tokio::sync::mpsc;

mod langtest;
mod libtest;

pub struct TestCtx {
    pub _internal_only: netidx::InternalOnly,
    pub rt: GXHandle<NoExt>,
}

pub async fn init(sub: mpsc::Sender<Pooled<Vec<GXEvent<NoExt>>>>) -> Result<TestCtx> {
    let _ = env_logger::try_init();
    let env = netidx::InternalOnly::new().await?;
    let mut ctx = ExecCtx::new(GXRt::<NoExt>::new(
        env.publisher().clone(),
        env.subscriber().clone(),
    ));
    let (root, mods) = crate::register(&mut ctx, BitFlags::all())?;
    Ok(TestCtx {
        _internal_only: env,
        rt: GXConfig::builder(ctx, sub)
            .root(root)
            .resolvers(vec![mods])
            .build()?
            .start()
            .await?,
    })
}

#[macro_export]
macro_rules! run {
    ($name:ident, $code:expr, $pred:expr) => {
        #[tokio::test(flavor = "current_thread")]
        async fn $name() -> ::anyhow::Result<()> {
            let (tx, mut rx) = tokio::sync::mpsc::channel(10);
            let ctx = $crate::test::init(tx).await?;
            let bs = ctx.rt;
            match bs.compile(arcstr::ArcStr::from($code)).await {
                Err(e) => assert!($pred(dbg!(Err(e)))),
                Ok(e) => {
                    dbg!("compilation succeeded");
                    let eid = e.exprs[0].id;
                    loop {
                        match rx.recv().await {
                            None => bail!("runtime died"),
                            Some(mut batch) => {
                                for e in batch.drain(..) {
                                    match e {
                                        graphix_rt::GXEvent::Env(_) => (),
                                        graphix_rt::GXEvent::Updated(id, v) => {
                                            assert_eq!(id, eid);
                                            eprintln!("{v}");
                                            assert!($pred(Ok(&v)));
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        }
    };
}
