# Blocks

A block is a group of code between `{` and `}` that has it's own scope, and
evaluates to the last value in the block. Expressions in a block are `;`
separated, meaning every expression except the last one must end in a `;` and
it is illegal for a block to have just one expression (it will not parse).

You can use blocks to hide intermediate variables from outer scopes, and to
group code together in a logical way.

```graphix
let important_thing = {
  let x = 0;
  let y = x + 1;
  43 - y
};

x; // compile error, x isn't in scope
y; // compile error, y isn't in scope
important_thing
```

This program won't compile because you can't reference y and x from
outside the block scope. Blocks are valid anywhere an expression is
valid, and they are just expressions. They will become very important
when we introduce lambda expressions.
