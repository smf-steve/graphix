# External Modules

A simple external module may be defined in either a file or by a value in
netidx. In the case of a file, the file name must end in `.gx` and the part
before that is the name of the module. For example a file `m.gx` contains the
expressions defining the module `m`. Expressions in the file do not need to be
surrounded by a `mod m { ... }`, the name of the module is taken from the
filename.

In netidx, a string value containing Graphix source is published, and the final
name in the path is the name of the module, no `.gx` is required for netidx
modules. For example we might publish `/libs/graphix/strops`, the name of the
module would be `strops`, and the resolver expects it to be a string containing
valid Graphix code, just like a file.

Here is a simple example with file modules,

```
eric@katana ~/t/ex> ls
m.gx  test.gx
```

`test.gx` is the program that we will run, `m.gx` is a module it will load.

`test.gx`
```
mod m;

m::hello
```

`m.gx`
```
let hello = "hello world"
```

running this we get,

```
eric@katana ~/p/graphix (main)> target/debug/graphix ~/tmp/ex/test.gx
"hello world"
```

## Module Load Path

The graphix shell reads the `GRAPHIX_MODPATH` environment variable at startup
and appends it's contents to the built in list of module paths. The syntax is a
comma separated list of paths. Paths that start with `netidx:` are netidx paths,
otherwise file paths are expected. The comma separator can be escaped with `\`.
For example,

```
GRAPHIX_MODPATH=netidx:/foo,/home/eric/graphix-modules,/very/str\,ange/path
```

would add
- netidx:/foo
- /home/eric/graphix-modules
- /very/str,ange/path

to the Graphix module path

### Default Module Path

By default the module resolve path has several entries,
- the parent directory of the program file passed on the command line. e.g. if
  we are running `/home/eric/test.gx` then Graphix will look for modules in
  `/home/eric`

- the Graphix init directory. This is a platform specific directory where you
  can put Graphix modules.
  - On Linux `~/.local/share/graphix`
  - On Windows `%APPDATA%\Roaming\graphix`
  - On Mac OS `~/Library/Application Support/graphix`

In REPL mode, which is when it's given no argument, the `graphix` command will
try to load the module `init`. If no such module exists it will silently carry
on. You can use this to load commonly used utilities in the repl automatically.

## Modules in Netidx

We can publish the same code as the files example in netidx and use it in
Graphix directly, but we have to run it in a slightly different way, first lets
publish it,

```
eric@katana ~/t/ex> printf \
  "/local/graphix/test|string|%s\n/local/graphix/m|string|%s" \
  "$(tr \n ' ' <test.gx)" "$(tr \n ' ' <m.gx)" \
  | netidx publisher
```

Graphix doesn't care about whitespaces like newline, so we can just translate
them to spaces to avoid confusing the command line publisher. Lets see if we
published successfully.

```
eric@katana ~> netidx subscriber /local/graphix/test
/local/graphix/test|string|"mod m;  m::hello"
```

Looks good, now lets run the code. In order to do this we need to add to the
resolve path to tell the Graphix shell where it should look for modules. We also
don't pass a `.gx` extension, so we are telling Graphix to look for a module
named `test` in it's configured module paths and run that.

```
eric@katana ~/p/graphix (main)> GRAPHIX_MODPATH=netidx:/local/graphix \
  target/debug/graphix test
"hello world"
```

## Module Hierarchies

Module hierarchies can be created using directories, for example to create
`m::n` you would create a directory `m` and in it a file called `mod.gx` and a
file called `n.gx`

```
eric@katana ~/t/ex1> find .
.
./m
./m/mod.gx
./m/n.gx
./test.gx
```

`test.gx` is the root of the hierarchy
```
mod m;

m::n::hello
```

`m/mod.gx` is the root of module `m`
```
mod n
```

`m/n.gx` is the `m::n` module
```
let hello = "hello world"
```

if we run the program we get,

```
eric@katana ~/p/graphix (main)> target/debug/graphix ~/tmp/ex1/test.gx
"hello world"
```

## Module Hierarchies in Netidx

Module hierarchies in netidx work the same as in the file system except that
`mod.gx` is never needed because in `netidx` a value can also be a container. So
to replicate the above example we'd publish,

```
/lib/graphix/test <- the contents of test.gx
/lib/graphix/m    <- the contents of m/mod.gx
/lib/graphix/m/n  <- the contents of m/n.gx
```
