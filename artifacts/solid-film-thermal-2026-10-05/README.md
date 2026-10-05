# Owned solid/liquid thermal exchange

The solid's reference-temperature cell inventory and ThermalFilmMixture now
exchange heat through an explicit contact list in one transaction. Exact
finite-capacity pairs are applied in symmetric forward/reverse half-step order.
Dry cells insulate. Contact conductance is supplied in W/K by the caller.

Film capacity uses the existing canonical-mass capacity calculation. The solid
books the opposite of the actual representable film energy change, and nominal
versus actual transfer discrepancy consumes the declared tolerance. This avoids
creating/loss of heat when a transfer is hidden by a large film thermal baseline.

Only one film and one thermal-solid candidate are cloned per transaction.
The existing public film heat batch and the cross-owner adapter use the same
checked staged cell update, avoiding a full film clone/temperature scan per link.

Tests qualify analytic unequal-capacity temperature relaxation, conservation,
unchanged fluid inventory, dry-cell insulation, temperature bounds, temporal
refinement, invalid links and numerical-loss rollback. A separate late-failure
fixture first proves a normal contact succeeds and changes both owners, then
adds a contact whose transfer is unrepresentable and proves complete rollback.
Film thermal/vapor and viscoelastic regression targets also pass.

Automatic contact detection, physical conductance construction, default demo
hookup, full joint mechanics/film/vapor time integration and temperature-dependent
material properties remain incomplete. No new GPU render or full-goal completion
is claimed. Qualification and source hashes are recorded in report.json.
