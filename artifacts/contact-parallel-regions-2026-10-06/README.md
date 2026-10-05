# Deterministic parallel tissue contact regions

Independent prescribed-contact regions now run on bounded native scoped workers (at most four and at most the available hardware/region count). Non-contact steps remain sequential; WASM falls back to sequential execution. Each region owns its mechanics, Maxwell history, heat and energy ledger and reads immutable shared pose/surface inputs. A common region kernel implements both paths. All workers join before the first error is selected in original region order. The complete frame candidate is published only after all regions succeed. Spawn/panic failures reject publication. No new pool dependency, force law, tolerance, iteration or CCD limit was introduced.

21 TissueDemo tests passed, including exact serial/parallel complete-state comparison, simultaneous error ordering, late-region rollback and recovery. An additional parallel second-frame failure test passed, preserving all regions, clock and accumulator and recovering to the control state. Eight imported contact steps match complete serial/parallel state after every step. The normal imported 68-step regression passed (103.29 seconds). These are correctness checks, not real-time qualification.

Three paired eight-step trials on the imported fixture used common precomputed Pose64 samples and excluded model/provider setup. Median region-step speed ratio was about 3.34x. Complete final states matched in every trial. Full-clip/end-to-end speedup is unproven and cannot be inferred by multiplying other microbenchmark ratios.

The full 480-step parallel CPU gate is starting at `/tmp/voxy-parallel-regions-full-clip.log`. The separate instantaneous-pose sequential gate remains live in session 34554. The normal-pruning-only sequential gate passed all 480 steps in 1174.83 seconds (source and terminal evidence in `artifacts/contact-normal-pruning-2026-10-05/`). Full baseline GPU visual evidence is in `artifacts/contact-frame-diagnosis-2026-10-05/`; it precedes these optimization variants.

Real-time performance, anatomical skin integration and the full engine parity objective remain unfinished.

Sequential combined qualification also completed: session 34554 passed all 480 steps and final energy receipts in 1051.94 seconds. The source snapshot and terminal log are in the instantaneous-poses artifact. This does not qualify the new parallel full gate, still live in session 97744.

Terminal full qualification: session 97744 exited 0, passing all 480 steps and final energy receipts in 766.94 seconds. `full-480-pass.log`, `implicit-qualified-source.rs` (hash checked against the pre-diagnostic solver), and the prior app/source snapshots are the evidence. Earlier pending text is superseded. This elapsed time is measured; it is not a controlled end-to-end speed comparison and is still far from real-time execution.
