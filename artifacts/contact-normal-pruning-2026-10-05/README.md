# Conservative normal-stencil pruning

A three-second macOS sampling profile of the existing native imported-contact renderer showed 263 of 2502 main-thread samples under normal-stencil construction. This snapshot is not a whole-clip time breakdown. Force response already uses a conservative triangle separation bound; the normal metric now uses the same bound to skip barriers that are guaranteed inactive. Active closest-feature order and barrier arithmetic are preserved. There is no force-law, CCD, energy or timestep tolerance change.

An independent unpruned path verifies bitwise normal vectors, barycentric weights, curvature, face identities/order and matching closed-gap errors. The oblique layered fixture covers large translation (1e6 m), activation thresholds and narrow gaps. The existing lower-bound tests include 10000 random oblique pairs.

139 physics checks passed (94 library, 8 static equilibrium, 29 prescribed contact, 8 viscoelastic; 4 manual checks ignored). The expanded oracle plus manual benchmark passed. The imported 68-step regression passed in 120.15 seconds. Timing comparisons with earlier runs are uncontrolled and do not establish whole-clip speedup.

On the synthetic layered fixture, five 500-call batches took roughly 31 ms unpruned and 1.4 ms pruned (about 22x for this operation only). Exact results are in `microbenchmark.json`; the runtime benchmark is ignored by default.

The optimized full 480-step CPU regression is starting; authoritative log `/tmp/voxy-normal-pruning-full-clip.log`. The separate renderer, exec session 12276, still runs the previously qualified solver binary and is not validation of this optimization. The engine parity objective, real-time performance and anatomical skin integration remain unfinished.

Reproduce the oracle/microbenchmark:

```sh
cargo test --release -p physics --lib normal_pruning_tests -- --include-ignored --nocapture
```

Live optimized CPU session: 52848. Poll the existing handle; observation timeouts are not terminal failures.

Terminal qualification update: session 52848 exited 0. The normal-pruning-only binary passed all 480 steps and the final energy receipt check in 1174.83 seconds. `full-480-pass.log` and `prescribed_surface-qualified-source.rs` are the evidence. Earlier live-job paragraphs are superseded. This timing was not a controlled whole-engine performance comparison.
