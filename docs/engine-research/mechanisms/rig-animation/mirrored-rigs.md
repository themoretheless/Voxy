# Mirrored skeletal transforms

Bind transforms and LINEAR, STEP and CUBICSPLINE scale channels accept finite,
nonzero scales of either sign. Sampled poses and blended poses reject singular
zero scales; failed animator advancement retains the previous clock and pose.

CPU tests cover all eight scale sign combinations, hierarchical palettes,
imported reflected positions and inverse-transpose normals, interpolation through
zero, signed endpoints and transactional rejection. The current suite passes
238 ordinary tests: 17 animation, 91 editor and 130 renderer.

The pinned RiggedFigure is modified separately with a reflected root matrix and
an animated negative X scale. Each fixture passes 65 GPU samples on Apple M4 Max
Metal, comparing CPU positions and authored normals and requiring visible pixels.
Maximum position error is 2.3841858e-7; normal errors are 2.0861626e-7 for the bind
reflection and 1.8907454e-7 for the animated reflection. Logs are retained under
artifacts/rig-mirrored-scale-2026-10-03/.

These synthetic fixtures verify skinning and visibility in the double-sided scene
renderer. They do not establish complete glTF single-sided winding/culling support,
legacy shader normal parity, continuous-time curve validity or non-Metal hardware
acceptance. Near-singular normal matrices remain subject to GPU admission checks.
