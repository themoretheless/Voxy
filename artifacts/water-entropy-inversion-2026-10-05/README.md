# Fixed-density water entropy inversion

`water_equilibrium_from_entropy` recovers subcritical pure-water equilibrium
from density, specific entropy and a supplied temperature interval. Energy and
entropy inversion now share one bracket/admission/bisection implementation.
The energy path retains its existing tolerance and endpoint semantics.

Fixtures cover mixed and fully vapor states, exact endpoint recovery and
unbracketed/nonfinite admission. Required execution:
`cargo test -p physics --test water_equilibrium`.

This supplies a state query for reversible adiabatic volume work. It does not
apply work, update volumes, advance mechanics or implement irreversible
interfacial kinetics. See report.json for actual execution status.
