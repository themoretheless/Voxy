# Analytic weighted root displacement

Root translation compiles immutable piecewise polynomial coefficients and prefix
integrals at clip construction. Clones share the cache through Arc. LINEAR uses
linear segments, STEP uses constant segments with right-continuous jumps, and
CUBICSPLINE uses Hermite derivatives scaled by segment duration in seconds.
Binary search locates a segment; complete loop cycles are summed algebraically.
A million cycles do not require a million iterations.

Integration by parts evaluates integral w dR for a linear fade weight:
w(end) times displacement minus the weight slope times the integral of relative
position. STEP impulses receive the weight at their actual key time, including
loop-boundary jumps. All positions are relative to the first key, so a large
constant authored offset produces exactly zero displacement. Coefficients and
integrals use f64; source, target and completion-tail contributions are accumulated
before conversion to f32 and finite-output admission. Errors preserve animator state.

Analytic tests verify cubic half-interval 0.15625, STEP one/four-cycle motion
2.75/10.25, exact split-step event accounting, clamp tails, nonzero cubic derivatives
in seconds, 1048576 cycles and constant 1e38 offsets. Animator-level regressions
verify STEP substep equality and cubic motion 0.5 then -0.5 during a two-second fade.
253 ordinary tests pass (30 animation, 92 editor, 131 renderer). Artifacts:
artifacts/rig-root-integral-2026-10-03/.

Existing animation clocks remain f32, so long-running time precision is still a
limit. Prefix storage costs CPU memory proportional to root keys and is outside
the GPU residency budget. No frame-performance or new GPU acceptance is inferred.
Explicit joint selection is implemented; see [selection](root-motion-selection.md).
Rotation extraction, in-place conversion, gameplay/collision
application, editor controls and interruption velocity continuity remain incomplete.
