# Named Types

You can name types to avoid having to type them more than once. Named types are
just aliases for the full structure of the type they reference. The fully
written out type is the same as the alias and visa versa. Lets go back to our
foods example from the section on variants.

```
type FoodKind = [
  `Vegetable,
  `Fruit,
  `Meat,
  `Grain,
  `Other(string)
];

let v: FoodKind = `Vegetable;
let f = |food: FoodKind| ...
```

Aliases are very useful for more complex types that are used many times.
Selective annotations can also help the type checker make sense of complex
program structures.

In the next section you'll see that we can do a lot more with them.
