use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::num::NonZeroUsize;
use std::ptr;
use std::ptr::NonNull;

type InvariantLifetime<'brand> = PhantomData<fn(&'brand ()) -> &'brand ()>;

pub struct CacheHandle<'cache, 'brand, K, V> {
    _lifetime: InvariantLifetime<'brand>,
    cache: &'cache mut LruCache<K, V>,
}

pub struct ValuePerm<'brand> {
    _lifetime: InvariantLifetime<'brand>,
}

// Struct used to hold a reference to a key
struct KeyRef<K> {
    k: *const K,
}

impl<K: Hash> Hash for KeyRef<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { (*self.k).hash(state) }
    }
}

impl<K: PartialEq> PartialEq for KeyRef<K> {
    fn eq(&self, other: &KeyRef<K>) -> bool {
        unsafe { (*self.k).eq(&*other.k) }
    }
}

impl<K: Eq> Eq for KeyRef<K> {}

// Struct used to hold a key value pair. Also contains references to previous and next entries
// so we can maintain the entries in a linked list ordered by their use.
struct LruEntry<K, V> {
    key: mem::MaybeUninit<K>,
    val: mem::MaybeUninit<V>,
    prev: *mut LruEntry<K, V>,
    next: *mut LruEntry<K, V>,
}

impl<K, V> LruEntry<K, V> {
    fn new(key: K, val: V) -> Self {
        LruEntry {
            key: mem::MaybeUninit::new(key),
            val: mem::MaybeUninit::new(val),
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }

    fn new_sigil() -> Self {
        LruEntry {
            key: mem::MaybeUninit::uninit(),
            val: mem::MaybeUninit::uninit(),
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }
}

pub struct LruCache<K, V> {
    map: HashMap<KeyRef<K>, NonNull<LruEntry<K, V>>>,
    cap: NonZeroUsize,

    // head and tail are sigil nodes to facilitate inserting entries
    head: *mut LruEntry<K, V>,
    tail: *mut LruEntry<K, V>,
}

impl<K: Eq + Hash, V> LruCache<K, V> {
    pub fn new(cap: NonZeroUsize) -> Self {
        let cache = LruCache::<K, V> {
            map: HashMap::with_capacity(cap.get()),
            cap,
            head: Box::into_raw(Box::new(LruEntry::new_sigil())),
            tail: Box::into_raw(Box::new(LruEntry::new_sigil())),
        };

        unsafe {
            (*cache.head).next = cache.tail;
            (*cache.tail).prev = cache.head;
        };

        cache
    }

    pub fn scope<'cache, F, R>(&'cache mut self, fun: F) -> R
    where
        for<'brand> F: FnOnce(CacheHandle<'cache, 'brand, K, V>, ValuePerm<'brand>) -> R,
    {
        let handle = CacheHandle {
            _lifetime: Default::default(),
            cache: self.into(),
        };
        let perm = ValuePerm {
            _lifetime: InvariantLifetime::default(),
        };
        fun(handle, perm)
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn cap(&self) -> NonZeroUsize {
        self.cap
    }

    fn detach(&mut self, node: *mut LruEntry<K, V>) {
        unsafe {
            (*(*node).prev).next = (*node).next;
            (*(*node).next).prev = (*node).prev;
        }
    }

    // Attaches `node` after the sigil `self.head` node.
    fn attach(&mut self, node: *mut LruEntry<K, V>) {
        unsafe {
            (*node).next = (*self.head).next;
            (*node).prev = self.head;
            (*self.head).next = node;
            (*(*node).next).prev = node;
        }
    }

    fn replace_or_create_node(&mut self, k: K, v: V) -> (Option<(K, V)>, NonNull<LruEntry<K, V>>) {
        if self.len() == self.cap().get() {
            // if the cache is full, remove the last entry so we can use it for the new key
            let old_key = KeyRef {
                k: unsafe { &(*(*(*self.tail).prev).key.as_ptr()) },
            };
            let old_node = self.map.remove(&old_key).unwrap();
            let node_ptr: *mut LruEntry<K, V> = old_node.as_ptr();

            // read out the node's old key and value and then replace it
            let replaced = unsafe {
                (
                    mem::replace(&mut (*node_ptr).key, mem::MaybeUninit::new(k)).assume_init(),
                    mem::replace(&mut (*node_ptr).val, mem::MaybeUninit::new(v)).assume_init(),
                )
            };

            self.detach(node_ptr);

            (Some(replaced), old_node)
        } else {
            // if the cache is not full allocate a new LruEntry
            // Safety: We allocate, turn into raw, and get NonNull all in one step.
            (None, unsafe {
                NonNull::new_unchecked(Box::into_raw(Box::new(LruEntry::new(k, v))))
            })
        }
    }
}

impl<'cache, 'brand, K: Hash + Eq, V> CacheHandle<'cache, 'brand, K, V> {
    pub fn len<'handle, 'perm>(&'handle self) -> usize {
        self.cache.len()
    }

    pub fn is_empty<'sperm>(&self) -> bool {
        self.len() == 0
    }

    pub fn cap<'sperm>(&self) -> NonZeroUsize {
        self.cache.cap()
    }

    pub fn put<'handle, 'perm>(
        &'handle mut self,
        k: K,
        mut v: V,
        _perm: &'perm mut ValuePerm<'brand>,
    ) -> Option<V> {
        let cache = &mut self.cache;
        let node_ref = cache.map.get_mut(&KeyRef { k: &k });

        match node_ref {
            Some(node_ref) => {
                // if the key is already in the cache just update its value and move it to the
                // front of the list
                let node_ptr: *mut LruEntry<K, V> = node_ref.as_ptr();
                let node_ref = unsafe { &mut (*(*node_ptr).val.as_mut_ptr()) };
                mem::swap(&mut v, node_ref);
                let _ = node_ref;
                cache.detach(node_ptr);
                cache.attach(node_ptr);
                Some(v)
            }
            None => {
                let (replaced, node) = cache.replace_or_create_node(k, v);
                let node_ptr: *mut LruEntry<K, V> = node.as_ptr();

                cache.attach(node_ptr);

                let keyref = unsafe { (*node_ptr).key.as_ptr() };
                cache.map.insert(KeyRef { k: keyref }, node);

                replaced.map(|(_k, v)| v)
            }
        }
    }

    pub fn get<'handle, 'perm>(
        &mut self,
        k: &K,
        _perm: &'perm ValuePerm<'brand>,
    ) -> Option<&'perm V> {
        let cache = &mut self.cache;
        if let Some(node) = cache.map.get_mut(&KeyRef { k }) {
            let node_ptr: *mut LruEntry<K, V> = node.as_ptr();

            cache.detach(node_ptr);
            cache.attach(node_ptr);

            Some(unsafe { &*(*node_ptr).val.as_ptr() })
        } else {
            None
        }
    }

    // get the mutable reference of an entry, but not adjust its position.
    pub fn peek_mut<'handle, 'key, 'perm>(
        &'handle self,
        k: &'key K,
        _perm: &'perm mut ValuePerm<'brand>,
    ) -> Option<&'perm mut V> {
        let cache = &self.cache;
        match cache.map.get(&KeyRef { k }) {
            None => None,
            Some(node) => Some(unsafe { &mut *(*node.as_ptr()).val.as_mut_ptr() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;

    fn assert_opt_eq<V: PartialEq + Debug>(opt: Option<&V>, v: V) {
        assert!(opt.is_some());
        assert_eq!(opt.unwrap(), &v);
    }

    fn assert_opt_eq_mut<V: PartialEq + Debug>(opt: Option<&mut V>, v: V) {
        assert!(opt.is_some());
        assert_eq!(opt.unwrap(), &v);
    }

    #[test]
    fn test_put_and_get() {
        let mut cache = LruCache::new(NonZeroUsize::new(2).unwrap());
        cache.scope(|mut cache, mut perm| {
            assert_eq!(cache.put("apple", "red", &mut perm), None);
            assert_eq!(cache.put("banana", "yellow", &mut perm), None);

            assert_eq!(cache.cap().get(), 2);
            assert_eq!(cache.len(), 2);
            assert!(!cache.is_empty());
            assert_opt_eq(cache.get(&"apple", &perm), "red");
            assert_opt_eq(cache.get(&"banana", &perm), "yellow");
        });
    }

    #[test]
    fn test_multi_get() {
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
    }

    #[test]
    fn test_peek_mut() {
        let mut cache = LruCache::new(NonZeroUsize::new(2).unwrap());

        cache.scope(|mut cache, mut perm| {
            cache.put("apple", "red", &mut perm);
            cache.put("banana", "yellow", &mut perm);

            assert_opt_eq_mut(cache.peek_mut(&"banana", &mut perm), "yellow");
            assert_opt_eq_mut(cache.peek_mut(&"apple", &mut perm), "red");
            assert!(cache.peek_mut(&"pear", &mut perm).is_none());

            cache.put("pear", "green", &mut perm);

            assert!(cache.peek_mut(&"apple", &mut perm).is_none());
            assert_opt_eq_mut(cache.peek_mut(&"banana", &mut perm), "yellow");
            assert_opt_eq_mut(cache.peek_mut(&"pear", &mut perm), "green");

            {
                let v = cache.peek_mut(&"banana", &mut perm).unwrap();
                *v = "green";
            }

            assert_opt_eq_mut(cache.peek_mut(&"banana", &mut perm), "green");
        });
    }
}
