# Embedded Scripting Systems

You can use Graphix as an embedded scripting engine in your application. For
this application we leave the shell behind and move to the
[`graphix-rt`](https://docs.rs/graphix-rt/latest/graphix_rt) crate. This is a
lower level crate that gives you a lot more control over the compiler and run
time.

## Tokio

`graphix-rt` uses tokio internally, it will run the compiler and run loop in
it's own tokio task. You Interface with this task via a
[`GXHandle`](https://docs.rs/graphix-rt/latest/graphix_rt/struct.GXHandle.html).
As a result, all operations on Graphix objects are async, and the compiler/run
loop will run in parallel with your application.

## Setting it up

The shell itself is actually the best example of using the `graphix-rt` crate. As we get further into internals, the details can change more often, but in general getting a Graphix runtime going involves the following,

```rust
// set up an execution context using the generic runtime with no customization
let mut ctx = ExecCtx::new(GXRt::<NoExt>::new(publisher, subscriber));
// ... use the context to register all your built-ins, etc

// set up a channel to receive events from the RT
let (tx, rx) = mpsc::channel(100);
// build the config and start the runtime
let handle = GXConfig::builder(ctx, tx)
    // set up a root module. Note if you want the standard library you must load
    // it as part of the root module. Otherwise you will get the bare compiler.
    .root(literal!("root.gx"))
    .build()?
    .start().await?
```

Once that all succeeds you have a running compiler/run loop, and a handle that
can interact with it. You are expected to read the `rx` portion of the mpsc
channel. If you do not, the run loop will block waiting for you to read once the
mpsc channel fills up.

## Getting Some Results

Once setup is complete, lets compile some code and get some results! To compile
code we call `compile` on the handle. This results in one or more toplevel
expressions and a copy of the environment (or a compile error).

```rust
let cres = handle.compile(literal!("2 + 2")).await?;

// in this case we know there is just one top level expression
let e = cres.exprs[0];

// the actual result will come to us on the channel. If the expression kept
// producing results, we'd keep seeing updates for it's id on the channel.
// This is a batch of GXEvents
let mut batch = rx.recv().await.ok_or_else(|| anyhow!("the runtime is dead"))?;
for ev in batch.drain(..) {
    match ev {
        GXEvent::Updated(id, v) if id == e.id => println!("2 + 2 = {v}"),
        GXEvent::Env(_) => () // new environment, but we don't care
    }
}
```
