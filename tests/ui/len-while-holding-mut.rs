use ref_stable_lru::LruCache;
use std::num::NonZeroUsize;

fn main() {
    let mut cache: LruCache<&'static str, String> = LruCache::new(NonZeroUsize::new(3).unwrap());

    cache.scope(|mut handle, mut perm| {
        handle.put("a", "b".to_string(), &mut perm);
        let r = handle.peek_mut(&"a", &mut perm).unwrap();
        // We can call `len` here, since `&mut V` can't change the structure of the cache.
        assert_eq!(handle.len(), 1);
        // Use r here.
        *r = "c".to_string();
    });
}
