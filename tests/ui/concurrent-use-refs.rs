use ref_stable_lru::LruCache;
use std::num::NonZeroUsize;

fn main() {
    let mut cache: LruCache<&'static str, String> = LruCache::new(NonZeroUsize::new(3).unwrap());

    let out = cache.scope(|mut handle, mut perm| {
        handle.put("a", "bb".to_string(), &mut perm);
        handle.put("b", "cc".to_string(), &mut perm);
        handle.put("c", "dd".to_string(), &mut perm);

        let futs = ["a", "b", "c"].iter().map(|k| {
            let v = handle.get(k, &perm).unwrap();

            async {
                // Assert v is a reference.
                let v: &String = v;
                v.get(..1).unwrap().to_string()
            }
        });

        let fut = async {
            let out = futures::future::join_all(futs).await;
            out.join(" ")
        };

        futures::executor::block_on(fut)
    });

    assert_eq!(out, "b c d".to_string());
}
