# Prescribed FEM support motion and actuator work

The existing InertialBody Verlet path now supports one next-position target per
pin. Free nodes retain inertia. Prescribed velocity is the support segment's
displacement divided by dt. Actuator work is trapezoidal potential-gradient
reaction work plus exact pin kinetic-energy change. Delta(K+U)-work is guarded
independently before publishing any positions or velocities.

Tests cover rigid translation with gravity and kinetic work, analytic simple
shear work, explicit stopping, free-node lag and timestep convergence, duplicate
and incomplete controls, free-node/nonfinite controls, inversion and strict
work-defect rollback. Existing free-body inertia and muscle tests are included.

```sh
cargo test -p physics --test moving_fem_supports --test finite_inertia --test muscle_dynamics
```

This is prescribed kinematics with actuator work. It does not limit actuator
energy or combine moving supports with rate-dependent muscle integration. The
moving mannequin has not yet been converted to FEM and requires stable substeps
and full-frame transaction integration. Results are in report.json after the
process completes. All changes remain local; the full engine goal is active.

Numerical follow-up: large pin kinetic work is cancelled analytically before
forming the energy defect. Reaction work has its own report field. If total
work loses a component above the declared energy tolerance, the state is rejected.
Smaller components remain in the separate report fields. A regression without
gravity caught and corrected a false rejection of roundoff-scale rest-strain
work during rigid translation, without widening the energy tolerance. An additional fast-compression regression
covers this gate. The first test run precedes this numerical follow-up; the
final support/inertia run qualifies it separately in report.json.

A second follow-up certifies the minimum oriented determinant over each Verlet
linear drift segment using cubic coefficients and derivative roots. Endpoint
validity alone can miss mid-segment collapse. Tests include supported and free
180-degree flips, a cubic interior inversion and an admissible quarter turn.
These additions passed the final-source support/inertia tests in debug and
release profiles. The original muscle suite also completed without restart.

The initial full debug run completed: 4 inertia, 4 initial support and 10 muscle
checks passed; the muscle target took 1558.19 seconds. The final isolated debug
run passed 4 inertia and 6 support tests, including both gravity and zero-gravity
translation. The initial release-cycle fixture is expensive in debug mode;
a runtime sample captured constitutive response and stress/force assembly.
This sample alone is not a general production-performance measurement.
Release qualification runs the current source for all three targets; its final
result is recorded in report.json. The original debug process was never
restarted or cancelled because of an observation timeout.

Final release qualification passed all 20 tests: 4 finite-inertia, 6 prescribed
support and 10 muscle checks. The muscle target completed in 16.00 seconds.
The final-source debug support/inertia run passed its 10 tests. The same test
assertions and physical tolerances are used in both profiles. These elapsed
times are fixture observations across builds, not a controlled production
performance benchmark. See report.json for precise run/version scope.
