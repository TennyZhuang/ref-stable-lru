use ref_stable_lru::LruCache;
use std::num::NonZeroUsize;

fn main() {
    let mut cache: LruCache<&'static str, String> = LruCache::new(NonZeroUsize::new(3).unwrap());

    let out = cache.scope(|mut handle, mut perm| {
        handle.put("a", "b".to_string(), &mut perm);
        handle.put("b", "c".to_string(), &mut perm);
        handle.put("c", "d".to_string(), &mut perm);

        let x = handle.get(&"a", &perm).unwrap().as_str();
        let y = handle.get(&"b", &perm).unwrap().as_str();
        let z = handle.get(&"c", &perm).unwrap().as_str();

        // Drop the `handle` means that no one can operate the `cache` in the scope, so references are always valid.
        drop(handle);

        [x, y, z].join(" ")
    });

    assert_eq!(out, "b c d".to_string());
}
