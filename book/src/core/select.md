# Select

Select lets us create a graph node with multiple possible output paths that
will choose one path for each value based on a set of conditions. Kind of like,

```
                     | if foo > 0 =>  ...
                     |
                     |
ref(foo) => select == if foo < 0 => ...
                     |
                     |
                     | otherwise => ...
```

is written as

```
select foo {
  n if n > 0 => ...,
  n if n < 0 => ...,
  n => ...
}
```
