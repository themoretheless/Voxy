# Atomic multi-cell film / vapor exchange

`exchange_solution_vapor_batch` stages all film cells, externally owned cell
velocities and one shared vapor reservoir. A late invalid cell rolls back the
complete batch. Duplicate cells and incompatible volatile species/reference
models reject the batch. Models are borrowed; cells can share immutable model
resources while prescribing different exposed areas and accommodation values.

Requests execute in caller order for the supplied interval. This is spatial
operator splitting; it is not a simultaneous multi-interface integrator. Local
single-interface accuracy controls do not bound splitting error. Coupled accuracy
still needs timestep refinement. Complete drying and dry-cell nucleation remain
unfinished, and the interactive impact demo has not connected vapor exchange.

The initial four-test run passed (`initial-tests.log`). Final tests add global
mass, energy and momentum accounting for two cells sharing vapor, alongside
ordered-reference equivalence, nonvolatile residue, late-failure rollback and
incompatible-model rejection. The final source revision also borrows models.
This final revision is currently being compiled/tested; no result is claimed yet.

Command: `cargo test -p physics --test surface_film_vapor`.
Changes remain local. The full engine objective remains incomplete.

The final sequential-batch revision passed all four tests, including global
accounting (`batch-tests.log`). A subsequent symmetric half-sweep revision
is under validation in `../film-vapor-symmetric-2026-10-05`; the sequential
ordering described above is the preceding source snapshot.
