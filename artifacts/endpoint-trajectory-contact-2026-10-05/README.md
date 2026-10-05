# Shared contact trajectory interpolation

CCD, prescribed obstacle quadrature poses, body quadrature responses, normal blocks and error-estimation boundaries now share trajectory_point. Authored endpoints are returned exactly; interior interpolation starts at the nearer endpoint and uses fused multiply-add. Analytic tests cover loss of unit endpoints after displacement from 1e16 and reversal consistency at binary-exact times. This does not establish exact arbitrary-precision interior interpolation.

Validation: 84 library tests pass (2 manual benchmarks ignored), 24 prescribed-surface tests, 8 viscoelastic tests, 52 related geometry/contact tests, and all 19 tissue tests pass. git diff --check passes. No energy or contact admission budgets changed.

The full imported contact render terminated with nonlinear nonconvergence at step 65 / 0.270833333 s, producing only six prefix frames. The prior completed diagnostic log is stored in ../affine-contact-precision-2026-10-05/nonlinear-trace.log. Endpoint-preserving interpolation alone did not remove that rejection.

All changes remain local and uncommitted.
