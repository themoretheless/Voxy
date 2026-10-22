# Direct instance and particle generation, 2026-10-21

Implemented for Apple Silicon unified memory. This qualification ran on Apple M4 Max / Metal. M1/M2/M3 were not physically tested.

## Ownership and device scope

`voxy_gpu_core::SharedUploadRing::prepare` invokes a fallible producer exactly once into a mapped write-only allocation and returns an immutable `SharedUploadSnapshot`. `voxy_render` owns the shader structs, validation, byte packing and bindings. No arbitrary CPU Vec is wrapped as a Metal buffer.

The gate requires macOS/aarch64, Metal, an integrated `Apple M` adapter, enabled `MAPPABLE_PRIMARY_BUFFERS`, and admission of the three persistent buffers into the existing managed memory budget. This is deliberately restricted to the Apple M family with unified memory, rather than all Metal adapters. Unsupported adapters, disabled features and insufficient budgets retain the validated Vec + legacy upload path.

`NativePlants::sync` now walks persistent per-variant placement indices and writes final `ArchiveInstance` values without constructing grouped render-struct Vecs. Initial mesh construction remains unchanged. `LiquidDemo::optical_particles` borrows simulation particles and lazily forms `FluidRenderParticle`; the shell and liquid snapshot example use the generated upload. The old `optical_scene` and slice update APIs remain compatible. Physics material evaluation and background/film geometry retain their existing representations.

This removes the intermediate render-struct Vec and renderer-owned CPU snapshot from the direct paths. Each final struct is validated in registers and stored to shared memory once. Instance validation uses SIMD rows and checks every world/normal component. CPU snapshots retained from a previous slice update are released after a successful generated update.

## Synchronization and failure behavior

- The renderer retains its current immutable snapshot. Each consuming encoder retains another reference through `on_submitted_work_done`; dropping an unsubmitted encoder or command buffer drops that reference too.
- Only after the CPU snapshot AND all encoder/completion references retire does `map_async(Write)` request CPU access again. A slot becomes ready only after that mapping succeeds. A current snapshot can be encoded into several views/submissions without remapping or rewriting its bytes.
- Snapshots retain `Arc<ComputeStorage>`, not merely raw wgpu handles. Dropping the ring cannot retire/destroy an allocation still owned by a snapshot or a GPU completion callback.
- Ring exhaustion generates once into a unique immutable mapped staging buffer. Each consuming encoder records the staging copy to the legacy target before the corresponding draws. No mutable shared legacy snapshot can overwrite an earlier unsubmitted frame.
- Invalid counts, capacity, non-finite transforms, invalid particle material/radius, camera or films publish no partial frame. All mesh groups prepare before any group is committed. Failed and panicking producers unmap and reclaim their slot through RAII.
- No device-wide wait was added to production. Existing coherent Metal shared mapping and wgpu map/unmap synchronization are retained. Transient staging follows the pre-existing policy outside resident-storage budget accounting.

## Paired CPU measurements

**These are CPU input microbenchmarks, not GPU completion, full frame or FPS results.** They execute the actual renderer update, validation, ring selection/generation, and encoder finish/discard paths. They omit draws and queue submissions. Input generators use synthetic placement transforms/particle coefficients, not a timed full physics simulation; real optical radius/material computation is outside this benchmark's scope.

Before: producer builds the render Vec, slice update owns a CPU snapshot, encoding copies it into shared memory. After: the iterator generates directly in the core-managed mapped allocation. Both paths use the same binary, struct formats and input values. Each run has one warm-up pair and nine measured pairs. A pair consists of 32 alternating/reversed three-frame AB/BA blocks, 96 frames per implementation. The two final runs therefore provide 18 measured pairs per case. A/A uses two independent objects executing the old slice path. Overflow holds all three slots on both objects.

Runs 10 and 11 have no sampled rustc or CPU-replay work. **Two concurrent street_walk rendering processes were present**, recorded in `clients-pairs-10.json` and `clients-pairs-11.json`. This is an explicitly GPU-contended environment; the CPU-only paired effect and stable A/A controls are reported, but no uncontended GPU or whole-frame conclusion is accepted. The device can still experience shared memory bandwidth/thermal effects from those renderers. Bootstrap intervals describe paired sampling uncertainty, not all system-level uncertainty.

| CPU input path | Count | Before, µs | After, µs | Median paired ratio | Bootstrap 95% |
|---|---:|---:|---:|---:|---:|
| Instances, direct | 64 | 1.529 | 1.069 | 0.695 | 0.693–0.701 |
| Instances, direct | 4096 | 57.578 | 22.599 | 0.388 | 0.387–0.394 |
| Instances, forced staging overflow | 4096 | 109.273 | 72.469 | 0.666 | 0.661–0.673 |
| Instances A/A | 4096 | 57.626 | 57.753 | 1.000 | 0.991–1.004 |
| Particles, direct | 64 | 0.962 | 0.795 | 0.831 | 0.827–0.836 |
| Particles, direct | 4096 | 13.604 | 4.351 | 0.324 | 0.320–0.327 |
| Particles, forced staging overflow | 4096 | 35.904 | 27.242 | 0.743 | 0.723–0.764 |
| Particles A/A | 4096 | 13.439 | 13.537 | 1.001 | 0.993–1.007 |

The paired ratios imply about 61% less CPU input work for 4096 instances and 68% for 4096 particles in this microbenchmark. Ratios are medians of per-pair ratios; they need not equal the ratio of the separately pooled time medians. Sub-microsecond small-particle results are especially narrow in scope.

### Retained exploratory/rejected runs

`summary.json` records every exclusion. Early full-render runs 1–3 used the scalar per-component generated instance validator and exposed an instance regression, motivating the final SIMD validator. They are not final acceptance evidence. Their coarse client monitor also missed street_walk. Run 4 used the SIMD version but had extreme GPU contention and was terminated by this task with SIGTERM; partial output and the signal provenance are retained. No other task was signalled. Runs 6–9 were CPU microbenchmark attempts overlapping compilation (and CPU replay in run 6); their raw logs remain available. Run 5 was never started because run 4 was aborted.

No clean final full-render/GPU performance claim is made. The implementation's synchronization and output correctness were tested separately, irrespective of performance contamination.

## Correctness and compatibility

- Core default tests: **30 passed**, four physical/manual tests ignored.
- Core shared upload suite including physical Metal tests: **4 passed**. Checks eight recycle cycles, exact bytes across six queued snapshots × two uses each, ring overflow, failed/panicking generation, cancelled commands, and managed ownership surviving ring destruction.
- Renderer default suite: **332 passed**, 75 physical/manual tests ignored.
- New generated physical parity: **2 passed**, exact image bytes against generated legacy fallback; six queued frames, shrinking/empty counts, late invalid data, mismatched iterator lengths, all 28 transform components rejecting infinity, invalid camera, discarded/repeated encoders, and fluid filtering.
- Previous camera/vegetation/fluid physical parity: **3 passed**; rigid palette parity: **1 passed**. This also exercises the existing slice snapshot path after the core slot ownership change.
- `cargo check -p voxy_app --bin voxy_app --example liquid_snapshot`: passed, including shell/demos consumers.
- `cargo clippy -p voxy_gpu_core -p voxy_render -p voxy_demos --lib`: passed.
- `cargo check -p voxy_gpu_core --target wasm32-unknown-unknown`: passed. Native completion captures are gated so non-Send WebGPU handles do not break the portable fallback build.

Raw logs, environment, binary digest, source hashes and `summary.json` are beside this report. No game window was opened by this task.

## Reproduction

From the repository root:

```sh
cargo test --release -p voxy_gpu_core --lib shared_upload -- --include-ignored --nocapture --test-threads=1
cargo test --release -p voxy_render --lib gpu_generated -- --ignored --nocapture --test-threads=1
cargo test --release -p voxy_render --lib --no-run
# Choose the voxy_render test executable printed above, freeze a copy, then:
python3 docs/architecture-performance/2026-10-21-direct-shared-generation/run-pairs.py \
  --binary /absolute/path/to/frozen/voxy_render-test-binary \
  --input-only --output /absolute/path/to/new-evidence-directory
# For render+submit+drain pairs omit --input-only; the harness requires an idle GPU.
python3 docs/architecture-performance/2026-10-21-direct-shared-generation/summarize.py
```

The summarizer reads the preserved final runs 10/11. The runner always creates a new directory and never overwrites this evidence. A quiet CPU admission window and client samples help identify contamination; before accepting a render run, inspect those samples and A/A controls as well.
