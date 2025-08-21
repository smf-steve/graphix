# Select

Select lets us create a graph node with multiple possible output paths that
will choose one path for each value based on a set of conditions. Kind of like,

```
                   | if foo > 0 =>  ...
                   |
                   |
ref(foo) => select | if foo < 0 => ...
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

select takes an expression as an argument and then evaluates one or more "arms".
Each arm consists of an optional type predicate, a destructuring pattern, and an
optional guard clause. If the type predicate matches, the pattern matches, and
the guard evaluates to true then the arm is "selected". Only one arm may be
selected at a time, the arms are evaluated in lexical order, and first arm to be
selected is chosen as the one and only selected arm.

The code on the right side of the selected arm is the only code that is
evaluated by select, all other code is "asleep", it will not be evaluated
until it is selected (and if it has netidx subscriptions or published values
they will be unsubscribed and unpublished until it is selected again).

## Matching Types

Consider we want to select from a value of type `[Array<i64>, i64, null]`,

```
let x: [Array<i64>, i64, null] = null;
x <- time::timer(duration:1.s, false) ~ [1, 2, 3, 4, 5];
x <- time::timer(duration:2.s, false) ~ 7;
select x {
  Array<i64> as a => array::fold(a, 0, |s, x| s + x),
  i64 as n => n,
  null as _ => 42
}
```
