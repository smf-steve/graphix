use anyhow::{bail, Context, Result};
use arcstr::{literal, ArcStr};
use derive_builder::Builder;
use enumflags2::BitFlags;
use graphix_compiler::{
    expr::{CouldNotResolve, ExprId, ModPath, ModuleResolver},
    format_with_flags,
    typ::{TVal, Type},
    CFlag, ExecCtx, PrintFlag,
};
use graphix_rt::{CompExp, GXConfig, GXEvent, GXExt, GXHandle, GXRt};
use graphix_stdlib::Module;
use input::InputReader;
use netidx::{
    path::Path,
    publisher::{Publisher, Value},
    subscriber::Subscriber,
};
use poolshark::global::GPooled;
use reedline::Signal;
use std::{collections::HashMap, process::exit, sync::LazyLock, time::Duration};
use tokio::{select, sync::mpsc};
use triomphe::Arc;
use tui::Tui;

mod completion;
mod input;
mod tui;

type Env<X> = graphix_compiler::env::Env<GXRt<X>, <X as GXExt>::UserEvent>;

const TUITYP: LazyLock<Type> = LazyLock::new(|| Type::Ref {
    scope: ModPath::root(),
    name: ModPath::from(["tui", "Tui"]),
    params: Arc::from_iter([]),
});

enum Output<X: GXExt> {
    None,
    Tui(Tui<X>),
    Text(CompExp<X>),
}

impl<X: GXExt> Output<X> {
    fn from_expr(gx: &GXHandle<X>, env: &Env<X>, e: CompExp<X>) -> Self {
        if let Some(typ) = e.typ.with_deref(|t| t.cloned())
            && typ != Type::Bottom
            && typ != Type::Any
            && TUITYP.contains(env, &typ).unwrap()
        {
            Self::Tui(Tui::start(gx, env.clone(), e))
        } else {
            Self::Text(e)
        }
    }

    async fn clear(&mut self) {
        match self {
            Self::None | Self::Text(_) => (),
            Self::Tui(tui) => tui.stop().await,
        }
        *self = Self::None
    }

    async fn process_update(&mut self, env: &Env<X>, id: ExprId, v: Value) {
        match self {
            Self::None => (),
            Self::Tui(tui) => tui.update(id, v).await,
            Self::Text(e) => {
                if e.id == id {
                    println!("{}", TVal { env: &env, typ: &e.typ, v: &v })
                }
            }
        }
    }
}

fn tui_mods() -> ModuleResolver {
    ModuleResolver::VFS(HashMap::from_iter([
        (Path::from("/tui"), literal!(include_str!("tui/mod.gx"))),
        (
            Path::from("/tui/input_handler"),
            literal!(include_str!("tui/input_handler.gx")),
        ),
        (Path::from("/tui/text"), literal!(include_str!("tui/text.gx"))),
        (Path::from("/tui/paragraph"), literal!(include_str!("tui/paragraph.gx"))),
        (Path::from("/tui/block"), literal!(include_str!("tui/block.gx"))),
        (Path::from("/tui/scrollbar"), literal!(include_str!("tui/scrollbar.gx"))),
        (Path::from("/tui/layout"), literal!(include_str!("tui/layout.gx"))),
        (Path::from("/tui/tabs"), literal!(include_str!("tui/tabs.gx"))),
        (Path::from("/tui/barchart"), literal!(include_str!("tui/barchart.gx"))),
        (Path::from("/tui/chart"), literal!(include_str!("tui/chart.gx"))),
        (Path::from("/tui/sparkline"), literal!(include_str!("tui/sparkline.gx"))),
        (Path::from("/tui/line_gauge"), literal!(include_str!("tui/line_gauge.gx"))),
        (Path::from("/tui/gauge"), literal!(include_str!("tui/gauge.gx"))),
        (Path::from("/tui/list"), literal!(include_str!("tui/list.gx"))),
        (Path::from("/tui/table"), literal!(include_str!("tui/table.gx"))),
        (Path::from("/tui/calendar"), literal!(include_str!("tui/calendar.gx"))),
        (Path::from("/tui/canvas"), literal!(include_str!("tui/canvas.gx"))),
        (Path::from("/tui/browser"), literal!(include_str!("tui/browser.gx"))),
    ]))
}

#[derive(Debug, Clone)]
pub enum Mode {
    /// Read input line by line from the user and compile/execute it.
    /// provide completion and print the value of the last expression
    /// as it executes. Ctrl-C cancel's execution of the last
    /// expression and Ctrl-D exits the shell.
    Repl,
    /// Load compile and execute the specified file. Print the value
    /// of the last expression in the file to stdout. Ctrl-C exits the
    /// shell.
    File(ArcStr),
    /// Compile and execute the code in the specified string. Besides
    /// not loading from a file this mode behaves exactly like File.
    Static(ArcStr),
    /// Check that the specified file compiles but do not run it
    Check(ArcStr),
}

impl Mode {
    fn file_mode(&self) -> bool {
        match self {
            Self::Repl => false,
            Self::File(_) | Self::Check(_) | Self::Static(_) => true,
        }
    }
}

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct Shell<X: GXExt> {
    /// do not run the users init module
    #[builder(default = "false")]
    no_init: bool,
    /// drop subscribers if they don't consume updates after this timeout
    #[builder(setter(strip_option), default)]
    publish_timeout: Option<Duration>,
    /// module resolution from netidx will fail if it can't subscribe
    /// before this time elapses
    #[builder(setter(strip_option), default)]
    resolve_timeout: Option<Duration>,
    /// define module resolvers to append to the default list
    #[builder(default)]
    module_resolvers: Vec<ModuleResolver>,
    /// enable or disable features of the standard library
    #[builder(default = "BitFlags::all()")]
    stdlib_modules: BitFlags<Module>,
    /// set the shell's mode
    #[builder(default = "Mode::Repl")]
    mode: Mode,
    /// The netidx publisher to use. If you do not wish to use netidx
    /// you can use netidx::InternalOnly to create an internal netidx
    /// environment
    publisher: Publisher,
    /// The netidx subscriber to use. If you do not wish to use netidx
    /// you can use netidx::InternalOnly to create an internal netidx
    /// environment
    subscriber: Subscriber,
    /// Provide a closure to register any built-ins you wish to use.
    ///
    /// Your closure should register the builtins with the context and return a
    /// string specifiying any modules you need to load in order to use them.
    /// For example if you wish to implement a module called m containing
    /// builtins foo and bar, then you would first implement foo and bar in rust
    /// and register them with the context. You would add a VFS module resolver
    /// to the set of resolvers containing prototypes that reference your rust
    /// builtins. e.g.
    ///
    /// ``` ignore
    /// pub let foo = |x, y| 'foo_builtin;
    /// pub let bar = |x| 'bar_builtin
    /// ```
    ///
    /// Your VFS resolver would map "/m" -> the above stubs. Your register
    /// function would then return "mod m\n" to force loading the module at
    /// startup. Then your user only needs to `use m`
    #[builder(setter(strip_option), default)]
    register: Option<Arc<dyn Fn(&mut ExecCtx<GXRt<X>, X::UserEvent>) -> Result<ArcStr>>>,
    /// Enable compiler flags, these will be ORed with the default set of flags
    /// for the mode.
    #[builder(default)]
    enable_flags: BitFlags<CFlag>,
    /// Disable compiler flags, these will be subtracted from the final set.
    /// (default_flags | enable_flags) - disable_flags
    #[builder(default)]
    disable_flags: BitFlags<CFlag>,
}

impl<X: GXExt> Shell<X> {
    async fn init(
        &mut self,
        sub: mpsc::Sender<GPooled<Vec<GXEvent<X>>>>,
    ) -> Result<GXHandle<X>> {
        let publisher = self.publisher.clone();
        let subscriber = self.subscriber.clone();
        let mut ctx = ExecCtx::new(GXRt::<X>::new(publisher, subscriber));
        let (root, mods) = graphix_stdlib::register(&mut ctx, self.stdlib_modules)
            .context("register stdlib modules")?;
        let usermods = self
            .register
            .as_mut()
            .map(|f| f(&mut ctx))
            .transpose()
            .context("register user modules")?;
        let root = match usermods {
            Some(m) => ArcStr::from(format!("{root};\nmod tui;\n{m}")),
            None => ArcStr::from(format!("{root};\nmod tui")),
        };
        let mut flags = match self.mode {
            Mode::File(_) | Mode::Check(_) | Mode::Static(_) => {
                CFlag::WarnUnhandled | CFlag::WarnUnused
            }
            Mode::Repl => BitFlags::empty(),
        };
        flags.insert(self.enable_flags);
        flags.remove(self.disable_flags);
        let mut mods = vec![mods, tui_mods()];
        for res in self.module_resolvers.drain(..) {
            mods.push(res);
        }
        let mut gx = GXConfig::builder(ctx, sub);
        gx = gx.flags(flags);
        if let Some(s) = self.publish_timeout {
            gx = gx.publish_timeout(s);
        }
        if let Some(s) = self.resolve_timeout {
            gx = gx.resolve_timeout(s);
        }
        Ok(gx
            .root(root)
            .resolvers(mods)
            .build()
            .context("building rt config")?
            .start()
            .await
            .context("loading initial modules")?)
    }

    async fn load_env(
        &mut self,
        gx: &GXHandle<X>,
        newenv: &mut Option<Env<X>>,
        output: &mut Output<X>,
        exprs: &mut Vec<CompExp<X>>,
    ) -> Result<Env<X>> {
        let env;
        macro_rules! file_mode {
            ($r:expr) => {{
                exprs.extend($r.exprs);
                env = gx.get_env().await?;
                if let Some(e) = exprs.pop() {
                    *output = Output::from_expr(&gx, &env, e);
                }
                *newenv = None
            }};
        }
        match &self.mode {
            Mode::File(file) => {
                let r = gx.load(file.clone()).await?;
                file_mode!(r)
            }
            Mode::Check(file) => {
                gx.check(file.clone()).await?;
                exit(0)
            }
            Mode::Static(s) => {
                let r = gx.compile(s.clone()).await?;
                file_mode!(r)
            }
            Mode::Repl if !self.no_init => match gx.compile("mod init".into()).await {
                Ok(res) => {
                    env = res.env;
                    exprs.extend(res.exprs);
                    *newenv = Some(env.clone())
                }
                Err(e) if e.is::<CouldNotResolve>() => {
                    env = gx.get_env().await?;
                    *newenv = Some(env.clone())
                }
                Err(e) => {
                    eprintln!("error in init module: {e:?}");
                    env = gx.get_env().await?;
                    *newenv = Some(env.clone())
                }
            },
            Mode::Repl => {
                env = gx.get_env().await?;
                *newenv = Some(env.clone());
            }
        }
        Ok(env)
    }

    pub async fn run(mut self) -> Result<()> {
        let (tx, mut from_gx) = mpsc::channel(100);
        let gx = self.init(tx).await?;
        let script = self.mode.file_mode();
        let mut input = InputReader::new();
        let mut output = Output::None;
        let mut newenv = None;
        let mut exprs = vec![];
        let mut env = self.load_env(&gx, &mut newenv, &mut output, &mut exprs).await?;
        if !script {
            println!("Welcome to the graphix shell");
            println!("Press ctrl-c to cancel, ctrl-d to exit, and tab for help")
        }
        loop {
            select! {
                batch = from_gx.recv() => match batch {
                    None => bail!("graphix runtime is dead"),
                    Some(mut batch) => {
                        for e in batch.drain(..) {
                            match e {
                                GXEvent::Updated(id, v) => {
                                    output.process_update(&env, id, v).await
                                },
                                GXEvent::Env(e) => {
                                    env = e;
                                    newenv = Some(env.clone());
                                }
                            }
                        }
                    }
                },
                input = input.read_line(&mut output, &mut newenv) => {
                    match input {
                        Err(e) => eprintln!("error reading line {e:?}"),
                        Ok(Signal::CtrlC) if script => break Ok(()),
                        Ok(Signal::CtrlC) => output.clear().await,
                        Ok(Signal::CtrlD) => break Ok(()),
                        Ok(Signal::Success(line)) => {
                            match gx.compile(ArcStr::from(line)).await {
                                Err(e) => eprintln!("error: {e:?}"),
                                Ok(res) => {
                                    env = res.env;
                                    newenv = Some(env.clone());
                                    exprs.extend(res.exprs);
                                    if exprs.last().map(|e| e.output).unwrap_or(false) {
                                        let e = exprs.pop().unwrap();
                                        let typ = e.typ
                                            .with_deref(|t| t.cloned())
                                            .unwrap_or_else(|| e.typ.clone());
                                        format_with_flags(
                                            PrintFlag::DerefTVars | PrintFlag::ReplacePrims,
                                            || println!("-: {}", typ)
                                        );
                                        output = Output::from_expr(&gx, &env, e);
                                    } else {
                                        output.clear().await
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}
