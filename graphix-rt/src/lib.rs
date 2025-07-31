//! A general purpose graphix runtime
//!
//! This module implements a generic graphix runtime suitable for most
//! applications, including applications that implement custom graphix
//! builtins. The graphix interperter is run in a background task, and
//! can be interacted with via a handle. All features of the standard
//! library are supported by this runtime.
use anyhow::{anyhow, bail, Result};
use arcstr::ArcStr;
use core::fmt;
use derive_builder::Builder;
use graphix_compiler::{
    env::Env,
    expr::{ExprId, ModuleResolver},
    typ::{FnType, Type},
    BindId, ExecCtx, NoUserEvent,
};
use log::error;
use netidx::{
    pool::Pooled,
    protocol::valarray::ValArray,
    publisher::{Value, WriteRequest},
    subscriber::{self, SubId},
};
use netidx_core::atomic_id;
use serde_derive::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{path::PathBuf, time::Duration};
use tokio::{
    sync::{
        mpsc::{self as tmpsc},
        oneshot,
    },
    task,
};

mod gx;
mod rt;
use gx::GX;
pub use rt::GXRt;

type UpdateBatch = Pooled<Vec<(SubId, subscriber::Event)>>;
type WriteBatch = Pooled<Vec<WriteRequest>>;

#[derive(Debug)]
pub struct CouldNotResolve;

impl fmt::Display for CouldNotResolve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not resolve module")
    }
}

pub struct CompExp {
    pub id: ExprId,
    pub typ: Type,
    pub output: bool,
    rt: GXHandle,
}

impl Drop for CompExp {
    fn drop(&mut self) {
        let _ = self.rt.0.send(ToGX::Delete { id: self.id });
    }
}

pub struct CompRes {
    pub exprs: SmallVec<[CompExp; 1]>,
    pub env: Env<GXRt, NoUserEvent>,
}

pub struct Ref {
    pub id: ExprId,
    pub last: Option<Value>,
    pub bid: BindId,
    pub target_bid: Option<BindId>,
    rt: GXHandle,
}

impl Drop for Ref {
    fn drop(&mut self) {
        let _ = self.rt.0.send(ToGX::Delete { id: self.id });
    }
}

impl Ref {
    pub fn set(&self, v: Value) -> Result<()> {
        self.rt.set(self.bid, v)
    }

    pub fn set_deref<T: Into<Value>>(&self, v: T) -> Result<()> {
        if let Some(id) = self.target_bid {
            self.rt.set(id, v)?
        }
        Ok(())
    }
}

atomic_id!(CallableId);

pub struct Callable {
    rt: GXHandle,
    id: CallableId,
    env: Env<GXRt, NoUserEvent>,
    pub typ: FnType,
    pub expr: ExprId,
}

impl Drop for Callable {
    fn drop(&mut self) {
        let _ = self.rt.0.send(ToGX::DeleteCallable { id: self.id });
    }
}

impl Callable {
    /// Call the lambda with args. Argument types and arity will be
    /// checked and an error will be returned if they are wrong.
    pub async fn call(&self, args: ValArray) -> Result<()> {
        if self.typ.args.len() != args.len() {
            bail!("expected {} args", self.typ.args.len())
        }
        for (i, (a, v)) in self.typ.args.iter().zip(args.iter()).enumerate() {
            if !a.typ.is_a(&self.env, v) {
                bail!("type mismatch arg {i} expected {}", a.typ)
            }
        }
        self.call_unchecked(args).await
    }

    /// Call the lambda with args. Argument types and arity will NOT
    /// be checked. This can result in a runtime panic, invalid
    /// results, and probably other bad things.
    pub async fn call_unchecked(&self, args: ValArray) -> Result<()> {
        self.rt
            .0
            .send(ToGX::Call { id: self.id, args })
            .map_err(|_| anyhow!("runtime is dead"))
    }
}

enum ToGX {
    GetEnv { res: oneshot::Sender<Env<GXRt, NoUserEvent>> },
    Delete { id: ExprId },
    Load { path: PathBuf, rt: GXHandle, res: oneshot::Sender<Result<CompRes>> },
    Compile { text: ArcStr, rt: GXHandle, res: oneshot::Sender<Result<CompRes>> },
    CompileCallable { id: Value, rt: GXHandle, res: oneshot::Sender<Result<Callable>> },
    CompileRef { id: BindId, rt: GXHandle, res: oneshot::Sender<Result<Ref>> },
    Set { id: BindId, v: Value },
    Call { id: CallableId, args: ValArray },
    DeleteCallable { id: CallableId },
}

#[derive(Clone)]
pub enum GXEvent {
    Updated(ExprId, Value),
    Env(Env<GXRt, NoUserEvent>),
}

/// A handle to a running GX instance.
///
/// Drop the handle to shutdown the associated background tasks.
#[derive(Clone)]
pub struct GXHandle(tmpsc::UnboundedSender<ToGX>);

impl GXHandle {
    async fn exec<R, F: FnOnce(oneshot::Sender<R>) -> ToGX>(&self, f: F) -> Result<R> {
        let (tx, rx) = oneshot::channel();
        self.0.send(f(tx)).map_err(|_| anyhow!("runtime is dead"))?;
        Ok(rx.await.map_err(|_| anyhow!("runtime did not respond"))?)
    }

    /// Get a copy of the current graphix environment
    pub async fn get_env(&self) -> Result<Env<GXRt, NoUserEvent>> {
        self.exec(|res| ToGX::GetEnv { res }).await
    }

    /// Compile and execute the specified graphix expression.
    ///
    /// If it generates results, they will be sent to all the channels that are
    /// subscribed. When the `CompExp` objects contained in the `CompRes` are
    /// dropped their corresponding expressions will be deleted. Therefore, you
    /// can stop execution of the whole expression by dropping the returned
    /// `CompRes`.
    pub async fn compile(&self, text: ArcStr) -> Result<CompRes> {
        Ok(self.exec(|tx| ToGX::Compile { text, res: tx, rt: self.clone() }).await??)
    }

    /// Load and execute the specified graphix module.
    ///
    /// The path may have one of two forms. If it is the path to a file with
    /// extension .bs then the rt will load the file directly. If it is a
    /// modpath (e.g. foo::bar::baz) then the module resolver will look for a
    /// matching module in the modpath. When the `CompExp` objects contained in
    /// the `CompRes` are dropped their corresponding expressions will be
    /// deleted. Therefore, you can stop execution of the whole file by dropping
    /// the returned `CompRes`.
    pub async fn load(&self, path: PathBuf) -> Result<CompRes> {
        Ok(self.exec(|tx| ToGX::Load { path, res: tx, rt: self.clone() }).await??)
    }

    /// Compile a callable interface to the specified lambda id.
    ///
    /// This is how you call a lambda directly from rust. When the returned
    /// `Callable` is dropped the associated callsite will be delete.
    pub async fn compile_callable(&self, id: Value) -> Result<Callable> {
        Ok(self
            .exec(|tx| ToGX::CompileCallable { id, rt: self.clone(), res: tx })
            .await??)
    }

    /// Compile an expression that will output the value of the ref specifed by
    /// id.
    ///
    /// This is the same as the deref (*) operator in graphix. When the returned
    /// `Ref` is dropped the compiled code will be deleted.
    pub async fn compile_ref(&self, id: impl Into<BindId>) -> Result<Ref> {
        Ok(self
            .exec(|tx| ToGX::CompileRef { id: id.into(), res: tx, rt: self.clone() })
            .await??)
    }

    /// Set the variable idenfified by `id` to `v`
    ///
    /// triggering updates of all dependent node trees.
    pub fn set<T: Into<Value>>(&self, id: BindId, v: T) -> Result<()> {
        let v = v.into();
        self.0.send(ToGX::Set { id, v }).map_err(|_| anyhow!("runtime is dead"))
    }
}

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct GXConfig {
    /// The subscribe timeout to use when resolving modules in
    /// netidx. Resolution will fail if the subscription does not
    /// succeed before this timeout elapses.
    #[builder(setter(strip_option), default)]
    resolve_timeout: Option<Duration>,
    /// The publish timeout to use when sending published batches. Default None.
    #[builder(setter(strip_option), default)]
    publish_timeout: Option<Duration>,
    /// The execution context with any builtins already registered
    ctx: ExecCtx<GXRt, NoUserEvent>,
    /// The text of the root module
    #[builder(setter(strip_option), default)]
    root: Option<ArcStr>,
    /// The set of module resolvers to use when resolving loaded modules
    #[builder(default)]
    resolvers: Vec<ModuleResolver>,
    /// The channel that will receive events from the runtime
    sub: tmpsc::Sender<Pooled<Vec<GXEvent>>>,
}

impl GXConfig {
    /// Create a new config
    pub fn builder(
        ctx: ExecCtx<GXRt, NoUserEvent>,
        sub: tmpsc::Sender<Pooled<Vec<GXEvent>>>,
    ) -> GXConfigBuilder {
        GXConfigBuilder::default().ctx(ctx).sub(sub)
    }

    /// Start the graphix runtime with the specified config,
    ///
    /// return a handle capable of interacting with it. root is the text of the
    /// root module you wish to initially load. This will define the environment
    /// for the rest of the code compiled by this runtime. The runtime starts
    /// completely empty, with only the language, no core library, no standard
    /// library. To build a runtime with the full standard library and nothing
    /// else simply pass the output of `graphix_stdlib::register` to start.
    pub async fn start(self) -> Result<GXHandle> {
        let (init_tx, init_rx) = oneshot::channel();
        let (tx, rx) = tmpsc::unbounded_channel();
        task::spawn(async move {
            match GX::new(self).await {
                Ok(bs) => {
                    let _ = init_tx.send(Ok(()));
                    if let Err(e) = bs.run(rx).await {
                        error!("run loop exited with error {e:?}")
                    }
                }
                Err(e) => {
                    let _ = init_tx.send(Err(e));
                }
            };
        });
        init_rx.await??;
        Ok(GXHandle(tx))
    }
}
