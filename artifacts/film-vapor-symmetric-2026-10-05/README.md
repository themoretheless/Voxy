# Symmetric multi-interface vapor exchange

The shared-reservoir batch now sweeps interfaces forward and backward for half
of the interval in each sweep. Every cell receives total dt while the vapor
feedback is updated between interfaces. All film inventories, cell velocities
and vapor still commit together or roll back together.

This is symmetric spatial operator splitting, not a simultaneous solver or a
global accuracy guarantee. The new refinement fixture compares forward and
reverse interface orders at 1, 2, 4 and 8 subintervals using tighter local solver
tolerances. It requires the discrepancy to reduce by more than threefold at each
halving of dt. The existing global mass/energy/momentum, residue, incompatible
species, sub-resolution flux and late-failure tests remain enabled.

The preceding sequential-batch revision passed four tests. All five tests of the symmetric revision passed (`tests.log`); Cargo exited
successfully. The refinement criterion passed for the documented two-cell
fixture. This does not qualify arbitrary meshes, material calibration or
all timestep regimes. Source hashes are in report.json.
Command: `cargo test -p physics --test surface_film_vapor`.

Complete drying, dry-cell nucleation and interactive vapor integration remain
unfinished. Physical parameters in the refinement fixture are illustrative,
not a calibration of water or oil. Changes remain local.
