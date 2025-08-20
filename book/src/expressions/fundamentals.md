# Fundamentals

Graphix has all the fundamental data types and expressions you'd expect. The
Graphix shell is a good way to explore what small Graphix expressions do. You
can run the Graphix shell by invoking `graphix` with no arguments.

# Numbers

`i32`, `u32`, `i64`, `u64`, `f32`, `f64`, and `decimal` are the fundamental
numeric types in Graphix. Numbers are written with their type prefixed, except
for `i64` and `f64` which are written bare (and are thus the default numeric
types). for example, `u32:3` is a `u32` literal value.

`decimal` is an exact decimal representation for performing financial
calculations without rounding or floating point approximation errors.

The basic arithmetic operations are implemented on all the number types with all
the other number types. The type system allows you to control the outcomes. For example,

```
〉1. + 1
-: [i64, f64]
2
```

The compiler will let you add a `f64` to an `i64` directly without casting,
however the return type of the operation will be the set of an `i64` and an
`f64`, representing that either type could be returned. If you try to pass this
result to a function that wants a specific numeric type, it will not typecheck.
On the other hand you may not care about the specific type, and may just want
the most concise code. Further,

```
〉1.2321 + 1
-: [i64, f64]
2.2321
```

In the first case we actually got an `i64:2` back from the addition, but in this
case we get an `f64:2.2321` because we can't represent the fractional part of
the `f64` in an `i64`

# Bool

Graphix has a boolean type, it's literals are written as `true` and `false`, and the name of the type is `bool`.

Boolean expressions using `&&`, `||`, and `!` are supported. These operators only operate on `bool`. They can be grouped with parenthesis. For example,

```
〉true && false
-: bool
false
〉true || false
-: bool
true
〉!true
-: bool
false
〉!1
error: in expr

Caused by:
    0: at: line: 1, column: 2, in: i64:1
    1: type mismatch bool does not contain i64
```

# Duration

A time duration. The type name is `duration`, and the literals are written as, `duration:1.0s`, `duration:1.0ms`, `duration:1.0us`, `duration:1.0ns`. Durations can be added,
