# Functions

Functions are first class values. They can be stored in variables, in data
structures, and they can be passed around to other functions. Etc. They are
defined with the syntax,

```
|arg0, arg1, ...| body
```

This is often combined with a let bind to make a named function.

```
let f = |x, y| x + y + 1
```

`f` is now bound to the lambda that adds it's two arguments and 1. You can also
use structure patterns in function arguments as long as the pattern will always
match.

```
let g =|(x, y), z| x + y + z
```

Type annotations can be used to constrain the argument types and the return
type,

```
let g = |(x, y): (f64, f64), z: f64| -> f64 x + y + z
```

Functions are called with the following syntax.


```
f(1, 1)
```

Would return 3. If the function is stored in a data structure, then sometimes
you need parenthesis to call it.

```
(s.f)(1, 1)
```

Would call the function `f` in the struct `s`.
