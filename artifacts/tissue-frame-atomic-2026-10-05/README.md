# Complete-frame admission and rollback for tissue previews

`TissueDemo::advance` now rejects negative and nonfinite durations before any
mutation. Zero is a no-op. It stages simulation state and publishes bodies,
biomechanics state, animation time and accumulator only after the whole display
frame succeeds. Immutable render embeddings remain with the owner.

The existing 100 ms preview timestep cap is retained. This is a demo admission
and rollback improvement, not qualification of the engine's global game clock
or overload policy.

The biomechanics preview is cloneable for staging. Its immutable boundary
triangles use a shared `Arc`; body state, reports, hold time and phase remain
independently owned mutable values. Generic physics is unchanged.

Regression coverage:

- Invalid dt and zero-dt behavior in body, abstract tissue and biomechanics modes.
- A failure injected after the second real bone-driven tissue substep. The test
  preserves a fractional-step accumulator before the frame, compares complete
  state after rejection, then requires exact equality with an unaffected control
  after a healthy frame.
- Existing bone-driven pin targets, free-tissue rotation response, late-bone
  failures, pose continuity, stable geometry and settling.

The callback used to exercise second-substep failure is private. Normal app
calls use the existing bone-matrix and tissue-solver path through this same
frame transaction.

Commands:

```sh
cargo test -p voxy_app --lib tissue_demo::tests
cargo test -p voxy_app --lib biomechanics_demo::tests
cargo run -p voxy_app --example body_motion_snapshot -- artifacts/tissue-frame-atomic-2026-10-05/poses.png
```

GPU images are offscreen readback on Apple M4 Max / Metal. Native-window
visibility and other hardware are not verified. Character meshes and cages are
still coarse and material parameters are uncalibrated. Changes remain local.

## Live equilibrium profile

A one-second `sample` capture of the existing running test process is preserved
as `equilibrium-debug.sample`. Sampled stacks pass through `Body::equilibrate`,
`Body::evaluate`, `Element::response`, material response, isochoric invariants and
small matrix products. This is an unoptimized test binary. The capture identifies
active calculation paths; it is not a production performance benchmark or proof
of convergence. Solver options and acceptance thresholds were not changed.

## Rig resource ownership follow-up (verified)

The body demo now constructs its immutable skeleton once at creation and shares
it through `Arc<Skeleton>` with staged frames. Physics and render pose sampling
use that resource instead of rebuilding joint names, hierarchy and inverse-bind
metadata at each call. The edited resource path compiled and passed seven tissue tests. Both existing
biomechanics regressions passed, including all load stages reaching equilibrium
on their unchanged solver settings (993.71 seconds in the unoptimized test run).
The current four-frame Metal PNG matches the previous bone-driven PNG byte for
byte. No measured speedup is claimed. The live process was polled until terminal
completion and was not restarted because of observation timeouts.


## Source-of-truth navigation

RAG index navigation found `voxy-voxy-adr-0001-8f968a85`, page
`1a54f62b-5e07-41c9-8d5c-1a796143b287`. Its local source
`docs/adr/0001-engine-foundation.md` was read directly to verify the single-writer
and immutable-resource principles. The subsequent wiki lexical search exceeded
its retrieval timeout; no facts were inferred from that failed search. The ADR
is an architectural reference, not evidence of current feature completeness.
