//! Solve-local reuse of complete, bit-identical capsule trajectory proofs.
use super::*;
use std::collections::HashMap;
use std::sync::Mutex;
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key([u64; 29]);
impl Key {
    fn new(
        a: CapsuleMotion,
        b: CapsuleMotion,
        trim_roots: bool,
        options: CapsuleSweepOptions,
    ) -> Self {
        let mut bits = [0; 29];
        for (slot, value) in bits.iter_mut().zip(
            a.start
                .into_iter()
                .chain(a.end)
                .chain(b.start)
                .chain(b.end)
                .flatten(),
        ) {
            *slot = value.to_bits();
        }
        bits[24] = a.radius.to_bits();
        bits[25] = b.radius.to_bits();
        bits[26] = u64::from(trim_roots);
        bits[27] = options.tolerance_m.to_bits();
        bits[28] = options.max_iterations as u64;
        Self(bits)
    }
}
struct Entry {
    key: Key,
    fraction: f64,
}
/// No global state, approximate keys, or new clearance certificates. A miss
/// executes the unchanged complete capsule solver; failures are never cached.
/// Bounds and candidate identities remain owned by the broad phase.
pub(super) struct FractionCache {
    shards: [Mutex<HashMap<(usize, usize), Entry>>; 16],
    #[cfg(any(test, feature = "experimental-strand-fraction-reuse"))]
    hits: std::sync::atomic::AtomicUsize,
    #[cfg(any(test, feature = "experimental-strand-fraction-reuse"))]
    misses: std::sync::atomic::AtomicUsize,
}
impl Default for FractionCache {
    fn default() -> Self {
        Self {
            shards: std::array::from_fn(|_| Mutex::new(HashMap::new())),
            #[cfg(any(test, feature = "experimental-strand-fraction-reuse"))]
            hits: Default::default(),
            #[cfg(any(test, feature = "experimental-strand-fraction-reuse"))]
            misses: Default::default(),
        }
    }
}
impl FractionCache {
    const PER_SHARD: usize = 512;
    #[cfg(any(test, feature = "experimental-strand-fraction-reuse"))]
    pub(super) fn counts(&self) -> (usize, usize) {
        use std::sync::atomic::Ordering::Relaxed;
        (self.hits.load(Relaxed), self.misses.load(Relaxed))
    }
    fn shard_index(pair: (usize, usize)) -> usize {
        (pair.0.wrapping_mul(31) ^ pair.1) % 16
    }
    /// Each shard is owned by one task for this query. Reassemble by original
    /// pair index before exposing any result or error, regardless of scheduling.
    pub(super) fn query_ordered<R: Send>(
        &self,
        pairs: &[(usize, usize)],
        workers: usize,
        evaluate: impl Fn(&(usize, usize), &mut FractionShard<'_>) -> Result<Option<R>, &'static str>
        + Sync,
    ) -> Result<Vec<R>, &'static str> {
        let mut buckets: [Vec<_>; 16] = std::array::from_fn(|_| Vec::new());
        for (index, &pair) in pairs.iter().enumerate() {
            buckets[Self::shard_index(pair)].push(index);
        }
        let ids: Vec<_> = (0..16).filter(|&i| !buckets[i].is_empty()).collect();
        let batches = crate::hair::ordered_queries::ordered_query_results(&ids, workers, |&id| {
            let mut shard = FractionShard {
                owner: self,
                cache: self.shards[id].lock().ok(),
            };
            // Clear/ignored pairs have no output. Keep only actual restrictions
            // and errors; sorting every clear pair dominated large cached queries.
            let results = buckets[id]
                .iter()
                .filter_map(|&index| match evaluate(&pairs[index], &mut shard) {
                    Ok(None) => None,
                    result => Some((index, result)),
                })
                .collect::<Vec<_>>();
            Ok(Some(results))
        })?;
        let mut results = batches.into_iter().flatten().collect::<Vec<_>>();
        results.sort_unstable_by_key(|(index, _)| *index);
        results
            .into_iter()
            .filter_map(|(_, result)| result.transpose())
            .collect()
    }
    #[cfg(test)]
    fn evaluate(
        &self,
        pair: (usize, usize),
        a: CapsuleMotion,
        b: CapsuleMotion,
        trim_roots: bool,
        options: CapsuleSweepOptions,
        compute: impl FnOnce() -> Result<f64, &'static str>,
    ) -> Result<f64, &'static str> {
        FractionShard {
            owner: self,
            cache: self.shards[Self::shard_index(pair)].lock().ok(),
        }
        .evaluate(pair, a, b, trim_roots, options, compute)
    }
}
/// A query task holds one shard, so other tasks never contend for this lock.
/// Poisoned storage bypasses reuse and still executes the original solver.
pub(super) struct FractionShard<'a> {
    #[allow(
        dead_code,
        reason = "keeps the owning cache alive for the shard's guard"
    )]
    owner: &'a FractionCache,
    cache: Option<std::sync::MutexGuard<'a, HashMap<(usize, usize), Entry>>>,
}
impl FractionShard<'_> {
    pub(super) fn evaluate(
        &mut self,
        pair: (usize, usize),
        a: CapsuleMotion,
        b: CapsuleMotion,
        trim_roots: bool,
        options: CapsuleSweepOptions,
        compute: impl FnOnce() -> Result<f64, &'static str>,
    ) -> Result<f64, &'static str> {
        let key = Key::new(a, b, trim_roots, options);
        if let Some(entry) = self
            .cache
            .as_ref()
            .and_then(|cache| cache.get(&pair))
            .filter(|entry| entry.key == key)
        {
            #[cfg(any(test, feature = "experimental-strand-fraction-reuse"))]
            self.owner
                .hits
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Ok(entry.fraction);
        }
        #[cfg(any(test, feature = "experimental-strand-fraction-reuse"))]
        self.owner
            .misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let fraction = compute()?;
        if fraction.is_finite() && fraction > 0. {
            if let Some(cache) = self.cache.as_mut() {
                if cache.contains_key(&pair) || cache.len() < FractionCache::PER_SHARD {
                    cache.insert(pair, Entry { key, fraction });
                }
            }
        }
        Ok(fraction)
    }
}
#[test]
fn exact_fraction_cache_invalidates_every_coordinate_radius_and_root_rule() {
    let a = CapsuleMotion {
        start: [[0., 1., 2.], [3., 4., 5.]],
        end: [[6., 7., 8.], [9., 10., 11.]],
        radius: 0.01,
    };
    let b = CapsuleMotion {
        start: [[12., 13., 14.], [15., 16., 17.]],
        end: [[18., 19., 20.], [21., 22., 23.]],
        radius: 0.02,
    };
    let cache = FractionCache::default();
    let options = CapsuleSweepOptions::default();
    assert_eq!(
        cache
            .evaluate((0, 1), a, b, false, options, || Ok(0.75))
            .unwrap(),
        0.75
    );
    assert_eq!(
        cache
            .evaluate((0, 1), a, b, false, options, || panic!(
                "identical proof was recomputed"
            ))
            .unwrap(),
        0.75
    );
    for slot in 0..29 {
        let (mut ma, mut mb, mut changed_options) = (a, b, options);
        let mutate = |motion: &mut CapsuleMotion, index: usize| {
            let value = if index < 6 {
                &mut motion.start[index / 3][index % 3]
            } else {
                &mut motion.end[(index - 6) / 3][index % 3]
            };
            *value = f64::from_bits(value.to_bits() ^ 1);
        };
        match slot {
            0..=11 => mutate(&mut ma, slot),
            12..=23 => mutate(&mut mb, slot - 12),
            24 => ma.radius = f64::from_bits(ma.radius.to_bits() ^ 1),
            25 => mb.radius = f64::from_bits(mb.radius.to_bits() ^ 1),
            26 => {}
            27 => changed_options.tolerance_m = f64::from_bits(options.tolerance_m.to_bits() ^ 1),
            28 => changed_options.max_iterations += 1,
            _ => unreachable!(),
        }
        cache
            .evaluate((0, 1), a, b, false, options, || Ok(0.75))
            .unwrap();
        assert_eq!(
            cache
                .evaluate((0, 1), ma, mb, slot == 26, changed_options, || Ok(0.5))
                .unwrap(),
            0.5,
            "key word {slot} escaped invalidation"
        );
    }
    let mut changed = a;
    changed.start[0][0] = -0.;
    assert_eq!(
        cache
            .evaluate((0, 1), changed, b, false, options, || Ok(0.5))
            .unwrap(),
        0.5
    );
    assert_eq!(
        cache
            .evaluate((0, 1), a, b, true, options, || Ok(0.25))
            .unwrap(),
        0.25
    );
    assert_eq!(
        cache.evaluate((0, 1), a, b, false, options, || Err("original rejection")),
        Err("original rejection")
    );
    assert_eq!(
        cache
            .evaluate((0, 1), a, b, false, options, || Ok(0.625))
            .unwrap(),
        0.625
    );
    assert_eq!(
        cache.evaluate((2, 3), a, b, false, options, || Err("original rejection")),
        Err("original rejection")
    );
    assert_eq!(
        cache
            .evaluate((2, 3), a, b, false, options, || Ok(1.))
            .unwrap(),
        1.
    );
    assert_eq!(
        cache
            .evaluate((2, 3), a, b, false, options, || panic!(
                "clear trajectory was recomputed"
            ))
            .unwrap(),
        1.
    );
}
#[test]
fn exact_fraction_cache_capacity_never_drops_a_query() {
    let motion = CapsuleMotion {
        start: [[0.; 3], [1.; 3]],
        end: [[0.; 3], [1.; 3]],
        radius: 0.01,
    };
    let cache = FractionCache::default();
    let options = CapsuleSweepOptions::default();
    for id in 0..20_000 {
        assert_eq!(
            cache
                .evaluate((id, id + 1), motion, motion, false, options, || Ok(0.5))
                .unwrap(),
            0.5
        );
    }
    assert!(
        cache
            .shards
            .iter()
            .map(|s| s.lock().unwrap().len())
            .sum::<usize>()
            <= 16 * FractionCache::PER_SHARD
    );
    assert_eq!(
        cache.evaluate((20_001, 20_002), motion, motion, false, options, || Err(
            "uncached rejection"
        )),
        Err("uncached rejection")
    );
}

#[test]
fn exact_fraction_cache_shard_batches_preserve_order_errors_and_poison_bypass() {
    let motion = CapsuleMotion {
        start: [[0.; 3], [1.; 3]],
        end: [[0.; 3], [1.; 3]],
        radius: 0.01,
    };
    let options = CapsuleSweepOptions::default();
    let pairs = [(1, 16), (1, 31), (1, 30), (1, 29)];
    for workers in [1, 2, 4, 8] {
        let cache = FractionCache::default();
        for repeat in 0..2 {
            let actual = cache
                .query_ordered(&pairs, workers, |&pair, shard| {
                    let value = shard.evaluate(pair, motion, motion, false, options, || {
                        assert_eq!(repeat, 0, "identical query missed its cache");
                        Ok(pair.1 as f64 / 32.)
                    })?;
                    Ok(Some((pair, value)))
                })
                .unwrap();
            assert_eq!(
                actual,
                pairs
                    .iter()
                    .map(|&pair| (pair, pair.1 as f64 / 32.))
                    .collect::<Vec<_>>()
            );
        }
        let sparse = cache
            .query_ordered(&pairs, workers, |&pair, _| {
                Ok((pair == pairs[0] || pair == pairs[3]).then_some(pair))
            })
            .unwrap();
        assert_eq!(sparse, vec![pairs[0], pairs[3]]);
        // The earliest original pair belongs to a later shard. Worker/shard
        // traversal must never replace the serial first-error contract.
        let error = cache.query_ordered::<()>(&pairs, workers, |&pair, _| {
            if pair == pairs[0] {
                Err("first original error")
            } else {
                Err("later original error")
            }
        });
        assert_eq!(error, Err("first original error"));
        let shard = FractionCache::shard_index(pairs[0]);
        let _ = std::panic::catch_unwind(|| {
            let _guard = cache.shards[shard].lock().unwrap();
            panic!("injected poison");
        });
        let bypass = cache
            .query_ordered(&pairs[..1], workers, |&pair, shard| {
                Ok(Some(shard.evaluate(
                    pair,
                    motion,
                    motion,
                    false,
                    options,
                    || Ok(0.125),
                )?))
            })
            .unwrap();
        assert_eq!(
            bypass,
            vec![0.125],
            "poisoned reuse must execute the original query"
        );
    }
}
