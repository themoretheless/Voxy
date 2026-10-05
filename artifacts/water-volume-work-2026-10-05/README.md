# Reversible equilibrium water volume work

`change_water_volume_adiabatically` computes the equilibrium state at a supplied
new volume and the initial specific entropy using IAPWS. The internal-energy
change is balanced against an externally owned, nonnegative mechanical work
store. Volume and both energy stores commit together after final energy/entropy
admission. Compression needs an available mechanical budget; unsupported EOS
states and unresolvable work reject without mutation. Contact heat and volume
work share the same representable energy receipt helper.

Fixtures cover expansion/cooling, compression/recovery, energy and entropy
balance, budget/domain failure rollback, and the independent thermodynamic
identity `(dU/dV)_S = -p` through a central finite-volume difference.
Required test: `cargo test -p physics --test water_equilibrium`.

This is reversible quasistatic work for a prescribed volume of a closed pure-water
mass. It does not integrate a piston, SPH forces, shock dissipation or finite-rate
interface transfer. Ice/supercritical/global-domain qualification and demo
integration remain unfinished. See report.json for actual executed status.
