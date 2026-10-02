# Voxy Metal color-space fix

Vendored from crates.io wgpu-hal 30.0.1, with original licenses retained.
Only src/metal/surface.rs changes runtime behavior: dynamic CoreGraphics
CFStringRef globals require dereferencing their exported storage address before
passing the CFString object to CGColorSpaceCreateWithName. Treating the storage
address as an object crashed the HDR10 surface probe with SIGBUS.

The null check reports a SurfaceError instead of constructing a null reference.
Remove this Cargo patch only after the selected upstream release contains the
fix and the linear/PQ window probes both pass. Other backends are unchanged.
