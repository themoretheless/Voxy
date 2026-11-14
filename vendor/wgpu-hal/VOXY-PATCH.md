# Voxy Metal backend patches

Vendored from crates.io `wgpu-hal 30.0.1`, with original licenses retained.

Runtime changes relative to upstream:
1. `src/metal/surface.rs`: dynamic CoreGraphics `CFStringRef` globals require
   dereferencing their exported storage address before passing the `CFString`
   object to `CGColorSpaceCreateWithName`. Treating the storage address as an
   object crashed the HDR10 surface probe with `SIGBUS`. Null checks report a
   `SurfaceError` instead of constructing a null reference.
2. `src/metal/device.rs` and `src/metal/original_gi_math.rs` (under feature
   `metal-safe-compute`): compute pipelines default to `fastMathEnabled = false`
   with `#pragma STDC FP_CONTRACT OFF`, while any stage may explicitly override
   the policy via pipeline-override constant `WGPU_METAL_PRECISE_MATH` (`> 0`
   forces precise math, `< 0` forces fast math, `0` or omitted keeps default).
