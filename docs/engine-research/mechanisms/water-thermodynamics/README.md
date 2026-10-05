# Water thermodynamic model ownership

Source: International Association for the Properties of Water and Steam, IAPWS R6-95(2018), Revised Release on the IAPWS Formulation 1995 for the Thermodynamic Properties of Ordinary Water Substance for General and Scientific Use (2018), https://iapws.org/technical-guidance/release/IAPWS-95 . Official PDF checksum and coefficient transcription are pinned in `iapws95-coefficients.json`.

Use one Helmholtz free-energy evaluator for water liquid and vapor. Pressure, internal energy, enthalpy, heat capacities and sound speed must derive from that same potential and its derivatives. Avoid separate fitted pressure/energy models with incompatible references. The existing SR1 saturation evaluator remains a coexistence correlation/initial guess and verification resource; it is not a general water EOS.

The coefficient resource includes all 8 ideal coefficients, 5 ideal exponential parameters and all 56 residual terms (51 power/exponential, 3 Gaussian, 2 nonanalytic). Parsing checks complete ordered indices 1–51; the remaining coefficients are transcribed from the official table. The homogeneous evaluator and its derivatives passed the selected official Table 6/7 checks in `artifacts/water-helmholtz-2026-10-05`. The subcritical coexistence solver passed the selected Table 8 checks in `artifacts/water-coexistence-2026-10-05`. Fixed-density equilibrium energy inversion and prescribed finite-reservoir heat transfer have separate qualification artifacts. This evidence does not establish the full stable-fluid domain, finite-rate evaporation, moving-volume pressure work or liquid/film/demo integration.

Acceptance must include official Table 6 potential derivatives at T=500 K, rho=838.025 kg/m3; all Table 7 liquid, vapor and near-critical states; and Table 8 saturation equilibrium values obtained from Maxwell equality. Exact critical singularities, metastable/unstable states, coexistence selection, temperature inversion and the published stable-fluid domain need explicit handling. Only then migrate conservative liquid/film/vapor energy inventory and boundary transfers, preserving rollback and checking mass, momentum, total energy and temporal convergence.

### Integration boundary verified in the current source

`crates/physics/src/liquid/evaporation.rs` currently computes liquid energy as
`State::capacity() * temperature`, vapor energy as `m * (cv*T + latent_heat)`
and decodes both by division by constant capacities in `transfer`. Its admission
also requires liquid cp = vapor cv + R. The particle and film adapters share this
state and adaptive midpoint transfer; a separate water-specific stepping loop
would duplicate the conservation and rollback owner.

IAPWS integration must make caloric evaluation/inversion an explicit constitutive
boundary in that shared transfer path. Stored internal energy, transported
enthalpy and mechanical boundary work must have distinct meanings. Fixed-volume
contact qualification does not establish moving-interface pressure work. Before
publishing a water mass transfer, define which owner supplies phase volume,
where displaced-volume work goes, which branch admits the receiving vapor, and
how an empty vapor cell is initialized. Temperature-dependent latent heat alone
cannot make the constant-capacity decoder consistent with the Helmholtz model.

Admission gates for that integration are joint mass/momentum/internal-plus-kinetic
energy accounting including the work owner, recovery of both temperatures from
the same EOS reference, evaporation and condensation direction, temporal
refinement, and rollback after a failure in the final receiver decode. Full
drying additionally needs a zero-liquid-mass boundary instead of the current
positive-residual-mass admission. These remain implementation requirements,
not claims that the current contact model already covers interface transfer.


The film integration also needs an inventory boundary change. In the current
`surface_film_mixture.rs`, component mass is component volume times one configured
constant density; `surface_film_thermal.rs` uses the same conversion for capacity.
A variable-density EOS cannot be substituted into those multiplications without
changing component mass during heating. Component mass must remain the inventory
coordinate and geometric volume must be derived from the selected EOS state.
Constant-density compatibility must preserve existing transfer results, while
water thermal expansion updates geometric volume with explicitly accounted
boundary work. Liquid/film/vapor transfers need staged component masses, momentum
and internal energy in the same reference. This migration and its independent
thermal-expansion/transfer acceptance remain implementation requirements.
