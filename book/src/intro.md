# Intro to Graphix

The first goal of Graphix is to make it easy to build user interfaces to
display, interact with, and control resources published in
[netidx](https://netidx.github.io/netidx-book)(The second goal is to be the
weirdest practical language on earth). As such graphix is a "reactive" or
dataflow language. Instead of compiling to machine code, or bytecode like other
languages graphix programs compile to a directed graph. Operations like + are
graph nodes, and edges represent paths between nodes that data can take. Running
the program means starting the flow of data into the graph so that it will flow
through, and be transformed by, the nodes of the graph. Consider,

```
let x = cast<i64>(net::subscribe("/foo")?)?;
print(x * 10)
```

net::subscribe, subscribes to a netidx path and returns it's value, or an error
(more on ? later). If you read this like a "normal" functional or imperative
program, it appears to get the current value of "/foo", multiply it by 10 and
print the result, no other explanation really makes sense. However in the dataflow paradigm, this compiles to a graph,

```
                                   const(10) ==
                                                |
                                                |
                                                v
const("/foo") => net::subscribe => cast<i64> => * => print
```

net::subscribe's value will change when the value in netidx changes, and that
new value will flow through the graph and be transformed by all the nodes it
passes through. So in graphix, this program will print the current value of
"/foo" in netix multiplied by 10, and when that value changes it will print an
updated value.

This is a powerful way to think about programming, and it's especially well
suited to building user interfaces and transforming data streams, as we will see
in this book.

In all other respects Graphix aims to be a normal language that would feel
familair to anyone who knows OCaml, Rust, Haskell, or another similar modern
language.

- It is lexically scoped
- It is expression oriented, every language construct is an expression that
  results in a value
- It is strongly statically typed, using type erasure on top of a flexible
  universal variant type. Making it both good at catching errors at compile
  time, and flexible at run time.
- It has extensive type inference capability, such that type annotations are
  not needed very often
- It's typing discipline is structural rather than nomial, but named type aliases
  are supported. This is different than most languages, but is useful for a
  "scripting language"
- It has parametric polymorphism for both lambdas and type aliases
- It has algebreic data types
- destructuring pattern matches are supported in select, let,
  and lambda arguments
