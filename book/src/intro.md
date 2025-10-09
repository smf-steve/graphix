# Intro to Graphix

Graphix is a programming language using the dataflow paradigm. It is
particularly well suited to building user interfaces, and interacting with
resources in [netidx](https://netidx.github.io/netidx-book). Dataflow languages
like Graphix are "reactive", like the popular UI library, except at the language
level instead of just as a library. A Graphix program is compiled to a directed
graph, operations (such as +) are graph nodes, edges represent paths data can
take through the program. Consider,

```
2 + 2
```

This compiles to a graph like,

const(2) ==> + <== const(2)

When executed the program will have 1 output, 4. Which is exactly what you'd
expect and is no different from a non data flow program. We need a
more complex example to see the difference,

```
let x = cast<i64>(net::subscribe("/foo")?)?;
print(x * 10)
```

net::subscribe, subscribes to a netidx path and returns it's value, or an error
(more on ? later). Now lets see what happens, the graph we get from this program
looks something like this,

```
                                   const(10) ==
                                                |
                                                |
                                                v
const("/foo") => net::subscribe => cast<i64> => * => print
```

Unlike the first example, the value of `net::subscribe` isn't a constant, it can
change if the value published in netidx changes. Graphix programs never
terminate on their own, they are just graphs, if one of their dependent nodes
changes, then they update. So if the published value of "/foo" is initially 10,
then this program will print 100, if the value of "/foo" changes to 5 then the
output of the program will change to 50, and so on forever.

This is a powerful way to think about programming, and it's especially well
suited to building user interfaces and transforming data streams.

Besides being a dataflow language Graphix tries hard to be a normal language
that would feel familiar to anyone who knows a modern functional programming
language. Some of it's features are,

- lexically scoped
- expression oriented
- strongly statically typed
- type inference
- structural type discipline
- parametric polymorphism
- algebraic data types
- pattern matching
- first class functions, and closures
