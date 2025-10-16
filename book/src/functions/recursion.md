# Recursion

Functions can be recursive, however there is currently no tail call optimization,
so you can easily exhaust available stack space. With that warning aside, lets
write a recursive function to add up pairs of numbers in an array,

```
let rec add_pairs = 'a: Number |a: Array<'a>| -> Array<'a> select a {
  [e0, e1, tl..] => array::push_front(add_pairs(tl), e0 + e1),
  a => a
}
```

running this we see,

```
〉add_pairs([1, 2, 3, 4, 5])
-: Array<'a: i64>
[3, 7, 5]
```
