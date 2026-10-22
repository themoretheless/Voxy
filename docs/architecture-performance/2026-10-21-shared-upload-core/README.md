# SharedUploadRing relocation into voxy_gpu_core — 2026-10-21

## Architecture

The production implementation now lives in
`crates/voxy_gpu_core/src/shared_upload.rs`, exported by voxy_gpu_core.
voxy_render re-exports SharedUploadRing for existing users; camera/instance/particle
formats, validation, camera snapshots, shaders and bind-group construction remain
in voxy_render. No new runtime dependency or dependency from core to render was
introduced. Apple Metal selection uses Device::adapter_info(), removing the native
HAL inspection/unsafe block from the common utility.

The following device-level responsibilities moved together:

- Three MAP_WRITE primary buffers, CPU/GPU mapping ownership and nonblocking poll.
- Deferred remapping after GPU completion and recovery when command buffers are discarded.
- Unique immutable overflow staging, including the retained-binding legacy helper.
- Resident allocation/admission/retirement through the existing core ComputeMemoryBudget.
- Buffer enumeration for prebuilt caller bindings and allocation byte accounting.

Rigid skinning's duplicate staging implementation was also removed from the
renderer. It now describes two byte ranges with the format-independent
SharedUploadCopy type and calls core stage_copies. The core validates every source
range, target range, alignment and COPY_DST usage before producing data or recording
any copy. Shader packing and all joint/parameter interpretation remain in render.
Transient staging remains excluded from the existing resident-budget contract;
this refactor does not silently change accounting semantics or add waits.

## Correctness and build checks

On Apple M4 Max / Metal:

- Core default suite: **30 passed, 3 ignored**.
- Core shared-upload suite including physical tests: **3 passed**. It verifies
  queued snapshots, cancellation recovery, recycling, unsupported-device fallback,
  atomic admission, split copies, unchanged target holes and rejection of invalid
  copy plans before invoking their producer.
- Render camera/vegetation/fluid physical parity: **3 passed**, including queued
  snapshots, overflow, binding refresh, empty/shrinking inputs and standalone filtering.
- Rigid skinning physical parity: **1 passed**, exercising the new two-range core
  staging path as well as direct mapped slots.
- Checks of voxy_gpu, voxy_shell and voxy_demos passed.
- Core/renderer Clippy passed with pre-existing warnings outside the changed mechanism.
- Test/example compatibility check, scoped formatting and diff whitespace checks passed.

Logs are beside this report. The unrelated dirty work was preserved. There was no
whole-workspace/release-package check and no M1/M2/M3 hardware test.

## Before/after comparison

This is **pre-move shared uploads versus the relocated shared uploads**, not another
comparison against the older queue.write_buffer path. The exact pre-move working-tree
implementation (production part only; trailing blank lines normalized) is frozen in
`crates/voxy_render/tests/fixtures/shared_upload_before.rs`; the identical artifact
copy is `shared_upload_before.rs` here. The summarizer checks that they match.
Its SHA-256 is:

`01df61f8900f16378c2b64688c54e519013764f924a8ba131a17b00b15f8e68b`

The reference is compiled only for the renderer's macOS/aarch64 tests. Production
contains only the core implementation; the renderer's old module was removed.

Both implementations are compared in the same release process. Camera-sized
(320 B), 4096-instance-sized (458752 B) and 4096-particle-sized (131072 B) inputs use
identical byte producers and the same independent Metal compute consumer. Every
GPU output word is verified outside the timed region. Slot assertions ensure that
the direct cases do not silently fall back. The overflow case deliberately holds
three unsubmitted leases and checks the staging path. The A/A control compares two
instances of the frozen implementation rather than treating a renamed new path as
its own baseline.

Two initial series used 24 contiguous frames per half-pair. Their A/A median ratios
were 0.814 and 1.434, demonstrating unacceptable drift for a no-regression conclusion.
They are retained as initial-pairs-*.log and included in summary.json, but are not
acceptance results. The protocol was changed, without changing production code,
to interleave three-frame blocks, reversing order each block. Each pair totals
96 frames per implementation; each series has nine measured pairs after a complete
warm-up pair. Two such series were retained in full. GPU waits, output verification
and held-slot cleanup are outside isolated upload timing; waits are included in
drained timing. CPU timing includes encoder creation, upload, compute encoding and
queue submission. All durations below are microseconds/frame; ratio is the median
of the nine paired after/before ratios, not a ratio of column medians.

| Case | Series | Before CPU µs | After CPU µs | Paired CPU ratio |
|---|---:|---:|---:|---:|
| camera | 1 | 10.218 | 10.250 | 1.013 |
| control_aa | 1 | 9.324 | 9.622 | 0.994 |
| instances | 1 | 19.017 | 19.492 | 0.961 |
| overflow | 1 | 18.388 | 16.686 | 0.966 |
| particles | 1 | 12.579 | 12.955 | 1.004 |
| camera | 2 | 9.019 | 9.209 | 1.022 |
| control_aa | 2 | 10.555 | 10.598 | 0.998 |
| instances | 2 | 16.613 | 16.883 | 0.989 |
| overflow | 2 | 16.192 | 17.167 | 1.027 |
| particles | 2 | 11.753 | 12.248 | 1.038 |

For the studied input preparation/submission workloads, median CPU differences
were **-3.9% to +3.8%**, with no material relocation regression detected. A/A median
ratios were 0.994 / 0.998 and individual control pairs also fluctuate. This supports
retaining the refactor, not an assertion of exactly zero instruction/time overhead.

The isolated sub-microsecond camera upload is a precision limit worth retaining:
its paired ratios were 1.069 / 1.044, with median durations 0.330→0.359 µs and
0.290→0.312 µs. The absolute difference is 0.022–0.029 µs, inside the observed control
spread; complete CPU preparation/submission camera ratios were 1.013 / 1.022.
Large payload isolated-upload ratios were approximately 0.997–1.004. Overflow CPU
ratios were 0.966 / 1.027. All per-pair regressions and ranges remain in summary.json.

This is a bounded relocation check. It does not establish an FPS improvement, a
universal zero-overhead guarantee, hardware-wide performance or exact equality of
GPU completion times under background load. Live system jobs were not stopped.
Environment and source hashes are in environment.json.

## Reproduce

```sh
cargo test --release -p voxy_gpu_core --lib -- --test-threads=1
cargo test --release -p voxy_gpu_core --lib shared_upload -- --include-ignored --nocapture --test-threads=1
cargo test --release -p voxy_render --lib gpu_frame_ -- --ignored --nocapture --test-threads=1
cargo test --release -p voxy_render --lib gpu_shared_rigid -- --ignored --nocapture --test-threads=1
cargo check -p voxy_gpu -p voxy_shell -p voxy_demos --lib
cargo check -p voxy_render --tests --example shared_upload_bench
cargo clippy -p voxy_gpu_core -p voxy_render --lib
cargo test --release -p voxy_render --lib paired_shared_upload_relocation -- --ignored --nocapture --test-threads=1
python3 docs/architecture-performance/2026-10-21-shared-upload-core/summarize.py
```

Run the benchmark twice, sequentially, recording each output as pairs-1.log and
pairs-2.log to regenerate the final summary. The initial logs document the discarded
coarse protocol. Neither those noisy measurements nor individual regressions were
removed.
