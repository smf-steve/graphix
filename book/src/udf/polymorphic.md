# Parametric Polymorphism

We can define type variables as part of the definition of named types and then
use them in the type definition in order to create type aliases with type
parameters. For example, suppose in our foods example we wanted to specify that
`Other could carry a value other than a string,

```
type FoodKind<'a> = [
  `Vegetable,
  `Fruit,
  `Meat,
  `Grain,
  `Other('a)
];
let v: FoodKind<`Cookie> = `Other(`Cookie);
v
```

if we paste this program into a file and run it we get,

```
eric@mazikeen ~/p/graphix (main) [1]> target/debug/graphix ~/test.gx
`Other(`Cookie)
```

We can even place constraints on the type that a type variable can take. For example,

```
type Point3<'a: Number> = {x: 'a, y: 'a, z: 'a};
let f = |p: Point3<'a>, x: 'a| {p with x: p.x + x};
f({x: 0., y: 1., z: 3.14})
```
