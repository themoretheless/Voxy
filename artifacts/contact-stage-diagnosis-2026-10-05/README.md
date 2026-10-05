# Implicit contact stage diagnosis

Optional IMPLICIT_STAGE_REJECTION output observes errors without replacing them. It distinguishes local quadrature, local normal blocks, world material midpoint, initial diagnostics, final world contact and the independent world quadrature error estimator.

The complete imported run terminated at step 65 / 0.270833333 s. Its smallest main leaf (dt=2.5431315104166666e-7 s) rejected local contact quadrature with closed surface contact gap. Diagnostic after-state retained the initial nearest gap 5.8031836985567714e-11 m and the unchanged body diagnostics. The half-step observation also failed locally and did not replace the original error. This indicates an infeasible stationary free-node initial guess for the moving obstacle, rather than a final committed world endpoint rejection.

An independent public regression reproduces the failure with a co-moving body and obstacle: stationary free-node geometry closes the barrier, while the known co-moving predicted pose remains open. Its before-fix test log is retained in ../feasible-contact-predictor-2026-10-05/predictor-before.log.
