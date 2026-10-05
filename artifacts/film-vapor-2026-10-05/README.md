# Film / finite-vapor exchange

The thermal film adapter uses the existing adaptive solution vapor integrator.
That internal shared step now validates its reservoir state and control inputs.
The particle-liquid entrypoint continues to use the same step.

Film composition/volume/sensible energy, externally owned lumped cell velocity
and the finite pure-vapor cell commit together. The adapter checks mass, energy
(including vapor latent reference and bulk kinetic energy), and momentum before
publishing. Only the configured solvent transfers; nonvolatile inventory stays.

The prescribed interface area and the Clausius-Clapeyron/Hertz-Knudsen ideal
solution model retain the original liquid solver limitations. Solvent specific
heat must equal vapor cv plus the gas constant. Complete solvent exhaustion,
dry-cell nucleation, film mechanical motion, surface recoil and automatic exposed
area detection remain open. This is not qualification of complete drying.

Tests cover evaporative cooling, evaporation and condensation at different vapor
pressures, nonvolatile residue, mass/energy/momentum, and rollback of all three
owners. Existing particle evaporation and six thermal film tests run alongside.
No result is claimed until the current process finishes.

Command:
`cargo test -p physics --test surface_film_vapor --test surface_film_thermal --test liquid_evaporation`

Source hashes and the active process handle are in report.json. Changes are local.

Intermediate results: all 11 particle evaporation tests and six thermal tests
passed. The film adapter tests are still running. A subsequent source review
added a bulk-volume mass check and guards against nonfinite momentum tolerance
scale and species changes below bulk resolution; these final adapter changes
need a follow-up run after the current process finishes.

Initial validation completed: 19 tests passed (11 particle evaporation, six
thermal film and two film/vapor adapter tests). Final adapter guards plus a
third regression for flux below donor inventory resolution compiled; their
test process is running. `initial-tests.log` records the completed run.

Final validation completed: all three film adapter tests passed
(`final-tests.log`). The verified final source hashes are recorded in
report.json. No GPU demo integration or complete-drying claim follows from
these CPU tests. The active full engine objective remains incomplete.
