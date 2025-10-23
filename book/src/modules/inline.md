# Inline Modules

In a single file, you can define modules inline. Expressions in modules are semi
colon separated. expressions defined in a module may be referred to directly
e.g. `m::name` or by bringing the module into scope with `use m`.

```graphix
mod m {
  let hello = "hello world";
  let goodbye = "goodbye world"
};

"we say [m::hello] followed by [m::goodbye]"
```

running this we get,

```
eric@katana ~/p/graphix (main) [1]> target/debug/graphix ~/test.gx
"we say hello world followed by goodbye world"
```
