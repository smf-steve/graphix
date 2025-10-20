# Inline Modules

In a file, you can define modules inline using the following syntax,

```
mod m {
  let name = value;
  expr
}
```

Expressions in modules are semi colon separated. At the moment everything in a
module is public, and can be referred to either directly `m::name` or by
bringing the module into scope with `use m`.
