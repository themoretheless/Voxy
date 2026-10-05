# Finite-source recoil time convergence

7 emitter release controls pass. The new regression independently compares
endpoint source velocity with -u*ln(initial_mass/final_mass). Three refinements
(16/32/64 intervals) show first-order trajectory convergence while closing mass,
momentum and total kinetic+reserve energy to 1e-12 on the fixture. This exposes
interval-start source sampling as a numerical accuracy limitation. Midpoint
source-velocity sampling and rotating/aperture source dynamics remain unfinished.
