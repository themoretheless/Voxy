# Homogeneous water internal-energy inversion

`water_homogeneous_from_energy(rho,u,[Tlo,Thi])` recovers temperature and the final homogeneous state from density and specific internal energy. It uses the same IAPWS-95 evaluator and cv in a bracketed safeguarded Newton solve. The caller selects a homogeneous phase branch; sampled invalid/locally unstable states or energies outside the bracket are rejected. No inventory is modified.

Energy residual tolerance is 1e-7 J/kg + 1e-12 times absolute requested specific energy. Tests cover liquid and vapor round trips, endpoint handling, recovered pressure and energy, invalid temperature brackets and unbracketed energy. Actual execution is tracked in report.json. This does not perform global phase selection, a two-phase energy/volume flash or evaporation transport integration.
