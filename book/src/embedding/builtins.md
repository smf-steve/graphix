# Writing Built in Functions

As mentioned in the introduction you can extend Graphix by writing
built in functions in rust. This chapter will deep dive into the full
API, if you just want to write a pure function see the
[Overview](./overview.md).

In order to implement a built-in graphix function you must implement
two traits,
[`graphix_compiler::BuiltIn`](https://docs.rs/graphix-compiler/latest/graphix_compiler/trait.BuiltIn.html)
and
[`graphix_compiler::Apply`](https://docs.rs/graphix-compiler/latest/graphix_compiler/trait.Apply.html). See
the rustdoc for details. These two traits give you more control than the
[`graphix_stdlib::CachedArgs`](https://docs.rs/graphix-stdlib/latest/graphix_stdlib/struct.CachedArgs.html) method we covered in the overview. Lets look at the simplest possible example,

## Understanding The Once Function

The `once` function evaluates it's arugment every cycle and passes
through one and only one update. The `update` method is the most
important method of `Apply`, it is called every cycle and returns
something only when the node being updated has "updated". The meaning
of that is specific to what the node does, but in the case of `once`
it means that the argument to `once` updated, and `once` has not
already seen an update. Consider the example program,

```graphix
let clock = time::timer(1, true);
println(once(clock))
```

We expect this example to print the datetime exactly one time. Lets
dig in to how that actually works. The clock created by `time::timer`
will tick once per second forever. The `time::timer` built-in will
call `set_timer` in the
[`Rt`](https://docs.rs/graphix-compiler/latest/graphix_compiler/trait.Rt.html),
which is part of the
[`ExecCtx`](https://docs.rs/graphix-compiler/latest/graphix_compiler/struct.ExecCtx.html). This
will schedule a cycle to happen 1 second from now, and will also
register that this toplevel node (`let clock = ...`) depends on the
timer event. When the timer event happens the approximate sequence of
events is,

- let clock = time::timer(1, true), update called on toplevel node (Bind)
    - time::timer(1, true), bind calls update on it's rhs
    - time::timer checks events to see if it should update, returns Some(DateTime(..))
    - bind sets the id of clock in events to Value::DateTime(..)
    - Rt checks for nodes that depend on `clock` schedules println(..)
- println(once(clock)), update called on toplevel node (CallSite)
    - once(clock), println calls update on it's argument, once(clock)
    - once::update calls update on clock
    - ref clock checks events to see if it updated, returns Some(Value::DateTime(..))
    - once::update checks if it's the first time it's argument has updated, it is
    - once::update returns Some(Value::DateTime(..))
    - println prints the datetime

## Implementing Once

```rust
#[derive(Debug)]
struct Once {
    val: bool,
}

impl<R: Rt, E: UserEvent> BuiltIn<R, E> for Once {
    const NAME: &str = "once";
    deftype!("core", "fn('a) -> 'a");

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|_, _, _, _, _| Ok(Box::new(Once { val: false })))
    }
}

impl<R: Rt, E: UserEvent> Apply<R, E> for Once {
    fn update(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> Option<Value> {
        match from {
            [s] => s.update(ctx, event).and_then(|v| {
                if self.val {
                    None
                } else {
                    self.val = true;
                    Some(v)
                }
            }),
            _ => None,
        }
    }

    fn sleep(&mut self, _ctx: &mut ExecCtx<R, E>) {
        self.val = false
    }
}
```

The BuiltIn trait is for construction, it declares the built in's
name, and it's type (which we get to write out in syntax due to the
deftype macro). It's init method is called every time the built in
needs to be registered with a new context, and returns a closure that
allocates the initial state of the function given a bunch of
information about the context it's called in. In this case we don't
care about any of that information, but it will be useful later.

The most important method of `Apply` is update. `sleep` is expected to
reset all the internal state and unregister anything registered with
the context.

## Higher Order Functions

Right, now that the easy stuff is out of the way, lets see how we can
implement a built-in that takes another Graphix function as an
argument. This gets compiler guts all over the place, sorry about
that. Again lets look at the simplest example from the standard
library, which is `array::group`.

```rust
#[derive(Debug)]
pub(super) struct Group<R: Rt, E: UserEvent> {
    queue: VecDeque<Value>,
    buf: SmallVec<[Value; 16]>,
    pred: Node<R, E>,
    ready: bool,
    pid: BindId,
    nid: BindId,
    xid: BindId,
}

impl<R: Rt, E: UserEvent> BuiltIn<R, E> for Group<R, E> {
    const NAME: &str = "array_group";
    deftype!(
        "core::array",
        "fn('a, fn(i64, 'a) -> bool throws 'e) -> Array<'a> throws 'e"
    );

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|ctx, typ, scope, from, top_id| match from {
            [_, _] => {
                let scope =
                    scope.append(&format_compact!("fn{}", LambdaId::new().inner()));
                let n_typ = Type::Primitive(Typ::I64.into());
                let etyp = typ.args[0].typ.clone();
                let mftyp = match &typ.args[1].typ {
                    Type::Fn(ft) => ft.clone(),
                    t => bail!("expected function not {t}"),
                };
                let (nid, n) =
                    genn::bind(ctx, &scope.lexical, "n", n_typ.clone(), top_id);
                let (xid, x) = genn::bind(ctx, &scope.lexical, "x", etyp.clone(), top_id);
                let pid = BindId::new();
                let fnode = genn::reference(ctx, pid, Type::Fn(mftyp.clone()), top_id);
                let pred = genn::apply(fnode, scope, vec![n, x], &mftyp, top_id);
                Ok(Box::new(Self {
                    queue: VecDeque::new(),
                    buf: smallvec![],
                    pred,
                    ready: true,
                    pid,
                    nid,
                    xid,
                }))
            }
            _ => bail!("expected two arguments"),
        })
    }
}

impl<R: Rt, E: UserEvent> Apply<R, E> for Group<R, E> {
    fn update(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> Option<Value> {
        macro_rules! set {
            ($v:expr) => {{
                self.ready = false;
                self.buf.push($v.clone());
                let len = Value::I64(self.buf.len() as i64);
                ctx.cached.insert(self.nid, len.clone());
                event.variables.insert(self.nid, len);
                ctx.cached.insert(self.xid, $v.clone());
                event.variables.insert(self.xid, $v);
            }};
        }
        if let Some(v) = from[0].update(ctx, event) {
            self.queue.push_back(v);
        }
        if let Some(v) = from[1].update(ctx, event) {
            ctx.cached.insert(self.pid, v.clone());
            event.variables.insert(self.pid, v);
        }
        if self.ready && self.queue.len() > 0 {
            let v = self.queue.pop_front().unwrap();
            set!(v);
        }
        loop {
            match self.pred.update(ctx, event) {
                None => break None,
                Some(v) => {
                    self.ready = true;
                    match v {
                        Value::Bool(true) => {
                            break Some(Value::Array(ValArray::from_iter_exact(
                                self.buf.drain(..),
                            )))
                        }
                        _ => match self.queue.pop_front() {
                            None => break None,
                            Some(v) => set!(v),
                        },
                    }
                }
            }
        }
    }

    fn typecheck(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        _from: &mut [Node<R, E>],
    ) -> anyhow::Result<()> {
        self.pred.typecheck(ctx)
    }

    fn refs(&self, refs: &mut Refs) {
        self.pred.refs(refs)
    }

    fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
        ctx.cached.remove(&self.nid);
        ctx.cached.remove(&self.pid);
        ctx.cached.remove(&self.xid);
        self.pred.delete(ctx);
    }

    fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
        self.pred.sleep(ctx);
    }
}
```

This implements `array::group`, which given an argument, stores that
argument's updates internally, and creates an array out of them when
the predicate returns true. It's type is 

```fn('a, fn(i64, 'a) -> bool) -> Array<'a>```

For example,

```graphix
let n = seq(0, 100);
array::group(n, |_, n| (n == 50) || (n == 99))
```

`seq(0, 100)` updates 100 times from 0 to 99. The `array::group` will
create two arrays, one containing `[0, .. 50]` and the other
containing `[51, .. 99]`

The implementation needs to build a `Node` representing the
predicate. `Node` is the fundamental type of everything in the graph,
ultimately the entire program compiles to a node. The kind of node we
need to create here is a function call site, that will handle all the
details of late binding optional arguments, default args, etc,
etc. The `genn` module is specifically for generating nodes.

### Typecheck

Because we generated code, we have to hook into the `typecheck`
compiler phase and make sure the type checker runs on it. This
requires that we implement the `typecheck` method. In our case all we
have to do is typecheck our generated call site. You can do powerful
things with this hook however.

### BindIds and Refs

`BindId` is a very fundamental type in compiler guts. The
[Event`](https://docs.rs/graphix-compiler/latest/graphix_compiler/struct.Event.html)
struct contains two tables indexed by it. The most important is
`variables`. Every bound variable has a `BindId`. If a variable has
updated this cycle, then it's updated value will be in the `variables`
table indexed by it's `BindId`. In order to call this predicate
function we actually create three different variables and store their
`BindIds` as `xid`, `nid`, and `pid`. `gen::ref` returns a reference
`Node` and the `BindId` of the variable it is referencing. Since those
ref nodes become the arguments to the predicate call site we create,
`xid` and `nid` allow us to control the arguments passed into the
function. We just have to set `xid` and `nid` in `Event::variables`
before we update the predicate in order to `call` the function. This
may cause it to update immediatly, or, it may depend on something else
that needs to update before it will update. Either way, once we've set
`xid` and `nid` once and called update on the predicate we've done our
duty (it may never update, and that's ok). That just leaves `pid`,
what is it for? Well, earlier it was mentioned that functions are
always late bound. This is how that works. The lambda argument we were
passed `from[1]`, whatever kind of node it is, will ultimately update
and return a `LambdaId`, which is an index into a table in the context
where all compiled functions actually reside. So every cycle we need
to call update on this node just like any other node, because the
`LambdaId` of the function we are supposed to be calling might change,
and if that happens the call site we created with `genn::apply` needs
to know about it. Luckily we don't have to handle any of the wonderful
details of late binding beyond this simple passing through of updates,
the call site will take care of that.

### ExecCtx::cached, refs, delete

What is all this `ctx.cached` stuff. Well, when call sites get
initialized for the first time, or when an branch of select wakes from
sleep, it turns out we need to know what the current value of every
variable they depend on is. Which means we need to cache globally the
current value of every variable. So if you're setting variables, 90%
chance you need to update cached.

And this also explains another function that you have to implement
when you're generating nodes, which is refs. Turns out we need to know
all the variables a node depends on, so we can, you know, set them
when it's being woken up from sleep, or stood up at a call site for
the first time.

That just leaves `delete`. Turns out the structure of the graph
changes at run time, and we need to keep everything straight. It would
be nice if we could do this with `Drop`, but that would require
holding a reference to the `ExecCtx` we are in at every `Node`. I'd
really rather not pay to wrap every access to the context in a mutex,
so we're doing it the hard way (for now).
