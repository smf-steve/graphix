# External Modules

External modules are defined by creating a file `m.gx` containing Graphix
expressions. Expressions in `m.gx` are treated the same as if they were defined
in an inline module `mod m { ... }`.

Module hierarchies can be created using directories, for example to create
`m::n` you would create a directory `m` and in it a file called `mod.gx` and a
file called `n.gx`

```
m/mod.gx
-------------
mod n

m/n.gx
let name = expr
```
