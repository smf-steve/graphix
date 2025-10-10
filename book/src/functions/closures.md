# Closures and First Class Functions

Functions can reference variables outside their definition as long as they are
in the lexical scope of the function. These references are valid regardless of
where the function is called.

```
let f = {
  let v = cast<i64>(net::subscribe("/foo/bar")$)$;
  |n| v + n
};
f(2)
```
