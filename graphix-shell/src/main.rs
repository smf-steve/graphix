use anyhow::{Context, Result};
use clap::Parser;
use flexi_logger::{FileSpec, Logger};
use graphix_compiler::expr::ModuleResolver;
use graphix_shell::{Mode, ShellBuilder};
use log::info;
use netidx::{
    config::Config,
    publisher::{BindCfg, DesiredAuth, Publisher, PublisherBuilder},
    subscriber::{Subscriber, SubscriberBuilder},
    InternalOnly,
};
use std::{path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(version, about)]
struct Params {
    /// enable logging and put the log in the specified directory. You
    /// should also set the RUST_LOG enviornment variable. e.g. RUST_LOG=debug
    #[arg(long)]
    log_dir: Option<PathBuf>,
    /// path to the netidx config to load, otherwise the default will
    /// be loaded (unless --no-netidx is specified)
    #[arg(long)]
    config: Option<PathBuf>,
    /// the desired netidx auth mechanism to use, otherwise use the config default
    #[arg(long)]
    auth: Option<DesiredAuth>,
    /// the kerberos user principal name to use for netidx, otherwise
    /// the default from the current user's cached tickets, only valid
    /// if using kerberos auth
    #[arg(long)]
    upn: Option<String>,
    /// the netidx nerberos service princial name, otherwise the
    /// default from the current user's cached ticket, only valid if
    /// using kerberos auth
    #[arg(long)]
    spn: Option<String>,
    /// the netidx tls identity to use, otherwise use the configured
    /// default, only valid if using tls auth.
    #[arg(long)]
    identity: Option<String>,
    /// specify the netidx publisher bind address.
    #[arg(long)]
    bind: Option<BindCfg>,
    /// drop subscribers if they don't consume published values with
    /// the specifed timeout (in seconds).
    #[arg(long)]
    publish_timeout: Option<u64>,
    /// module resolution from netidx should fail if we can't
    /// subscribe to the module before the timeout expires. Default,
    /// wait forever.
    #[arg(long)]
    resolve_timeout: Option<u64>,
    /// disable netidx, net functions will only work internally
    #[arg(short, long)]
    no_netidx: bool,
    /// do not attempt to run the init module
    #[arg(short = 'i', long)]
    no_init: bool,
    /// run the program in the specified file instead of starting the REPL
    file: Option<PathBuf>,
}

impl Params {
    async fn get_pub_sub(&self) -> Result<(Publisher, Subscriber)> {
        let cfg = match &self.config {
            None => Config::load_default()?,
            Some(p) => Config::load(p)?,
        };
        let auth = match &self.auth {
            None => cfg.default_auth(),
            Some(a) => a.clone(),
        };
        let publisher = PublisherBuilder::new(cfg.clone())
            .bind_cfg(self.bind)
            .build()
            .await
            .context("creating publisher")?;
        let subscriber = SubscriberBuilder::new(cfg)
            .desired_auth(auth)
            .build()
            .context("creating subscriber")?;
        Ok((publisher, subscriber))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let p = Params::parse();
    if let Some(dir) = &p.log_dir {
        let _ = Logger::try_with_env()
            .context("initializing log")?
            .log_to_file(
                FileSpec::default()
                    .directory(dir)
                    .basename("netidx-shell")
                    .use_timestamp(false),
            )
            .start()
            .context("starting log")?;
    }
    info!("graphix shell starting");
    let mut _internal = None;
    let (publisher, subscriber) = if p.no_netidx {
        let i = InternalOnly::new().await?;
        let (p, s) = (i.publisher().clone(), i.subscriber().clone());
        _internal = Some(i);
        (p, s)
    } else {
        p.get_pub_sub().await?
    };
    let mut shell = ShellBuilder::default();
    shell.no_init(p.no_init);
    if let Some(t) = p.publish_timeout {
        shell.publish_timeout(Duration::from_secs(t));
    }
    if let Some(t) = p.resolve_timeout {
        shell.resolve_timeout(Duration::from_secs(t));
    }
    if let Some(f) = &p.file {
        shell.mode(Mode::File(f.clone()));
        match f.parent() {
            Some(p) if p.as_os_str().is_empty() => (),
            None => (),
            Some(p) => {
                shell.module_resolvers(vec![ModuleResolver::Files(p.canonicalize()?)]);
            }
        }
    }
    shell.publisher(publisher).subscriber(subscriber).build()?.run().await
}
