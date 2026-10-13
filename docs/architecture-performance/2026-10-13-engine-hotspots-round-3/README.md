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

Results (7 rounds for the pipeline, 5 for the queries; all checksums, hit counts and
quad counts identical before and after):

| workload | before p50 | after p50 | ratio |
| --- | ---: | ---: | ---: |
| voxel_pipeline palette_to_dense | 58.2 µs | 47.1 µs | 0.81 |
| voxel_pipeline palette_with_updates_new_palette_entry | 121.2 µs | 112.3 µs | 0.93 |
| voxel_pipeline mesh_all_resident (9 chunks) | 2.90 ms | 2.95 ms | 1.02 |
| voxel_pipeline light_all_resident (9 chunks) | 3.31 ms | 3.19 ms | 0.96 |
| voxel_pipeline rebuild_all_resident (9 chunks) | 6.68 ms | 6.41 ms | 0.96 |
| voxel_queries raycast x10000 | 9.51 ms | 7.86 ms | 0.83 |
| voxel_queries sweep_aabb x10000 | 2.27 ms | 2.11 ms | 0.93 |
| voxel_queries water_tick | 3.27 ms | 3.28 ms | 1.00 |

The sub-15 µs palette and commit rows move both ways between runs on this host and
are not claimed. An earlier five-round pass showed `light_all_resident` at 0.60; the
seven-round pass above is the one reported, so the lighting gain is the modest 0.96,
most of it the cheaper palette expansion.

## Behavior path (`cargo bench -p voxy_gameplay --bench angular_motion`)

The behavior path ran 3.5x slower than the batch path for the same per-object work.
Each `set_local` refreshed its subtree through a heap-allocated traversal stack and
rebuilt the local matrix it had just validated. Childless nodes (the per-behavior
case) now refresh without the stack, and the validated matrix is reused for the root.

The per-object behavior path itself is unchanged within noise (`angular_motion`
behavior 0.95 at 1k objects, 1.00 at 10k and 100k; batch 0.99 to 1.05): its cost is
the eight validated handle lookups per object, not the refresh. The leaf refresh does
show in the scene graph's own edit workloads (`scene_scaling`, 5 rounds):

| workload | before p50 | after p50 | ratio |
| --- | ---: | ---: | ---: |
| 1000 one_percent_leaf_edits | 563 ns | 338 ns | 0.60 |
| 10000 one_percent_leaf_edits | 5.3 µs | 3.1 µs | 0.59 |
| 100000 one_percent_leaf_edits | 52.2 µs | 29.9 µs | 0.57 |
| 1000 / 10000 / 100000 wide_single_leaf_batch | 129 / 95 / 98 ns | 92 / 75 / 75 ns | 0.71 / 0.79 / 0.77 |
| 1000 / 10000 / 100000 chain_single_leaf_batch | 95 / 93 / 95 ns | 74 / 75 / 77 ns | 0.78 / 0.81 / 0.81 |
| 1000 all_node_edits | 444.7 µs | 293.6 µs | 0.66 |
| 100000 all_node_edits | 91.49 ms | 84.39 ms | 0.92 |

Projection, root edits and the batch paths are unchanged within noise.

## Surface-film sphere contact (`cargo bench -p physics --bench sphere_contact`)

`entries` returned a `Vec` of zero to two roots for every vertex and edge test, six
allocations per triangle per query (about 15% of the benchmark's instructions were in
the allocator). A two-slot array holds the same roots in the same order.

| workload | before p50 | after p50 | ratio |
| --- | ---: | ---: | ---: |
| sphere_contact, 1000 queries (7 rounds) | 830.0 µs | 749.5 µs | 0.90 |

Checksum identical (35559.18119745800504461).

## Gates

`cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`
were both red on `main` after the upstream merge (nine lint errors in the hair
solver, 52 unformatted files, and lints in other merged crates); this branch restores
both. Tests after the changes: voxy_core 10, voxy_world 22, voxy_lighting 5 (the
reference-oracle test included), voxy_mesher 8, physics_voxel 43, voxy_runtime 16,
voxy_scene 104, physics surface-film suites 146 across 19 binaries, all passed.
