//! Sharded read-write in-memory asset cache with per-key single-flight deduplication.
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Condvar, Mutex, RwLock};

const SHARDS: usize = 16;

type CacheKey = (TypeId, String);
type CachedValue = Arc<dyn Any + Send + Sync>;

#[derive(Debug)]
enum LoadOutcome {
    Pending,
    Ready(CachedValue),
    Failed(String),
}

#[derive(Debug)]
struct InflightLoad {
    state: Mutex<LoadOutcome>,
    done: Condvar,
}

impl InflightLoad {
    fn new() -> Self {
        Self {
            state: Mutex::new(LoadOutcome::Pending),
            done: Condvar::new(),
        }
    }

    fn finish(&self, outcome: LoadOutcome) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = outcome;
            self.done.notify_all();
        }
    }

    fn wait_clone<T: 'static + Clone>(&self) -> Result<T, String> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| String::from("asset load lock poisoned"))?;
        while matches!(*guard, LoadOutcome::Pending) {
            guard = self
                .done
                .wait(guard)
                .map_err(|_| String::from("asset load condvar poisoned"))?;
        }
        match &*guard {
            LoadOutcome::Ready(arc) => arc
                .downcast_ref::<T>()
                .cloned()
                .ok_or_else(|| String::from("cached asset type mismatch")),
            LoadOutcome::Failed(error) => Err(error.clone()),
            LoadOutcome::Pending => unreachable!(),
        }
    }
}

enum CacheSlot {
    Ready(CachedValue),
    Loading(Arc<InflightLoad>),
}

impl std::fmt::Debug for CacheSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready(_) => f.write_str("Ready(..)"),
            Self::Loading(_) => f.write_str("Loading(..)"),
        }
    }
}

/// Thread-safe sharded asset cache sharing loaded values by `(TypeId, path)`.
///
/// Concurrent cache hits acquire only per-shard read locks, and concurrent cache misses for
/// distinct keys run their loaders in parallel outside any shard lock while duplicate misses
/// for the same key coalesce onto a single in-flight loader.
#[derive(Debug)]
pub struct AssetCache {
    shards: [RwLock<HashMap<CacheKey, CacheSlot>>; SHARDS],
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new()
    }
}

struct InflightGuard<'a> {
    shard: &'a RwLock<HashMap<CacheKey, CacheSlot>>,
    key: CacheKey,
    inflight: Arc<InflightLoad>,
    settled: bool,
}

impl Drop for InflightGuard<'_> {
    fn drop(&mut self) {
        if !self.settled {
            if let Ok(mut map) = self.shard.write()
                && matches!(
                    map.get(&self.key),
                    Some(CacheSlot::Loading(existing)) if Arc::ptr_eq(existing, &self.inflight)
                )
            {
                map.remove(&self.key);
            }
            self.inflight
                .finish(LoadOutcome::Failed(String::from("asset loader aborted")));
        }
    }
}

fn shard_index(key: &(TypeId, &str)) -> usize {
    #[derive(Default)]
    struct MixHasher(u64);
    impl Hasher for MixHasher {
        fn finish(&self) -> u64 {
            let x = self.0.wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 29)
        }
        fn write(&mut self, bytes: &[u8]) {
            for &b in bytes {
                self.0 = (self.0 ^ u64::from(b)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            }
        }
        fn write_u64(&mut self, i: u64) {
            self.0 = (self.0 ^ i).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        }
    }
    let mut h = MixHasher(0x517c_c1b7_2722_0a95);
    key.0.hash(&mut h);
    h.write(key.1.as_bytes());
    (h.finish() as usize) & (SHARDS - 1)
}

impl AssetCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            shards: std::array::from_fn(|_| RwLock::new(HashMap::new())),
        }
    }

    /// Cache-lookup-then-load pattern for repeated assets.
    ///
    /// # Errors
    /// Propagates errors from `loader` or lock poisoning.
    pub fn load_or_cache<T: 'static + Clone + Send + Sync>(
        &self,
        path: &str,
        loader: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let type_id = TypeId::of::<T>();
        let shard = &self.shards[shard_index(&(type_id, path))];
        {
            let map = shard
                .read()
                .map_err(|_| String::from("asset cache read lock poisoned"))?;
            if let Some(CacheSlot::Ready(cached)) = map.get(&(type_id, path.to_owned())) {
                return cached
                    .downcast_ref::<T>()
                    .cloned()
                    .ok_or_else(|| String::from("cached asset type mismatch"));
            }
        }

        enum SlotAction<T> {
            Hit(Result<T, String>),
            Wait(Arc<InflightLoad>),
            Load(Arc<InflightLoad>),
        }

        let key = (type_id, path.to_owned());
        let action = {
            let mut map = shard
                .write()
                .map_err(|_| String::from("asset cache write lock poisoned"))?;
            match map.get(&key) {
                Some(CacheSlot::Ready(cached)) => SlotAction::Hit(
                    cached
                        .downcast_ref::<T>()
                        .cloned()
                        .ok_or_else(|| String::from("cached asset type mismatch")),
                ),
                Some(CacheSlot::Loading(existing)) => SlotAction::Wait(Arc::clone(existing)),
                None => {
                    let inflight = Arc::new(InflightLoad::new());
                    map.insert(key.clone(), CacheSlot::Loading(Arc::clone(&inflight)));
                    SlotAction::Load(inflight)
                }
            }
        };

        let inflight = match action {
            SlotAction::Hit(result) => return result,
            SlotAction::Wait(existing) => return Self::wait_inflight(existing),
            SlotAction::Load(inflight) => inflight,
        };

        let mut guard = InflightGuard {
            shard,
            key,
            inflight: Arc::clone(&inflight),
            settled: false,
        };
        match loader() {
            Ok(value) => {
                let arc: CachedValue = Arc::new(value.clone());
                {
                    let mut map = shard
                        .write()
                        .map_err(|_| String::from("asset cache write lock poisoned"))?;
                    map.insert(guard.key.clone(), CacheSlot::Ready(Arc::clone(&arc)));
                }
                guard.settled = true;
                inflight.finish(LoadOutcome::Ready(arc));
                Ok(value)
            }
            Err(error) => {
                if let Ok(mut map) = shard.write() {
                    map.remove(&guard.key);
                }
                guard.settled = true;
                inflight.finish(LoadOutcome::Failed(error.clone()));
                Err(error)
            }
        }
    }

    fn wait_inflight<T: 'static + Clone>(inflight: Arc<InflightLoad>) -> Result<T, String> {
        inflight.wait_clone::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn concurrent_same_key_misses_run_loader_once_and_hits_share_reads() {
        let cache = Arc::new(AssetCache::new());
        let loads = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(std::sync::Barrier::new(8));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let cache = Arc::clone(&cache);
                let loads = Arc::clone(&loads);
                let barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    barrier.wait();
                    let val = cache
                        .load_or_cache("mesh.glb", || {
                            loads.fetch_add(1, Ordering::SeqCst);
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            Ok(vec![1_u32, 2, 3, 4])
                        })
                        .unwrap();
                    assert_eq!(val, vec![1, 2, 3, 4]);
                });
            }
        });
        assert_eq!(loads.load(Ordering::SeqCst), 1);
        let cached = cache
            .load_or_cache::<Vec<u32>>("mesh.glb", || unreachable!("must hit"))
            .unwrap();
        assert_eq!(cached, vec![1, 2, 3, 4]);
    }

    #[test]
    fn failed_load_clears_slot_for_retry() {
        let cache = AssetCache::new();
        assert_eq!(
            cache.load_or_cache::<u32>("bad", || Err("boom".into())),
            Err(String::from("boom"))
        );
        assert_eq!(cache.load_or_cache::<u32>("bad", || Ok(42)), Ok(42));
    }
}
