# Engine hot-path optimizations, round 3 (2026-10-13)

Paired before/after CPU measurements. `before` binaries are the benchmark executables
built from `main` at `be884bf7` (saved before any change), `after` from this branch;
the two binaries alternate per round on the same machine, five rounds, median of each
benchmark's own per-run p50/median. Callgrind profiles of the pipeline and query
benches (`callgrind_annotate --inclusive`) chose the targets.

## Scene tables (`cargo bench -p voxy_scene --bench processor_requirements`)

The reuse path of `AssociatedData::synchronize` cost more than rebuilding every row
from scratch: `active_components_with` re-validated the owner and looked the second
component up again per node, and the reuse check then fetched both component
revisions with two more node lookups per row. One slot scan
(`active_components_with_revisions`) now yields both components and both revisions;
the table rebuild passes them to the factory and presizes the staged table; `query`
and `get` resolve both revisions with one handle validation.

| workload (slots, stride, inactive stride, eligible) | before p50 | after p50 | ratio |
| --- | ---: | ---: | ---: |
| 1000,1,0,1000 reuse_unchanged_64byte_rows | 64.8 µs | 58.6 µs | 0.90 |
| 1000,1,0,1000 one_percent_churn_factory_reuse | 66.7 µs | 60.2 µs | 0.90 |
| 10000,1,0,10000 transactional_rebuild_64byte_rows | 477.5 µs | 446.2 µs | 0.93 |
| 10000,1,0,10000 reuse_unchanged_64byte_rows | 813.6 µs | 680.4 µs | 0.84 |
| 10000,1,0,10000 one_percent_churn_full_rebuild | 623.4 µs | 449.2 µs | 0.72 |
| 10000,1,0,10000 one_percent_churn_factory_reuse | 807.7 µs | 693.8 µs | 0.86 |
| 100000,1,0,100000 transactional_rebuild_64byte_rows | 5.53 ms | 4.83 ms | 0.87 |
| 100000,1,0,100000 one_percent_churn_full_rebuild | 5.74 ms | 4.80 ms | 0.84 |
| 100000,1,0,100000 one_percent_churn_factory_reuse | 8.80 ms | 7.08 ms | 0.80 |

Scans (`active_requirement_scan`, `published_table_scan`, `validated_associated_scan`)
are unchanged within noise. The remaining gap between reuse and rebuild is the `Arc`
clone and release per reused row, which the by-reference factory contract requires.

## Voxel pipeline and voxel queries

Profile (callgrind, `voxel_pipeline`): `build_light` 55% of instructions, of which
`propagate` 25% and `PalettedBlocks::palette_indices` 16% (called once per chunk by
both the lighting fill and the mesher); `build_mesh` 39%. Profile (`voxel_queries`):
`raycast` 64%, of which `ChunkCursor::sample` 48%, `PalettedBlocks::get` 13% and
`split_voxel` 8%.

- `split_voxel`: the chunk edge is a power of two, so the Euclidean quotient and
  remainder are an arithmetic shift and a mask (identical for negative coordinates;
  the const assertion pins the precondition).
- `PalettedBlocks::palette_indices`: widths that divide a 64-bit word (1, 2, 4, 8, 16)
  never straddle two words, so they unpack with a plain shift loop per word; the
  generic path casts the slot directly instead of `u16::try_from` per slot.
- `propagate`: the neighbor's level is tested before its opaque flag, so the flag is
  only read when a raise is possible (same order, same writes, same budget count).

RESULTS_PIPELINE

## Behavior path (`cargo bench -p voxy_gameplay --bench angular_motion`)

The behavior path ran 3.5x slower than the batch path for the same per-object work.
Each `set_local` refreshed its subtree through a heap-allocated traversal stack and
rebuilt the local matrix it had just validated. Childless nodes (the per-behavior
case) now refresh without the stack, and the validated matrix is reused for the root.

RESULTS_BEHAVIOR

## Gates

RESULTS_GATES
