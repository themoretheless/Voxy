# Canonical implicit path endpoint

The implicit solver now retains old_position + 2*local_midpoint_displacement as the final position: the same endpoint used by path quadrature and nonlinear admission. Velocity still comes independently from the impulse equation, preserving representable tiny impulses. The actual final trajectory is rechecked. Force convergence additionally bounds 2*sum(residual*(gradient-mass*acceleration)/inertia_weight), the energy effect of the kinematic residual, within 1/8 of the unchanged energy budget (with the existing loose-diagnostic cap). Existing force-norm and independent work/collision guards are retained. No energy/work tolerance was increased.

80 library tests passed (2 manual benchmarks ignored), 23 prescribed-contact and 8 viscoelastic tests passed, 19 tissue tests passed: 130 total. New tests cover an open-gap ULP energy sensitivity and an actual strong moving-contact solve at initial 1 micrometre gap with 1e-9 J independent energy admission, swept-path openness and bounded drift residual. Existing tiny-step impulse, gravity, Galilean, Maxwell/thermal and rollback tests pass.

The actual full imported Metal render still FAILED at step 65, time 0.270833333 s, now on implicit contact nonlinear nonconvergence. It committed 64 steps and generated six prefix frames; these do not qualify the full two-second clip. Last committed receipts and refinement are in full-contact-render.log. The fix removes an identified endpoint inconsistency but has not established robust convergence at this contact.

Next inspect force norm versus the new residual-work condition at iteration exhaustion, and contact-coordinate precision/conditioning. All changes remain local, uncommitted and unpushed.
