# LRU Cache with compile-time reference stability

The main codes are copy from [lru-rs](https://github.com/jeromefroe/lru-rs), very grateful for the project.

The main motivation for implementing this project is that `LRUCache` should allow multiple immutable references obtained through `get` method.

The main idea is separating the value operating permissions from the data structure itself. I'll give an elaboration in my blog post later.

## Example

Below is a simple example shows the main feature of `LRUCache`.

```rust
let mut cache = LruCache::new(NonZeroUsize::new(2).unwrap());

cache.scope(|mut cache, mut perm| {
    assert_eq!(cache.put("apple", "red", &mut perm), None);
    assert_eq!(cache.put("banana", "yellow", &mut perm), None);
    assert_eq!(cache.put("lemon", "yellow", &mut perm), Some("red"));

    let colors: Vec<_> = ["apple", "banana", "lemon", "watermelon"]
        .iter()
        .map(|k| cache.get(k, &perm))
        .collect();
    assert!(colors[0].is_none());
    assert_opt_eq(colors[1], "yellow");
    assert_opt_eq(colors[2], "yellow");
    assert!(colors[3].is_none());
});
```
