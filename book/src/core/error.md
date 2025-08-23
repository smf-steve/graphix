# Error Handling

Errors in Graphix are represented by the built in error type, which carries a
string describing the error. The postfix operator `?` connects error values to
the nearest `errors` binding that is in scope, otherwise passes them through.
There is an `errors` binding at the top level of the core library which serves
as the default place errors go. The shell will print errors that make it to the
top level to stderr.

Array indexing has type `['a, error]` where `'a` is the array element type. If
the index is valid, `'a` is returned, otherwise an error is returned. We can
combine this with `?` which is essentially a compact syntax for

```
select v {
  error as e => errors <- e,
  v => v
}
```

If `v` is an error, then the error is sent to the closest `errors` binding, and
the select evaluates to bottom (because that's what connect evaluates to).
Otherwise the select evaluates to v. Consider,

```
let a = [1, 2, 3, 4, 5];
let errors = never(); // create a local error handler
println(errors);
a[10]?;
a[4]? + 1
```

Since the first index of `a` is out of bounds it will return an error, `?` will
take the error and send it to the local error handler, which `print` depends on,
so the error will be printed. The second index is valid, so `?` will return 5.

```
eric@katana ~> proj/graphix/target/debug/graphix ./test.gx
error:"in file \"/home/eric/test.gx\" at line: 4, column: 1 array index out of bounds"
6
```
