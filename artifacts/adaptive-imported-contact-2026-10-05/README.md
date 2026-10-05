# Adaptive imported character contact

Implicit surface contact now uses a fixed quadrature partition during each nonlinear solve. On work rejection, independent per-panel contact-energy differences are compared with joint body/obstacle force-integrated work; the worst interval is bisected and the nonlinear solve restarts from unchanged state. The endpoint-energy estimator chooses refinement only: it never replaces actuator work. Accurate contact quadrature with remaining midpoint material error returns to the existing temporal subdivision. Searches remain bounded to 32 quadrature attempts. Existing collision/work/residual budgets and atomic publication are retained.

The new endpoint regression covers a gap falling from 42 nm to 42 pm and independent contact work error below 1e-10 J. Invalid partitions and held-contact no-refinement are checked.

Confirmed results: 76 physics library tests passed (2 existing manual benchmarks ignored), 21 prescribed-contact and 8 viscoelastic tests passed, 18 tissue tests passed. The previously failing imported 52-step expected-success regression now PASSES (40.89 seconds).

The full two-second Metal contact render FAILED at step 65 (time 0.270833333 s) with implicit contact line search failed, after committing 64 steps. Six prefix frames cover 0 to 0.25 s; the last frame was visually inspected. No complete contact GIF is delivered. This advances beyond the former step-52 failure but does not qualify the full clip. Last committed energy/refinement receipts are preserved in full-contact-render.log. No production anatomy, physiological calibration, realtime or other GPU-backend claim is made. Source changes remain local and unpushed.
