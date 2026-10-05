# Shared interface time controller

The existing evaporation adaptive midpoint step-doubling controller is extracted
into `adaptive_midpoint<S: Copy>`. Particle and film adapters continue through
the existing `advance` admission and constant-capacity constitutive trial.
The generic controller owns time adaptation and accepted candidate values;
constitutive models supply trial admission and normalized local error. This
prepares reuse for IAPWS caloric states without another stepping/rollback loop.
Nonfinite or negative normalized errors reject the trial.

This does not implement IAPWS mass transfer, changing-volume work or full drying.
Required validation after the current contact test process completes:
`cargo test -p physics --test liquid_evaporation --test surface_film_vapor`.
See report.json for executed status; an edited controller is not test evidence.
