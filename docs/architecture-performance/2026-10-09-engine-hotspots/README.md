# Engine hot-path optimizations (2026-10-09)

Paired before/after CPU measurements for the changes in this increment. `before`
binaries were built from the baseline commit in a separate worktree, `after` from
this change; the two binaries alternated per round on the same machine
(`report.json` has the raw samples, medians of per-run p50).

## Voxel edit -> rebuild pipeline (`cargo bench -p voxy_runtime --bench voxel_pipeline`)

| workload | before p50 | after p50 | ratio |
| --- | ---: | ---: | ---: |
| palette_from_dense | 300 µs | 66 µs | 0.22 |
| palette_with_updates_1 | 403 µs | 8.7 µs | 0.02 |
| world_commit_1_write | 388 µs | 1.0 µs | 0.003 |
| world_commit_64_writes | 372 µs | 5.2 µs | 0.01 |
| mesh_all_resident (9 chunks) | 3.62 ms | 2.84 ms | 0.79 |
| light_all_resident (9 chunks) | 5.36 ms | 3.32 ms | 0.62 |
| rebuild_all_resident (9 chunks) | 9.52 ms | 6.76 ms | 0.71 |

What changed: `PalettedBlocks::from_dense` builds the sorted palette with a small
vector instead of two B-trees; `with_updates` rewrites packed indices in place when
the canonical palette is unchanged (last-occurrence removal and new blocks still take
the dense path, so the representation stays canonical); `World::commit` groups the
sorted writes per chunk instead of filtering the whole list per chunk; chunk
validation checks the palette instead of expanding the chunk; `build_light` classifies
each distinct block once, fills the halo per face directly, and seeds propagation with
frontier cells only (budget accounting and output are identical, pinned by a test that
keeps the previous implementation as an oracle); `build_mesh` classifies per palette
slot, decides uniform x-rows at once, resolves the halo neighbor once per boundary
slice and skips interior slices of single-block chunks.

On top of that the desktop app now rebuilds only chunks a commit can affect
(`voxy_runtime::invalidated_derived_chunks`: the edited chunks, their six face
neighbors for the light halo and the receipt's invalidated mesh neighbors) and merges
them into the cached derived set, instead of re-lighting and re-meshing every resident
chunk after every water tick or explosion. A test proves the merged set equals a full
rebuild.

## Voxel physics queries (`cargo bench -p physics_voxel --bench voxel_queries`)

| workload | before p50 | after p50 | ratio |
| --- | ---: | ---: | ---: |
| sweep_aabb x10000 | 2.74 ms | 2.17 ms | 0.79 |
| raycast x10000 (128 steps max) | 11.2 ms | 8.3 ms | 0.74 |
| water_tick (3040 active cells) | 4.30 ms | 4.13 ms | 0.96 (unchanged code) |

Rays and sweeps resolve the chunk once per run of cells (`ChunkCursor`) instead of a
map lookup per voxel; results and checksums are identical.

## Scene graph and gameplay

`processor_requirements` (`voxy_scene`): 1.2x to 2.7x faster on every active-component
scan / rebuild workload at 1k-100k objects (component maps use a deterministic
hasher, `components::<T>()` iterates slots directly, extraction overwrites staged
instances in place). `scene_scaling`: projection 1.3x-2.9x faster, component updates
1.2x-1.6x faster; dense all-node batches unchanged within noise after keeping the
linear scratch path for dense batches (sparse batches use sorted edits).
`angular_motion` (`voxy_gameplay`): batch path 1.4x-1.8x faster, behavior path 5%
faster (`support_max` uses a fixed array instead of a heap vector per evaluation).
Transform-only atomic batches no longer shadow-copy the whole graph.

## Renderer and app loop (CPU parts)

`smooth_normals` welds with a deterministic hasher (same output, checked against an
ordered-map reference); `update_mesh` compares indices once instead of three times;
`Renderer::update_skinned_pose` validates and uploads the animated actor once per
fixed tick instead of twice; demo window titles and the rush diagnostic panel are only
rebuilt when their inputs change; the water active list is deduplicated by sort instead
of a per-tick B-tree. The dead `cfg(feature = "nightly")` SIMD module in
`physics::biomechanics` (feature never declared, could not compile under
`unsafe_code = "forbid"`) was removed, and the absolute `include_str!` path in
`female_rig.rs` that broke `cargo test -p voxy_app` off the author's machine was made
relative.

## Limitations

See `report.json`: shared 4-vCPU host, no GPU (render changes measured on CPU parts),
1000-object `scene_scaling` rows were noisy in both directions, water planner untouched.
