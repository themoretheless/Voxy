# Associated-data reuse: local CPU probe

Command: `cargo bench -p voxy_scene --bench processor_requirements`. Apple M4 Max,
optimized bench profile, 3 warmups and 21 samples per workload. Raw 84 workload
summaries and observed source hashes are in the adjacent CSV/JSON. This run had
concurrent host activity and large tails (some tens of milliseconds); it cannot
establish stable tail latency, cross-engine superiority or a frame budget.

The existing fixture independently verifies checksums and eligible counts. Added
workloads exercise unchanged synchronization, revision-validated published scans,
and paired mutation-plus-synchronization of ceil(eligible/100) matching owners.
Both churn paths mutate the same number of owners; reused factory call counts are
asserted. Factories copy 64 bytes of integers; they do not construct GPU resources
or simulate expensive derived state. The checksum body does not access the source
components after retrieving a published row.

| 100k slots / eligible | Full rebuild with 1% churn median | Factory reuse with 1% churn median |
| --- | ---: | ---: |
| 100,000 | 3.056 ms | 4.825 ms |
| 10,000 | 2.224 ms | 2.770 ms |
| 50,000 (half inactive) | 2.727 ms | 2.543 ms |
| 5,000 (sparse, half inactive) | 3.574 ms | 2.778 ms |

These medians do not support a universal reuse speedup. ComponentTable full
rebuild remains suitable for cheap rows. AssociatedData provides resource identity
preservation, selective factory calls and stale-row suppression; retain it as an
explicit option for derived resources, not a mandatory fast path. Its membership
scan and replacement row allocations still happen at synchronization, even when
factories are skipped. Expensive factories, memory peaks, randomized workload
order and isolated repeat runs are needed before selecting automatic policies.

Code inspection after this run found duplicate owner/activity/table lookup in
AssociatedData::query: its dense iterator validated eligibility and then called
get, repeating those checks. The implementation now reads the already-borrowed row
and compares both current component revisions directly. The raw data above is
from before this change. Focused regression and a new optimized run are pending;
no measured improvement from the change is claimed yet.

The direct-row query rerun completed all 84 rows and benchmark assertions; raw
results are saved in `associated-data-direct-query-2026-10-02.csv` with source
hashes in its JSON. Background activity and differing cache states prevent a
causal before/after claim. For 100k slots / 5k matches, the validated scan median
was 0.152 ms; churn full rebuild was 2.080 ms and reuse was 2.118 ms. This repeat
reinforces that cheap-row reuse is not a universal win. Focused test and benchmark
Clippy completion are still pending.
