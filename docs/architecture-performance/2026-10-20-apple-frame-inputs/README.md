# Apple Silicon camera, instance and particle uploads — 2026-10-20

## Implemented owners and payloads

| Owner | Shared input | Bytes |
|---|---|---:|
| Renderer | ordinary voxel camera | 64 |
| Renderer | photoreal/bodycam camera | 320 |
| NativeVegetation | camera | 240 |
| NativeVegetation | instance vertex stream | 112 per instance |
| ScreenSpaceFluidRenderer | camera | 288 |
| ScreenSpaceFluidRenderer | particle vertex stream | 32 per particle |
| ScreenSpaceFluidRenderer | triangular film stream | 64 per cell |

These owners select three primary MAP_WRITE buffers per input on macOS/aarch64
Metal with MAPPABLE_PRIMARY_BUFFERS enabled. Ordinary surface/device constructors
already enable that feature when supported. Caller-supplied devices must enable it.
Allocation/admission failure and unsupported devices keep their legacy uploads.

The direct path writes the final payload into StorageModeShared buffers and binds
those same buffers as uniform/vertex inputs. There is no queue.write_buffer,
intermediate upload buffer or upload blit on that path. This uses the existing
vendored Metal backend's coherent MTLBuffer.contents mapping; no HAL modification
or bytesNoCopy wrapping of arbitrary Vec allocations was needed.

**CPU copies remain in the compatibility slice APIs.** update_instances and fluid
update methods retain an owned CPU snapshot, then write it into the shared primary
buffer at encoding. CPU snapshots preserve ownership, complete validation before
publication and repeat-encode semantics. The paired measurements include those
copies. This patch removes staging/GPU transfer copies, not every CPU memory copy
or DRAM transaction. Camera values are generated/selected at encoding time.

Coverage is the owners listed above. ArchiveSceneRenderer's separate GI/ray/source-
capture camera and visibility owners, per-object SceneTransform uniforms, ocean
simulation and other GPU-resident simulation buffers are outside this patch.

## Synchronization and retained bindings

Each input uses the existing SharedUploadRing fence protocol. CPU writes finish
and mapped views are dropped before unmap. Remapping is attached to the complete
command buffer with map_buffer_on_submit; a slot becomes reusable only after that
submission's GPU consumers finish. Input preparation can precede several render
passes in the same encoder. The returned buffers never escape to another submission.
Only nonblocking Poll is used by production; the waits in tests/benchmarks are not
part of the renderer.

A full ring stages a unique immutable snapshot and records its copy before its
consumers in the same encoder. This overflow path still copies, but cannot overwrite
an earlier unsubmitted snapshot. Discarded encoders/command buffers reclaim slots
through the existing cancellation handler. Empty vertex streams do not lease a slot.

Three camera binding sets are cached per owner; no per-frame group creation is
needed. Voxel/photoreal groups are rebuilt when resident metadata/light buffers
change. Vegetation's shadow, receiver and colour passes use one selected camera
and the selected instance streams. Fluid depth, thickness, bilateral filter and
composition passes use the selected camera and streams. Diagnostics lease their
own camera input for repeat use after submission.

The standalone create_compute_filter API returns an externally retained bind group
that binds the legacy camera. encode_compute_filter therefore records a unique
camera staging copy immediately before this optional consumer. That compatibility
compute-filter camera path is **not zero-copy**; its numeric output is tested against
legacy. Normal fluid rendering and diagnostics use shared camera bindings.

Rings participate in ComputeMemoryBudget. The fluid allocation_bytes value now
includes admitted ring bytes. required_allocation_bytes reports the minimum portable
footprint; new_with_adapter_budget admits all optional ring bytes only if they fit
its explicit remaining budget, otherwise retaining the portable footprint. Overflow
staging remains outside the existing resident-budget contract.

## Physical validation

Apple M4 Max / Metal, macOS 26.7.2 (25G325), rustc 1.98.0-nightly
(d1fc603d1 2026-05-26), release profile. Base integration HEAD:
a8c57d98b0533da2dbc723ab7821ac49ac396831, with these changes uncommitted.
M1/M2/M3 use the same capability-gated path but were not physically tested.

Three new physical GPU tests passed with clean validation scopes:

- Voxel and photoreal camera helpers: every uniform word read by an independent
  Metal compute shader matches the CPU reference, including three shared leases,
  three overflow snapshots and replacement of cached metadata bindings while older
  encoders remain unsubmitted.
- Vegetation input helpers: a vertex probe consumes every camera row and every
  instance attribute through the actual cached camera group and vertex streams.
  Full RGBA8 images match legacy bit for bit across six unsubmitted snapshots,
  overflow, shrinking/empty lists and rejected nonfinite transforms. Images differ
  across valid frames, so a blank/unchanging target cannot satisfy the test. This
  checks the actual input preparation/ABI; it is not a full shadow/PBR scene capture.
- Full screen-space fluid rendering: exact RGBA8 parity across six unsubmitted
  snapshots, camera changes, particle/film changes, shrinking/empty lists and
  rejected over-capacity updates. Tight explicit budgets preserve the legacy
  footprint. Repeated diagnostics validate cleanly, and all standalone compute
  filter output bytes match legacy after the compatibility camera staging copy.

Existing ring/rigid/shared-mesh GPU regressions passed (3 tests); fluid input/shader
regressions passed (6 tests, 5 GPU/manual tests ignored by that default filter).
The portable footprint test and Noop fallback passed; the older physical allocation
resize test remained ignored, while the new physical fluid test checked a tight
budget with primary mapping enabled. Renderer Clippy and shell/demo consumer checks
passed. Adjacent logs retain the complete results. Clippy retains existing warnings outside the changed
production files; this is not a warning-free workspace claim. No full workspace
suite, M1/M2/M3 hardware run, or full-window/FPS qualification was performed.

## Same-process alternating measurements

Two series, each nine AB/BA pairs per case, after warming both paths. Each half-pair
has 24 frames; the harness drains after every three-frame batch. Production and
legacy share generation, validation and GPU consumer workloads. CPU timing includes
input preparation, encoding and queue submission. Drained time also includes GPU
completion waits. No noisy samples or regressions were removed.

Each case isolates the changed input: vegetation instance pairs use the legacy
camera; vegetation camera pairs keep instances fixed; fluid particle/film pairs use
the legacy camera; fluid camera pairs have empty particle/film lists. Main renderer
camera probes use actual generation and cached binding selection, with an independent
compute consumer of every word. Vegetation uses the vertex probe rather than its
4096x4096 shadow pass. Fluid pairs render the complete 64x16 fluid pipeline, not a
production-resolution scene. Existing slice APIs' CPU snapshot cost is included.

The CPU columns are medians of half-pairs in microseconds/frame. Paired ratio is
the median of nine shared/legacy ratios, rather than a ratio of column medians.
Camera count 64/320 is its byte size; vegetation camera count 1 is a fixed instance;
fluid camera count 0 denotes empty streams. Other counts are instance/particle/cell
counts.

| Path | Count | Series | Legacy CPU µs | Shared CPU µs | Paired ratio |
|---|---:|---:|---:|---:|---:|
| fluid_camera | 0 | 1 | 182.847 | 164.231 | 0.924 |
| fluid_films | 1024 | 1 | 217.434 | 206.083 | 0.832 |
| fluid_films | 64 | 1 | 186.675 | 167.406 | 1.013 |
| fluid_particles | 4096 | 1 | 240.238 | 190.073 | 0.776 |
| fluid_particles | 64 | 1 | 171.861 | 165.644 | 0.946 |
| photoreal_camera | 320 | 1 | 132.436 | 76.182 | 0.623 |
| vegetation_camera | 1 | 1 | 25.813 | 18.061 | 0.691 |
| vegetation_instances | 4096 | 1 | 177.814 | 121.219 | 0.695 |
| vegetation_instances | 64 | 1 | 34.531 | 32.090 | 0.959 |
| voxel_camera | 64 | 1 | 125.818 | 63.165 | 0.601 |
| fluid_camera | 0 | 2 | 632.281 | 584.616 | 0.868 |
| fluid_films | 1024 | 2 | 270.497 | 232.854 | 0.862 |
| fluid_films | 64 | 2 | 252.684 | 251.927 | 0.983 |
| fluid_particles | 4096 | 2 | 363.670 | 294.212 | 0.806 |
| fluid_particles | 64 | 2 | 190.160 | 178.427 | 0.922 |
| photoreal_camera | 320 | 2 | 67.847 | 42.543 | 0.559 |
| vegetation_camera | 1 | 2 | 47.003 | 31.337 | 0.696 |
| vegetation_instances | 4096 | 2 | 230.132 | 158.108 | 0.703 |
| vegetation_instances | 64 | 2 | 52.300 | 42.392 | 0.815 |
| voxel_camera | 64 | 2 | 61.960 | 32.944 | 0.534 |

Both series support lower median CPU time for the requested paths. At 4096 elements,
instances improved about 30% and particles 19–22%. The four camera cases improved
8–47%, depending on the consumer. Small lists are noisy: 64 instances improved only
4% in one series, and 64 films have ratios 1.013 / 0.983, establishing no speedup.
The film path is retained for coherent camera/particle/film snapshots; its tiny-list
performance is unqualified. Individual CPU regressions remain in the raw pairs.

**No GPU/frame/FPS speedup is established.** Other system jobs were active, absolute
CPU medians differ substantially between series, and drained results contain large
outliers. Every drained measurement and paired min/max remains in summary.json and
the raw logs. No unrelated process was stopped; only this task's suspended check
processes were resumed to finish verification.

## Reproduce

```sh
cargo test --release -p voxy_render --lib gpu_frame_ -- --ignored --nocapture --test-threads=1
cargo test --release -p voxy_render --lib fluid -- --test-threads=1
cargo test --release -p voxy_render --lib gpu_shared_ -- --ignored --nocapture --test-threads=1
cargo test --release -p voxy_render --lib allocation_tests -- --test-threads=1
cargo test --release -p voxy_render --lib unsupported_device_keeps_legacy_and_does_not_charge_memory
cargo check -p voxy_shell -p voxy_demos --lib
cargo clippy -p voxy_render --lib
cargo test --release -p voxy_render --lib paired_frame_ -- --ignored --nocapture --test-threads=1
python3 docs/architecture-performance/2026-10-20-apple-frame-inputs/summarize.py
```

Run benchmarks sequentially after checks, preferably on an otherwise idle machine.
Raw pair logs, validation logs and the reproducible JSON summarizer are retained
beside this report. The working-tree integration conflicts observed at task start
were resolved by other work; this task did not edit or finalize that merge.
