# Canonical species mass migration

The candidate stores kilograms per species and cell. Component volumes are a
derived read-only cache; total cell volume is derived from total canonical mass
and the current common density. Mass boundary operations no longer reconstruct
canonical kilograms from volume. Legacy volume APIs convert at the owner.

Advection, pressure flow, shear flow, squeezing and diffusion transport the
same canonical mass rows. Vented species flux is already in kilograms. Thermal
capacity and carried sensible energy use M*c and M*c*T. The vapor adapter reads
canonical solvent mass rather than volume*density.

Derived geometry publication is part of staged mixture transactions. Coupled
body and sensible-energy owners are staged until that publication succeeds.
This first implementation clones transaction state; performance qualification
and removal of redundant staging remain required.

Constant density and illustrative material/vapor laws remain. This does not
enable variable-density EOS geometry, pressure-work coupling or production
liquid mechanics. Consult report/logs for actual validation, including failures.

The six primary targets passed 56 tests. Initial mixture tests had two failures
in bitwise height comparisons with the independent volume-only implementation.
Mass/density projection now uses a different arithmetic path. Those comparisons
use an explicit 8 EPS relative rounding bound, with exact equality at zero;
analytic flow, species balance, and body/report checks were retained.

Coupling investigation reproduced an unresolved capture: a 1e-30 kg particle
was successfully removed although its addition to the filled film rounded away.
The candidate guard preflights every positive volume-deposit species addition
against staged canonical mass, so legacy callers cannot silently lose particles.
The explicit kilogram interface continues to return actual receipts, including
zero for unresolved requests; callers must balance the received quantity.
Capture, rebound, atomic contact, thermal and transfer tests are now required
on this guarded source. Their execution is separate from the earlier 56 passes.

The guarded run passed 40 tests: capture 10, rebound 4, atomic contact 3,
thermal 11 and inventory transfer 12. The new direct-mass fixture deposits
123.456789 kg at density 0.1 kg/m³; the volume projection multiplied by density
differs from the request, while canonical storage and full-removal receipts
retain exactly the requested kilograms. This is a numerical fixture rather than
a calibrated water material.

Primary and guard logs refer to separately pinned revisions. Remaining work
includes variable-density EOS/pressure-work coupling, investigation of the
standalone pure-film unresolved capture boundary, and transaction performance.
