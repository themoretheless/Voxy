# Native rig frame profile

Set `VOXY_ANIMATION_PROFILE=1` for `--animation-native-smoke`. Profiling extends
Play to 120 fixed simulation steps and emits per-presented-frame stage timings.
Without this opt-in the smoke retains its normal 12-step acceptance duration.

The total CPU interval starts before imports/audio polling and fixed updates,
then includes scene extraction, UI preparation, owner synchronization, LOD
selection, draw preparation and the host render/submit/present call. It excludes
event-loop idle and does not wait for GPU completion. Thus it is not GPU timing,
a complete input-to-display latency measurement, or an FPS guarantee.

The release reference fixture has two owners sharing one 370-vertex rig source,
768 base indices and 510 reduced indices. Excluding five initial Play frames,
235 samples yield CPU median 8237.125 us and p95 9624.750 us. Stage medians:
animation 1054.959 us, LOD 2412.125 us, host submit/present 5312.958 us. Stage
percentiles must not be summed: their maxima need not occur in the same frame.

Certificates now retain one accepted exact palette/world pair per owner.
Matching palettes and matrices reuse it across cameras and paused frames.
Source or pose replacement installs a new verified certificate only after the
animation frame has passed preflight. Failed certification, camera selection or geometry admission preserves the
previous cache and view history. The new certificate commits with the accepted
view selection, after optional GPU/CPU geometry admission. Alternating world matrices can miss this bounded cache;
CPU certificate storage remains separate from the GPU geometry budget.

The GPU regression checks reuse by buffer identity across two cameras in both
GPU and CPU fallback paths, plus the existing admission/error/eviction cases.
Native acceptance still verifies independently moving and paused owners, far
reduced LOD, near-plane base fallback, eviction and Stop with zero animation bytes.
The captured debug baseline is not a release comparison or proof of speedup.

Evidence: `artifacts/rig-frame-profile-2026-10-03/` contains raw logs, CSV,
source/binary hashes and the report. Large rigs, split views, GPU timestamps,
normal/attribute LOD quality and other hardware remain open acceptance work.
