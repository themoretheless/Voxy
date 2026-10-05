# Implicit midpoint prescribed contact

InertialBody::step_implicit_with_surface_motion solves the midpoint position using an inertial objective and the existing material/contact potential at the midpoint obstacle pose. Free-node residual: 4m/dt² (x_mid-x_old-dt v_old/2) + gradient_mid - m acceleration. Prescribed pins use endpoint-average midpoint positions. Velocities are reconstructed by the midpoint rule; pin kinetic impulse work is accounted separately.

The nonlinear solver uses the positive contact-normal diagonal plus inertia as a preconditioner, bounded iterations and Armijo backtracking. Candidate endpoints pass existing internal gap, tetrahedral swept volume and prescribed mesh continuous separation checks. The converged result must also pass endpoint geometry, finite response and independently computed energy-work admission. Obstacle work is midpoint obstacle gradient dotted with prescribed displacement; it is not inferred from energy change. Nothing publishes before admission.

This entry point requires time-independent materials and shares existing material/stationary-support guards. It does not yet perform Maxwell relaxation/thermal splitting, and is not wired to the imported tissue demo. That clip's frame-52 failure is therefore not claimed fixed.

16 prescribed contact integration tests passed, including three new solver fixtures: actual constant-acceleration free motion and contact impulse; positive moving-obstacle work with energy closure and atomic crossing/incomplete-target rejection; 100 steps of Galilean-equivalent actual dynamics with independent work=speed*impulse checks. History-bearing input rejects atomically. Tests qualify these fixtures, not general convergence or production readiness.

Next route the mechanical solve through the existing transactional Maxwell/thermal split, compare explicit/implicit behavior with independent reference solutions and qualify the actual imported rig clip at unchanged admission tolerance.
