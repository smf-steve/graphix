# 0.1.11

- add map built-in type, O(log(N)) lookup, insert, remove. Based on a
  memory pooled immutable-chunkmap
  
- introduce try catch. ? will now send errors to the nearest catch in
  dynamic scope.
  
- introduce or never operator $, which will return the non error value
  or never

- a lot of type checker and compiler bugs fixed
