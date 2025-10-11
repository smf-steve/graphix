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
