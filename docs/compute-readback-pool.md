# Shared compute readback staging

`ComputeReadbackPool` bounds portable CPU-readback staging per wgpu device. The default limits are 64 MiB and 64 buffers, including active leases, cached buffers and quarantined buffers. This is not a total VRAM, world storage or CUDA allocation budget.

Configure the pool before constructing compute programs or a Renderer. Programs, jobs, the Renderer and gravity jobs retain the same pool. `stats()` exposes charged bytes, buffers, cache/quarantine counts and creations/reuses. Separate devices have separate admission limits.

ComputeJob encode/snapshot/readback and ComputeDispatch copy_buffer acquire capacity before recording commands. Exhaustion returns ComputeError::ReadbackBudget. encode_readback and encode_snapshot now return Result; callers must handle admission failure. Consumers decide whether to retry later or report failure.

A mapping callback retains its lease until completion. Successfully mapped buffers return to the cache only after unmap; consumed pending reads release their lease immediately. Cancelling before confirmed mapping, or a mapping failure, quarantines the allocation and preserves its budget charge even if the last program is dropped. Native discard_quarantine explicitly destroys abandoned buffers and waits for completion; any unsubmitted encoder referencing those buffers must be discarded first. Poll failures retain charges. This operation blocks and is never called implicitly by normal readback. Browser quarantine has no explicit retirement API yet. Automatic device-loss recovery remains separate work.

## Evidence

Apple M4 Max / Metal compute_smoke passed shared 8192-byte admission across programs, mapped cancellation, exact reuse, an unsubmitted cancellation quarantine and explicit cleanup. The existing 1042 exact-result checks, independent resident snapshots, dispatch validation and shader reload checks also passed. No throughput improvement is claimed.

Final verification: three focused staging admission/cancellation/device-isolation tests and two existing compute ownership tests passed. Native voxy_render/voxy_gpu examples and wasm32 voxy_web compilation passed. Strict Clippy still fails with twelve diagnostics in fluid_screen.rs; the new readback module has no diagnostics in that run. A compilation check does not establish execution on NVIDIA, mobile or XR hardware.
