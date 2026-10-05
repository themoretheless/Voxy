# Finite substrate heat exchange

`ThermalFilmMixture::exchange_substrate_heat` exchanges sensible heat with one finite lumped substrate reservoir per cell. The caller owns substrate energies (J), capacities (J/K) and contact conductances (W/K). Film and substrate publish together only after all cells validate. Dry cells insulate. Both this contact exchange and in-plane conduction share the same exact two-capacity relaxation kernel.

Tests cover independent analytic unequal-capacity heating/cooling, pair total heat, unchanged species inventory, exact-relaxation subdivision equivalence, late invalid conductance rollback for both owners and dry-cell insulation. See log/report for actual results. Conductance is supplied, not a calibrated material/contact model. Spatial heat diffusion within solids, ambient heat exchange and interactive evaporation remain unfinished.
