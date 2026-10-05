# Ordinary-water saturation correlations

Primary reference: IAPWS SR1-86(1992), https://iapws.org/technical-guidance/release/Supp-sat . Downloaded source PDF is pinned by SHA-256 in report.json. Scanned PDF pages 3, 4, 5 and 7 were visually inspected after extracting their embedded images and correcting PDF image orientation; pdftotext returned only the title.

`liquid::water_saturation(T)` implements pressure and analytic derivative, saturated liquid/vapor densities and specific enthalpies (equations 1–4, 6, 7). Internal energy is h-p/rho; latent enthalpy is the vapor/liquid enthalpy difference. SI units, ITS-90 temperatures, triple-to-critical domain 273.16–647.096 K inclusive. Published Table 1 supplies independent verification values at 273.16, 373.1243 and 647.096 K. Test tolerances reflect the table precision; derivative and Clapeyron consistency are checked at six additional temperatures.

Current finite-cell vapor integrator still requires liquid cp = vapor cv + R for its constant-latent reference. This new saturation model is not yet integrated into that solver. Saturation correlations alone do not provide off-saturation vapor thermodynamics, full drying, dry nucleation or an arbitrary-pressure water EOS. These require a consistent general energy/state model and conservative boundary transfer. No real-water evaporation demo claim is made.

Reproduce: `cargo test -p physics --test water_saturation`. See report/log for actual validation status.
