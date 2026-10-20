# Apple Silicon shared transient uploads — 2026-10-20

Implemented for the frequently updated rigid-skinning dual-quaternion palette and
its 32-byte parameter block. Enabled automatically by SceneSurface on native
macOS/aarch64 Metal when MAPPABLE_PRIMARY_BUFFERS is supported. Renderer already
requests this feature. Devices supplied by external callers must enable it;
unsupported devices and insufficient managed ring budgets retain legacy uploads.
Other cameras, mesh uploads, fluids and animation paths have not been migrated.

## Memory and synchronization

SharedUploadRing admits three primary MAP_WRITE buffers against ComputeMemoryBudget.
The vendored Metal backend selects StorageModeShared for MAP_WRITE, maps the actual
MTLBuffer.contents() pointer with is_coherent=true, and has empty unmap/flush hooks
(vendor/wgpu-hal/src/metal/device.rs:486–539). The fast path generates the final
shader palette and parameter bytes through wgpu's WriteOnly mapping. It does not
build a packed upload Vec, call queue.write_buffer, or encode an upload copy.
The validated CPU pose snapshot remains a separate CPU allocation, preserving the
existing upload/encode API and rejection behavior. This is zero staging transfer
for the final GPU input, not elimination of all CPU storage or DRAM traffic.

Each encode leases one slot, drops the CPU view, unmaps it, records consumers, and
uses map_buffer_on_submit to regain CPU access only after that submission's GPU
uses finish. Production calls only Poll; no device-wide wait is added. Deferred
callback cancellation re-maps slots from discarded encoders/command buffers.
Cached bind groups bind disjoint, explicitly sized palette and parameter ranges.
Repeated encode calls and multiple unsubmitted snapshots use independent leases.

If all slots are busy, an immutable, uniquely owned staging snapshot is generated
and its two copies are recorded before the skinning dispatch in the same encoder.
Thus overflow still copies, but cannot overwrite another unsubmitted snapshot and
does not wait for it. Staging is outside the existing resident-memory budget contract;
the persistent ring and legacy fallback targets are accounted. A configured budget
includes retired ring resources until the application's usual retirement cleanup.

## Measurements

Apple M4 Max, Metal, macOS 26.7.2 (25G325), rustc 1.98.0-nightly
(d1fc603d1 2026-05-26), release profile. Base HEAD when final checks ran:
0150379877cb96007aaae0965d4e6e5e80e433f7, with this patch uncommitted.
Only M4 Max was physically tested; M1/M2/M3 share the selected code path but were
not tested on hardware.

Two final series, each nine same-process pairs after warming both paths. Order
alternates shared/legacy and legacy/shared. Each half-pair has 48 frames and drains
after each three-frame batch. No samples were trimmed. CPU timing covers production,
encoding and submission; drained wall timing also covers GPU completion waits.

The generic upload benchmark generates dual quaternions, reads every generated word
in a real Metal shader, and verifies every output word outside the timed interval
after every half-pair. Its shared path asserts ring availability. The integrated
benchmark exercises the actual rigid upload/encode functions, including matrix
validation and four-vertex GPU skinning; it is not a full character/frame benchmark.
The baseline retains the legacy Vec generation + two queue.write_buffer operations.

CPU results below are medians per half-pair in microseconds/frame. Ratio is the
median of the nine **paired** shared/legacy ratios, not the ratio of column medians.

| Test | Size | Series | Legacy CPU µs | Shared CPU µs | Paired ratio |
|---|---:|---:|---:|---:|---:|
| upload | 1024 bytes | 1 | 63.543 | 30.907 | 0.566 |
| upload | 32768 bytes | 1 | 117.924 | 40.135 | 0.329 |
| upload | 262144 bytes | 1 | 231.217 | 66.445 | 0.286 |
| upload | 1048576 bytes | 1 | 548.914 | 153.119 | 0.273 |
| upload | 1024 bytes | 2 | 55.124 | 24.705 | 0.463 |
| upload | 32768 bytes | 2 | 124.273 | 41.402 | 0.313 |
| upload | 262144 bytes | 2 | 179.720 | 44.761 | 0.249 |
| upload | 1048576 bytes | 2 | 387.264 | 78.738 | 0.215 |
| rigid | 32 joints | 1 | 83.392 | 41.321 | 0.487 |
| rigid | 1024 joints | 1 | 246.560 | 129.644 | 0.512 |
| rigid | 8192 joints | 1 | 812.046 | 686.089 | 0.832 |
| rigid | 32 joints | 2 | 43.898 | 23.380 | 0.537 |
| rigid | 1024 joints | 2 | 139.167 | 79.749 | 0.578 |
| rigid | 8192 joints | 2 | 565.720 | 435.359 | 0.776 |

Integrated paired median CPU reductions are 46–51% (32 joints), 42–49%
(1024 joints), and 17–22% (8192 joints). Individual noisy pairs remain in the logs;
for example, the first series at 32 joints includes a CPU ratio above 1.

**Drained GPU/frame improvement is not qualified.** Concurrent system load included
other builds and CPU test jobs. Drained paired ratios have extreme outliers, and
one integrated 1024-joint series has median ratio 1.047 (regression). All drained
measurements and min/max paired ratios remain in summary.json and the raw files.
CPU medians support retaining this scoped change; they establish neither a universal
speedup nor any FPS, p99, or GPU execution-time claim. No other jobs were stopped.
The preliminary files preserve the earlier, less-loaded measurements reported in
progress updates; they predate cancellation/overflow hardening and are not the final
acceptance numbers above.

## Verification

- Physical Metal tests: two new tests passed, plus the existing shared mesh budget
  GPU test selected by the same filter. No validation errors.
- Ring test: three unsubmitted snapshots, eight complete recycling cycles, busy
  rejection without running either callback, cancelled encoder/command-buffer
  recovery, invalid-size rejection, and atomic managed-budget rejection.
- Actual skinning test: three direct slots plus three simultaneous staged overflow
  snapshots match legacy output bit for bit; invalid poses preserve the last valid
  snapshot, and re-encoding without a new upload preserves output.
- Noop fallback test: passed, without charging managed memory.
- voxy_shell and voxy_demos consumer checks passed.
- Renderer library Clippy passed with existing warnings outside modified files.
  This is not a warning-free workspace gate.
- Scoped rustfmt and git diff --check passed. No complete workspace test run or
  full-window animation/FPS qualification was performed.

## Reproduce

```sh
cargo test --release -p voxy_render --lib gpu_shared_ -- --ignored --nocapture --test-threads=1
cargo test --release -p voxy_render --lib unsupported_device_keeps_legacy_and_does_not_charge_memory
cargo run --release -p voxy_render --example shared_upload_bench
cargo test --release -p voxy_render --lib paired_rigid_palette_benchmark -- --ignored --nocapture --test-threads=1
cargo check -p voxy_shell -p voxy_demos --lib
cargo clippy -p voxy_render --lib
python3 docs/architecture-performance/2026-10-20-apple-shared-upload/summarize.py
```

Raw upload CSVs, production pair logs, validation logs and the summary are retained
beside this report. Run benchmarks sequentially, after checks/builds, with an idle
machine if GPU/frame completion timings are needed.
