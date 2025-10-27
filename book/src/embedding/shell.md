# Stand Alone Graphix Applications

As we saw in the overview, using the
[graphix-shell](https://docs.rs/graphix-shell/latest/graphix_shell) crate you
can build a stand alone Graphix application using rust. With those basics out of
the way, in this section we'll see,

- linking multiple Graphix files into a module tree
- integrating rust built-ins
- building a custom repl with rust built-ins added

## Module Trees, VFS resolver

In order to split a stand alone Graphix application up into mutiple files and
modules and link it all into a single rust binary you can use a kind of module
resolver called the `VFS`. This is pretty much exactly what it sounds like. Here
is an example from the TUI library that is built into the shell by default.

```rust
fn tui_mods() -> ModuleResolver {
    ModuleResolver::VFS(HashMap::from_iter([
        (Path::from("/tui"), literal!(include_str!("tui/mod.gx"))),
        (Path::from("/tui/input_handler"), literal!(include_str!("tui/input_handler.gx"))),
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
```

And then when you build the shell, you can specify this
[`ModuleResolver`](https://docs.rs/graphix-compiler/latest/graphix_compiler/expr/enum.ModuleResolver.html),
and your toplevel program can simply `use` these modules as if they were files
on disk.

```rust
ShellBuilder::<NoExt>::default()
    .module_resolvers(vec![tui_mods()])
    .mode(Mode::Static(literal!(include_str!("main.gx"))))
    .publisher(publisher)
    .subscriber(subscriber)
    .no_init(true)
    .build()?
    .run()
    .await
```

You can have as many module resolvers as you like, when loading modules they are
checked in order, so earlier ones shadow later ones.

## Custom Builtins

To use custom builtins with the shell you must provide a register function

```rust
fn register(ctx: &mut ExecCtx<NoExt>) -> Result<ArcStr> {
    BuiltIn0::register(ctx)?;
    BuiltIn1::register(ctx)?;
    Ok(literal!("builtins.gx"))
}

ShellBuilder::<NoExt>::default()
    .register(Arc::new(|ctx| register(ctx)))
    .module_resolvers(vec![tui_mods()])
    .mode(Mode::Static(literal!(include_str!("main.gx"))))
    .publisher(publisher)
    .subscriber(subscriber)
    .no_init(true)
    .build()?
    .run()
    .await
```

See [Implementing Builtins](./builtins.md) for details on actually implementing
builtins.

## Custom REPL

You can build a REPL with custom additional Graphix code, or custom builtins,
when you build your shell, just change the mode to `Mode::Repl`. Then you'll get
a custom REPL with your desired builtins and pre loaded modules already present.
