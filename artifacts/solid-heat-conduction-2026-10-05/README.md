# Shared finite-capacity heat transfer for solid cells and liquid films

The common transfer law is implemented once in heat_exchange.rs. Existing film
conduction calls it through its inventory adapter. InertialBody's cell thermal
inventory now supports signed energy increments and conservative exchange across
explicit conductance links. Forward/reverse half-step ordering gives a symmetric
closed network update. A candidate thermal owner publishes only after every
link and the accumulated absolute numerical defect pass admission.

Tests qualify the analytic two-cell temperature decay, conservation, entropy
increase, temperature bounds, second-order time refinement and rollback after a
successful first link followed by an overflowing second link. The shared scalar
law preserves representable heat when an intermediate energy overflows or the
relaxation fraction underflows. Film thermal and vapor regression targets are
also checked.

The first rollback fixture incorrectly assumed an arbitrarily small energy
budget must fail; compensated accumulation produced an exact zero defect, so
admission was correct. The final fixture instead proves a successful first link
and an actual later numerical overflow, then verifies complete owner rollback.

Conductances must be supplied in W/K from a physical geometry/material/contact
model. Automatic FEM face conductance construction, solid/liquid joint ownership,
demo hookup and temperature-dependent materials remain incomplete. No new GPU
render or full-thread goal completion is claimed.
