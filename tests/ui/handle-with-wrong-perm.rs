use ref_stable_lru::LruCache;
use std::num::NonZeroUsize;

fn main() {
    let mut cache1: LruCache<&'static str, String> = LruCache::new(NonZeroUsize::new(3).unwrap());

    // Use an inner `perm` to operate outer `handle`.
    cache1.scope(|mut handle1, perm1| {
        let mut cache2: LruCache<&'static str, String> =
            LruCache::new(NonZeroUsize::new(3).unwrap());

        // We don't really want to use `handle2`, but construct a fake `ValuePerm` and try to modify `cache1`.
        cache2.scope(|_handle2, mut perm2| {
            let x = handle1.get(&"a", &perm1).unwrap().as_str();
            let y = handle1.get(&"b", &perm1).unwrap().as_str();
            let z = handle1.get(&"c", &perm1).unwrap().as_str();
            // Should fail here due to lifetime (`'brand`) not match.
            handle1.put("a", "".to_string(), &mut perm2);
            [x, y, z].join(" ")
        })
    });

    // Use an outer `perm` to operate inner `handle`.
    // We don't really want to use `handle1`, but construct a fake `ValuePerm` and try to modify `cache2`.
    cache1.scope(|_handle1, mut perm1| {
        let mut cache2: LruCache<&'static str, String> =
            LruCache::new(NonZeroUsize::new(3).unwrap());

        cache2.scope(move |mut handle2, perm2| {
            let x = handle2.get(&"a", &perm2).unwrap().as_str();
            let y = handle2.get(&"b", &perm2).unwrap().as_str();
            let z = handle2.get(&"c", &perm2).unwrap().as_str();
            // Should fail here due to lifetime (`'brand`) not match.
            handle2.put("a", "".to_string(), &mut perm1);
            [x, y, z].join(" ")
        })
    });
}
