// Independent geometry queries borrow immutable motion. Join in input order;
// the first rejected pair and every accepted fraction retain serial semantics.
pub(super) fn ordered_query_results<T: Sync, R: Send>(
    pairs: &[T],
    workers: usize,
    evaluate: impl Fn(&T) -> Result<Option<R>, &'static str> + Sync,
) -> Result<Vec<R>, &'static str> {
    if workers <= 1 || pairs.len() < 2 {
        return pairs
            .iter()
            .filter_map(|pair| evaluate(pair).transpose())
            .collect();
    }
    #[cfg(feature = "experimental-reusable-query-workers")]
    if rayon::current_thread_index().is_none() {
        // Experimental per-calling-thread owner; no implicit global Rayon pool.
        // Creation failure retains the existing scoped execution path.
        thread_local! {
            static POOL:std::cell::OnceCell<Option<rayon::ThreadPool>>=const {std::cell::OnceCell::new()};
        }
        let result = POOL.with(|slot| {
            let pool = slot
                .get_or_init(|| {
                    rayon::ThreadPoolBuilder::new()
                        .num_threads(
                            std::thread::available_parallelism()
                                .map_or(1, usize::from)
                                .min(8),
                        )
                        .thread_name(|i| format!("hair-query-{i}"))
                        .build()
                        .ok()
                })
                .as_ref()?;
            use rayon::prelude::*;
            let chunk = pairs
                .len()
                .div_ceil(workers.min(pool.current_num_threads()).min(pairs.len()));
            let batches: Vec<Result<Vec<R>, &'static str>> = pool.install(|| {
                pairs
                    .par_chunks(chunk)
                    .map(|batch| {
                        batch
                            .iter()
                            .filter_map(|pair| evaluate(pair).transpose())
                            .collect()
                    })
                    .collect()
            });
            Some(
                batches
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .map(|batches| batches.into_iter().flatten().collect()),
            )
        });
        if let Some(result) = result {
            return result;
        }
    }
    scoped_query_results(pairs, workers, evaluate)
}
fn scoped_query_results<T: Sync, R: Send>(
    pairs: &[T],
    workers: usize,
    evaluate: impl Fn(&T) -> Result<Option<R>, &'static str> + Sync,
) -> Result<Vec<R>, &'static str> {
    if pairs.is_empty() {
        return Ok(Vec::new());
    }
    let chunk = pairs.len().div_ceil(workers.min(pairs.len()));
    let batches = std::thread::scope(|scope| {
        let evaluate = &evaluate;
        let handles: Vec<_> = pairs
            .chunks(chunk)
            .map(|batch| {
                scope.spawn(move || {
                    batch
                        .iter()
                        .filter_map(|pair| evaluate(pair).transpose())
                        .collect::<Result<Vec<_>, _>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("geometry query worker panicked"))
            .collect::<Vec<_>>()
    });
    let mut limits = Vec::new();
    for batch in batches {
        limits.extend(batch?);
    }
    Ok(limits)
}

#[cfg(all(test, feature = "experimental-reusable-query-workers"))]
mod tests {
    use super::*;
    #[test]
    #[ignore = "host scheduling microbenchmark; not a physical trajectory or FPS gate"]
    fn benchmark_reusable_query_worker_scheduling() {
        let pairs: Vec<_> = (0..4096usize).collect();
        let evaluate = |&id: &usize| Ok((id % 3 == 0).then_some((id, (id as f64 + 1.) / 4097.)));
        let reference = ordered_query_results(&pairs, 1, evaluate).unwrap();
        for reusable in [false, true, true, false] {
            let began = std::time::Instant::now();
            for _ in 0..128 {
                let result = if reusable {
                    ordered_query_results(&pairs, 8, evaluate)
                } else {
                    scoped_query_results(&pairs, 8, evaluate)
                }
                .unwrap();
                assert_eq!(result, reference);
                std::hint::black_box(result);
            }
            eprintln!(
                "QUERY WORKER BENCH reusable={reusable} calls=128 ordered_results_equal=true elapsed_ms={} scope=scheduling_microbenchmark_not_physics_or_fps",
                began.elapsed().as_secs_f64() * 1000.
            );
        }
    }
    #[test]
    fn reusable_workers_keep_borrowed_inputs_and_bounded_thread_identities() {
        let pairs: Vec<_> = (0..1024).collect();
        let mut identities = std::collections::HashSet::new();
        let borrowed_offset = &17;
        for _ in 0..24 {
            let results = ordered_query_results(&pairs, 8, |value| {
                Ok(Some((
                    *value + borrowed_offset,
                    std::thread::current().id(),
                )))
            })
            .unwrap();
            assert_eq!(
                results
                    .iter()
                    .map(|(value, _)| *value - borrowed_offset)
                    .collect::<Vec<_>>(),
                pairs
            );
            identities.extend(results.into_iter().map(|(_, thread)| thread));
        }
        assert!(
            identities.len() <= 8,
            "queries created fresh workers: {}",
            identities.len()
        );
    }
}
