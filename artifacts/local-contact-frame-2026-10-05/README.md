# Local-frame implicit contact evaluation

The implicit solver now evaluates contact in a frame at the initial body's first node. Local endpoints are constructed directly from the displacement unknown with FMA, retaining corrections below world-coordinate resolution. Contact gradients and PSD normal blocks use the same local quadrature poses. Material evaluation, swept collision admission, physical energy diagnostics and committed endpoints remain in world coordinates; final independent work/energy admission is still mandatory. No budgets or physical coefficients were loosened. Local surface topology and authored exclusion masks retain their existing ownership.

A new analytic narrow-gap regression verifies that an outward 2e-18 m displacement disappears at a world height near 1 m but changes local force and objective in the physically expected direction. This demonstrates retained sensitivity, not full arbitrary-precision geometry.

85 library tests pass (2 manual benchmarks ignored), 24 prescribed-contact, 8 viscoelastic, 52 related contact/geometry and 19 tissue tests pass. git diff --check passes. The full imported contact render terminated at step 65 / 0.270833333 s with implicit contact line search failed. Only six prefix frames were produced. The one-second runtime sample shows active recursive temporal subdivision and contact quadrature evaluation, not an idle process. The separate rejection trace also terminated at step 65 with line search failure, and is saved as rejection-trace.log. Its early blocked searches have all 48 volume trials admissible and zero CCD-admissible trials; the original source face 1308 is implicated near path time 1. No complete imported contact clip is qualified.

All changes remain local and uncommitted.
