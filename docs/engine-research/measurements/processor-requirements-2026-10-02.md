# Typed requirements and associated-data CPU measurements

Run: `cargo bench -p voxy_scene --bench processor_requirements` on Apple M4 Max,
macOS 26.7.1, Rust 1.98.0-nightly (2026-05-26 commit), optimized bench profile.
Raw samples summaries are in `processor-requirements-2026-10-02.csv`; environment,
command and observed source hashes are in the adjacent JSON. The workspace was
dirty and other tasks were active on the host. This is one local run, not an
isolated cross-engine comparison or an end-to-end game-frame budget.

The fixture creates 1k/10k/100k independent root nodes. Every node owns a u64;
every first or tenth node owns the second required bool. Activity either includes
all nodes or disables alternating requirement groups, leaving half the matches
active. Independent expected sums and eligible row counts are checked before
timing. The three workloads are a direct active requirement checksum scan,
a complete transactional rebuild of 64-byte rows (including allocations and old
row destruction), and a checksum scan of published rows with live/activity checks.
The factory only copies integers; expensive resource construction is not modeled.
All use 3 warmup operations followed by 21 timed operations. Columns show median,
20th ordered sample and maximum; 21 samples do not establish population p99.
Timer overhead matters for tiny published-table cases.

| Slots | Eligible | Direct scan median | Rebuild median | Published scan median |
| ---: | ---: | ---: | ---: | ---: |
| 1,000 | 1,000 | 15.417 us | 22.500 us | 1.875 us |
| 10,000 | 10,000 | 156.458 us | 220.167 us | 18.792 us |
| 100,000 | 100,000 | 1.571 ms | 2.444 ms | 0.307 ms |
| 100,000 | 10,000 | 1.524 ms | 2.157 ms | 0.021 ms |
| 100,000 | 50,000 (half inactive) | 1.198 ms | 1.528 ms | 0.171 ms |
| 100,000 | 5,000 (sparse, half inactive) | 1.185 ms | 1.310 ms | 0.011 ms |

## Architectural consequence

A scan scales with scene slots even when few nodes meet the requirements. Keeping
published dense associated rows is useful for repeated dispatch. For the 100k/
10k case, two direct scans cost approximately 3.047 ms versus approximately
2.199 ms for one rebuild and two published scans. This arithmetic is a local
break-even estimate under unchanged membership AND component values, not a
measured two-frame workload or a claimed incremental-cache speedup.

Rebuilding on every single dispatch is slower than the direct scan in these
fixtures. Keep direct borrowed queries for infrequent work and publish associated
data at explicit changes/barriers. Before implementing reuse, cover both membership
and value changes: `component_mut` currently exposes unrestricted mutation and
has no per-type revision contract. A membership-only cache would miss replacement
or mutation of still-present requirements. Activity and generational reuse must
also invalidate eligibility. Do not cache solely by NodeId or component presence.

The next implementation needs an explicit component revision/change tracking
contract, including mutable borrow behavior, removal/reinsert, ancestor activity,
and scene ownership; then benchmark sparse churn against full rebuild with the
same resulting rows and lifecycle guarantees. This run does not choose an
incremental algorithm. Peak allocations, dependency-heavy factories, fragmentation,
hierarchy changes, churn and worker dispatch remain unmeasured.

Validation: the optimized benchmark completed all 36 workload rows and its
checksum/count assertions; focused strict Clippy and rustfmt checks passed.
