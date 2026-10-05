# IAPWS-95 homogeneous water evaluator

Attributed source: International Association for the Properties of Water and Steam, IAPWS R6-95(2018), https://iapws.org/technical-guidance/release/IAPWS-95 . Coefficients and source checksum: `docs/engine-research/mechanisms/water-thermodynamics/iapws95-coefficients.json`.

`water_homogeneous_state(T,rho)` uses all ideal and 56 residual terms. Private second-order two-variable forward differentiation computes one consistent Helmholtz potential and derivatives for pressure, internal energy, enthalpy, entropy, cv, cp and sound speed. No runtime coefficient parsing or additional owner for thermal inventory is introduced. Generated fixed coefficient arrays mirror the pinned resource.

Validation targets all eleven official Table 7 single-phase states (liquid, vapor, near-critical), finite-difference caloric derivative against cv, the h=u+p/rho identity and invalid-domain rejection. The same run covers the four saturation tests, including pressure inversion. See actual logs/report; pending tests are not completion evidence.

Temperature envelope is 273.16–1273 K and pressure must be positive and at most 1 GPa. Nonpositive local stiffness/cv and nonfinite properties are rejected. This does not select global stable phases or check the melting curve; positive local stability can include metastability. Exact critical singularity is explicitly unsupported. Maxwell coexistence selection, energy inversion, arbitrary mixture/water transport and integration into evaporation/film demo remain unfinished. This is not a full production water simulation qualification.
