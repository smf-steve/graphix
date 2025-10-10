# Closures and First Class Functions

Functions can reference variables outside their definition as long as they are
in the lexical scope of the function. These references are valid regardless of
where the function is called.

```
let f = {
  let v = cast<i64>(net::subscribe("/local/foo")$)$;
  |n| v + n
};
f(2)
```

In the above example `f` captures `v` and can use it even when it is called from
a scope where `v` isn't visible. It will output 2 + whatever value "/local/foo"
has.

Functions can be stored in data structures just like any other value.

```
type T = { count: i64, f: fn(T) -> T };
let t = { count: 0, f: |t: T| {t with count: t.count + 1} };
(t.f)(t)
```

when run will output,

```
{count: 1, f: 158}
```

Since functions are always late bound, you can have multiple values of type `T`
where each `t.f` is a different function.

```
type T = { count: i64, f: fn(T) -> T };
let t0 = { count: 0, f: |t: T| {t with count: t.count + 1} };
let t1 = { count: 0, f: |t: T| {t with count: t.count - 1} };
println((t0.f)(t0));
println((t1.f)(t1))
```

when run will output,

```
{count: 1, f: 158}
{count: -1, f: 159}
```

You can clearly see that f is bound to different functions by the runtime the
numbers 158 and 159 are the actual representations of functions at runtime.
While Graphix is not an object oriented language, you can use closures and late
binding to simulate some useful aspects of the OOP.
