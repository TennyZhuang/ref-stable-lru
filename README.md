# LRU Cache with compile-time reference stability

* [crate](https://crates.io/crates/ref-stable-lru)
* [blog post][blog_post]

The main codes are copy from [lru-rs](https://github.com/jeromefroe/lru-rs), very grateful for the project.

The main motivation for implementing this project is that `LRUCache` should allow multiple immutable references obtained through `get` method.

The main idea is separating the value operating permissions from the data structure itself. [The blog post][blog_post] elaborates the idea. You can also take a look at [uitest](./tests/ui/test.rs), which explains the API design goals.

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

[blog_post]:https://blog.cocl2.com/posts/rust-ref-stable-collection/
