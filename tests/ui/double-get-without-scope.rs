use ref_stable_lru::LruCache;
use std::num::NonZeroUsize;

fn main() {
    let mut cache: LruCache<&'static str, String> = LruCache::new(NonZeroUsize::new(2).unwrap());

    let x = cache.get(&"a").unwrap().as_str();
    // Should failed here, since `x` already mutually borrowed cache.
    let y = cache.get(&"b").unwrap().as_str();
    let z = cache.get(&"c").unwrap().as_str();
    [x, y, z].join(" ");
}
