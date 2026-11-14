`triple_alpha_fy05.reaclib` contains the three forward fy05 triple-alpha records
extracted from https://raw.githubusercontent.com/pynucastro/pynucastro/main/pynucastro/data/reaclib_default2_20250330
on 2026-10-01. Original header and coefficient lines are preserved; chapter 8
markers identify three-reactant capture. This fixture supplies numerical data,
not a validity interval; the example only evaluates its held 2e8 K point.

`helium_structure_seed.csv` is an experimental four-shell numerical starting
profile at radius 1e8 m, produced by an earlier structure solver. It is not an
equilibrium oracle, measured star, or calibration dataset. The physical
free-free regression conservatively remaps it into eight shells, solves the
current equations again, and independently checks the resulting balance.
