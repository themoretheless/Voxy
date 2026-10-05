# Instantaneous quadrature poses

Public `with_positions` retains the swept BVH for motion admission. Private quadrature samples stage identical validated geometry, immutable owner, instantaneous BVH and prepared triangles without a swept index that their force/normal queries do not use. No contact law, CCD, energy, timestep or residual threshold changes. Temporary samples explicitly clear any stale swept cache. Public motion caching remains present; the fallback CCD result matches when the cache is absent.

140 focused physics tests passed. Oracle cases compare geometry, forces, normal blocks, crossing/error decisions and owner identity for public versus temporary staging. The imported 68-step regression with both normal pruning and temporary staging passed in 116.13 seconds. Timing versus previous runs is uncontrolled and does not prove whole-clip speedup.

Five synthetic batches of 25 poses with 4096 triangles measured about 1.29x preparation speed, solely for this operation. Exact timings: `microbenchmark.json`. Run `cargo test --release -p physics --lib instantaneous_pose_tests -- --include-ignored --nocapture` to reproduce.

A new full 480-step CPU gate for the combined source is starting, log `/tmp/voxy-instant-pose-full-clip.log`. Earlier normal-pruning-only gate (session 52848) and baseline GPU render (session 12276) remain distinct validations of earlier binaries. Full combined qualification, real-time performance, anatomical skin integration and engine parity remain outstanding.

The stale-cache oracle also passes when the source already has a swept cache; the transient pose explicitly clears it. `current-tested-source.rs` includes this later cfg(test)-only strengthening; production code is unchanged from the full-gate source snapshot. Baseline GPU render completed successfully (41 frames/2 seconds), recorded separately in the frame-diagnosis artifact.

Terminal update: session 34554 exited 0 after all 480 steps and the final energy receipt check (1051.94 seconds). `full-480-pass.log` qualifies the sequential combined normal-pruning/instantaneous-pose binary. Earlier pending text is superseded. Timings were not controlled whole-engine performance comparisons. The later parallel-region gate remains separate.
