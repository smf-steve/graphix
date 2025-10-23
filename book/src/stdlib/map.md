# Map

```graphix
mod map: sig {
    /// return a new map where each element is the output of f applied to
    /// the corresponding key value pair in the current map
    val map: fn(Map<'a, 'b>, fn(('a, 'b)) -> ('c, 'd) throws 'e) -> Map<'c, 'd> throws 'e;

    /// return a new map containing only the key-value pairs where f applied to
    /// (key, value) returns true
    val filter: fn(Map<'a, 'b>, fn(('a, 'b)) -> bool throws 'e) -> Map<'a, 'b> throws 'e;

    /// filter_map returns a new map containing the outputs of f
    /// that were not null
    val filter_map: fn(Map<'a, 'b>, fn(('a, 'b)) -> Option<('c, 'd)> throws 'e) -> Map<'c, 'd> throws 'e;

    /// return the result of f applied to the init and every k, v pair of m in
    /// sequence. f(f(f(init, (k0, v0)), (k1, v1)), ...)
    val fold: fn(Map<'a, 'b>, 'c, fn('c, ('a, 'b)) -> 'c throws 'e) -> 'c throws 'e;

    /// return the length of the map
    val len: fn(Map<'a, 'b>) -> i64;

    /// get the value associated with the key k in the map m, or null if not present
    val get: fn(Map<'a, 'b>, 'a) -> Option<'b>;

    /// insert a new value into the map
    val insert: fn(Map<'a, 'b>, 'a, 'b) -> Map<'a, 'b>;

    /// remove the value associated with the specified key from the map
    val remove: fn(Map<'a, 'b>, 'a) -> Map<'a, 'b>;

    /// iter produces an update for every key-value pair in the map m.
    /// updates are produced in the order they appear in m.
    val iter: fn(Map<'a, 'b>) -> ('a, 'b);

    /// iterq produces an update for each value in m, but only when clock updates. If
    /// clock does not update but m does, then iterq will store each m in an internal
    /// fifo queue. If clock updates but m does not, iterq will record the number of
    /// times it was triggered, and will update immediately that many times when m
    /// updates.
    val iterq: fn(#clock:Any, Map<'a, 'b>) -> ('a, 'b);
}
```
