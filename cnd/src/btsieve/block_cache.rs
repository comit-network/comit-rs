macro_rules! block_cache {
    ($type:ident) => {
        pub struct $type {
            map: Arc<Mutex<LruCache<Hash, Block>>>,
        }

        impl Clone for $type {
            fn clone(&self) -> Self {
                Self {
                    map: Arc::clone(&self.map),
                }
            }
        }

        impl std::fmt::Debug for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self.map)
            }
        }

        impl $type {
            fn new(capacity: usize) -> Self {
                let map: LruCache<Hash, Block> = LruCache::new(capacity);
                Self {
                    map: Arc::new(Mutex::new(map)),
                }
            }
        }
    };
}
