# Subcritical pure-water equilibrium phase selection and energy flash

`water_equilibrium_at_temperature(T,rho)` uses IAPWS-95 coexistence densities to select liquid, vapor or a two-phase mixture. The mixture uses the mass-fraction specific-volume lever rule and mass-weighted internal energy/entropy. Enthalpy is u+p/rho. Outside coexistence densities, it uses the shared homogeneous EOS.

`water_equilibrium_from_energy(rho,u,[Tlo,Thi])` brackets temperature and re-evaluates equilibrium phase selection at each trial. It performs no inventory writes. Specific-energy tolerance is 1e-6 J/kg + 1e-11 relative. Tests cover fractions 0, .1, .5, .9, 1; volume/energy/entropy identities; homogeneous endpoints; and fixed-density heating from a two-phase state to full vapor followed by energy inversion. Actual execution is tracked in report.json.

This is equilibrium thermodynamics, not finite-rate interface mass transfer. Ice/supercritical states, exact critical singularity, surface energy and particle/film/drying demo integration remain unfinished. No automatic removal of residual liquid mass is introduced.
