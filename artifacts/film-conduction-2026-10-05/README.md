# In-plane film heat conduction

`ThermalFilmMixture::conduct_heat(k, dt, max_step)` exchanges sensible heat along existing mesh edges without transferring liquid or species. Conductance is k times harmonic-mean film height times shared-edge length divided by the finite-volume center distance. Conductivity is supplied in W/(m K); it is not a calibrated default for water or oil.

Each pair uses the exact closed two-capacity relaxation, composed as symmetric forward/reverse half-step sweeps. This avoids explicit thermal stability limits but requires temporal refinement for multi-cell accuracy. Dry cells insulate. The entire call stages energy and publishes only on success. Substrate heating, ambient heat transfer and temperature-dependent properties remain absent. This is not yet enabled in the interactive demo.

Validation includes unequal-capacity two-cell analytic decay, heat conservation, fixed species inventory, stiff relaxation, dry insulation, isothermal invariance and invalid-control rollback. A four-cell temporal refinement fixture is also being validated. See test log and report for actual executed results.

Reproduce: `cargo test -p physics --test surface_film_thermal`.
