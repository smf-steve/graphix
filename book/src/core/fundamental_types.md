# Fundamental Types

Graphix has a few fundamental data types, the Graphix shell is a good way to
explore them by trying out small Graphix expressions. You can run the Graphix
shell by invoking `graphix` with no arguments.

## Numbers

`i32`, `u32`, `i64`, `u64`, `f32`, `f64`, and `decimal` are the fundamental
numeric types in Graphix. Numbers are written with their type prefixed, except
for `i64` and `f64` which are written bare (and are thus the default numeric
types). for example, `u32:3` is a `u32` literal value.

`decimal` is an exact decimal representation for performing financial
calculations without rounding or floating point approximation errors.

The basic arithmetic operations are implemented on all the number types with all
the other number types. The type system allows you to control the outcomes. For
example,

```
〉1. + 1
-: [i64, f64]
2
```

The compiler will let you do arithmatic on different types of numbers directly
without casting, however the return type of the operation will be the set of all
the types in the operation, representing that either type could be returned. If
you try to pass this result to a function that wants a specific numeric type, it
will fail at compile time.

```
〉1.2321 + 1
-: [i64, f64]
2.2321
```

In the first case we actually got an `i64:2` back from the addition, but in this
case we get an `f64:2.2321` because we can't represent the fractional part of
the `f64` in an `i64`. In general when operating on numbers of different types
you may get any type in the set back, you shouldn't rely on more precise
behavior than that.

Division by zero is raised as an error to the nearest error handler (more on
that later) and will be printed to stderr by the shell if it is never handled.
Overflow and underflow are handled by wrapping,

```
〉0 / 0

thread 'tokio-runtime-worker' panicked at /home/eric/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/mod.rs:319:5:
attempt to divide by zero
-: i64
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
unhandled error: error:"in expr at line: 1, column: 1 attempt to divide by zero"
〉u32:0 - u32:1
-: u32
4294967295
```

It is safe to continue using the shell and runtime if such an error occurrs,
even if it is not caught. However the particular arith operation that caused the
error will not update, which may cause problems depending on what your program
is doing with it.

## Bool

Graphix has a boolean type, it's literals are written as `true` and `false`, and
the name of the type is `bool`.

Boolean expressions using `&&`, `||`, and `!` are supported. These operators
only operate on `bool`. They can be grouped with parenthesis. For example,

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

## Duration

A time duration. The type name is `duration`, and the literals are written as,
`duration:1.0s`, `duration:1.0ms`, `duration:1.0us`, `duration:1.0ns`. Durations
can be added, and can be multiplied and divided by scalars.

```
〉duration:1.0s + duration:1.0s
-: duration
2.s
〉duration:1.0s * 50
-: duration
50.s
〉duration:1.0s / 50
-: duration
0.02s
```

## DateTime

A date and time in the UTC time zone. The type name is `datetime` and literals
are written in RFC3339 format inside quotes. For example,
`datetime:"2020-01-01T00:00:00Z"`. You can add and subtract `duration` from
`datetime`.

```
〉datetime:"2020-01-01T00:00:00Z" + duration:30.s
-: datetime
2020-01-01 00:00:30 UTC
```

You can enter `datetime` literals in local time and they will be converted to UTC. For example,

```
〉datetime:"2020-01-01T00:00:00-04:00"
-: datetime
2020-01-01 04:00:00 UTC
```

## String

Strings in Graphix are UTF8 encoded text. The type name is `string` and the
literal is written in quotes `"this is a string"`. C style escape sequences are
supported, `"this is \" a string with a quote and a \n"`. Non printable
characters such as newline will be escaped by default when strings are printed
to the console, you can use `print` to print the raw string.

### String Interpolation

String literals can contain expressions that will be evaluated and joined to the string,
such expressions are surrounded by unescaped `[]` in the string. For example,

```
〉let row = 1
〉let column = 999
〉"/foo/bar/[row]/[column]"
-: string
"/foo/bar/1/999"
```

Values in an interpolation need not be strings, they will be cast to a string
when they are used. You can write a literal `[` or `]` in a string by escaping
it.

```
〉"this is a string with a \[ and a \] but it isn't an interpolation"
-: string
"this is a string with a [ and a ] but it isn't an interpolation"
```
