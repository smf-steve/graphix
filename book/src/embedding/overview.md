# Embedding and Extending Graphix

There are multiple ways you can embed Graphix in your application and
extend it with rust code.

## Writing built-in functions in rust

You can implement Graphix functions in rust. Most of the standard
library is actually written in rust (to improve startup time), and you
can easily add more built-ins using rust code for computationally
heavy tasks, or IO.

There are two different ways to write built-ins, for a simple pure
function you can use a the `CachedArgs` interface which takes care of
most of the details for you. You only need to implement one method to
evaluate changes to your arguments. For example min finds the minimum
value of all it's arguments,

```rust
use graphix_stdlib::{deftype, CachedArgs, EvalCached};
use anyhow::{bail, Result};
use arcstr::{literal, ArcStr};
use netidx::subscriber::Value;

#[derive(Debug, Default)]
struct MinEv;

impl EvalCached for MinEv {
    const NAME: &str = "min";
    deftype!("core", "fn('a, @args:'a) -> 'a");

    fn eval(&mut self, from: &CachedVals) -> Option<Value> {
        let mut res = None;
        for v in from.flat_iter() {
            match (res, v) {
                (None, None) | (Some(_), None) => return None,
                (None, Some(v)) => {
                    res = Some(v);
                }
                (Some(v0), Some(v)) => {
                    res = if v < v0 { Some(v) } else { Some(v0) };
                }
            }
        }
        res
    }
}

type Min = CachedArgs<MinEv>;
```

Then you must register this built-in with the exec context,

```rust
ctx.register_builtin::<Min>()?
```

And then you can call it from Graphix,

```graphix
let min = |@args| 'min
```

The special form function body `'min` references a built-in rust
function.
  
See [Writing Built in Functions](./builtins.md) for details.

## Building Stand Alone Graphix Applications

You can build single binary stand alone Graphix applications using the
`graphix-shell` crate. All your Graphix source code, and built-ins are
compiled together with the compiler and runtime into a single binary
that you can then deploy and run. When combined with writing built-ins
in rust this becomes a powerful mixed language toolset.

For example here is the netidx browser from `netidx-tools`:

```rust
use crate::publisher;
use anyhow::{Context, Result};
use arcstr::literal;
use graphix_rt::NoExt;
use graphix_shell::{Mode, ShellBuilder};
use netidx::{
    config::Config,
    publisher::{DesiredAuth, PublisherBuilder},
    subscriber::Subscriber,
};

pub async fn run(
    cfg: Config,
    auth: DesiredAuth,
    params: publisher::Params,
) -> Result<()> {
    let publisher = PublisherBuilder::new(cfg.clone())
        .desired_auth(auth.clone())
        .bind_cfg(params.bind)
        .build()
        .await
        .context("creating publisher")?;
    let subscriber = Subscriber::new(cfg, auth).context("create subscriber")?;
    ShellBuilder::<NoExt>::default()
        .mode(Mode::Static(literal!(include_str!("browser.gx"))))
        .publisher(publisher)
        .subscriber(subscriber)
        .no_init(true)
        .build()?
        .run()
        .await
}
```

See [Stand Alone Graphix Applications](./shell.md) for details

## Embedding Graphix in Your Application

Using the `graphix-rt` crate you can embed the Graphix compiler and
runtime in your application. Then you can,

- compile and run Graphix code
- receive events from Graphix expressions
- inject events into Graphix pipelines
- call Graphix functions

The runtime uses tokio and runs in a background task so it integrates
well into a normal async workflow.

See [Using Graphix as Embedded Scripting](./rt.md) for details
