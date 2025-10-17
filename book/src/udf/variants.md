# Variants

Variants allow you to define a case that belongs to a set of possible cases a
value is allowed to be. For example we might categorize foods,

```
[`Vegetable, `Fruit, `Meat, `Grain, `Other(string)]
```

Here we've defined a set of variants that together cover all the cases we want
to model. We can write a function that will only accept a member of this set,

```
let f = |food: [`Vegetable, `Fruit, `Meat, `Grain, `Other(string)]| ...
```

and the type checker will ensure that it is an error caught at compile time to
pass any other type of value to this function. The most interesting variant in this set is probably

```
`Other(string)
```

Because it carries data with it. Variant cases can carry an zero or more values
with them. We can use pattern matching to extract these values at run time. Lets
write the body of our food function,

```
let f = |food: [`Vegetable, `Fruit, `Meat, `Grain, `Other(string)]| select food {
  `Vegetable => "it's a vegetable",
  `Fruit => "it's a fruit",
  `Meat => "it's meat",
  `Grain => "it's grain",
  `Other(s) => "it's a [s]"
};
f(`Other("foo"))
```

If we copy the above into a file and run it we will get,

```
eric@mazikeen ~/p/graphix (main) [1]> target/debug/graphix ~/test.gx
"it's a foo"
```

In this example the type checker will ensure that,

- every item in the set is matched by a non guarded arm of the select (see the section on select)
- no extra items that can't exist in the set are matched
- you can't pass anything to f that isn't in the set

Single variant cases are actually a perfectly valid type in Graphix, although
they are much more useful in sets. Once we start naming types (in a later
section), they will become even more useful.
