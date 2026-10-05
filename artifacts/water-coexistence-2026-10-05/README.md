# IAPWS-95 seeded liquid/vapor coexistence

`water_coexistence(T)` solves equal pressure and chemical potential using the same IAPWS-95 Helmholtz evaluator on both branches. SR1 densities are initial guesses only. Newton iterations use logarithmic densities, analytic thermodynamic slopes, damping, local positive stiffness/cv checks and rejection of collapsed equal-density roots. Slightly negative trial liquid pressure is permitted internally because approximate seed densities can create it; published equilibrium states require positive pressure.

Residual bounds: pressure 1e-4 Pa + 1e-8 relative; chemical potential 1e-10 R T J/kg. Official IAPWS R6-95(2018), Table 8 at 275, 450, 625 K is the independent validation fixture. Tests check pressure, both densities, both enthalpies/entropies and coexistence residuals. Inspect report/log for actual execution.

Exact critical endpoint, ice coexistence, global phase selection among arbitrary phases, energy/volume flash and evaporation transport are unfinished. Close to critical, the Newton system may reject poor conditioning. This is a seeded liquid/vapor equilibrium solver, not a universal equilibrium qualification.
