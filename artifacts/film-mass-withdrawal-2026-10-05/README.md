# Film evaporation mass boundary

`FilmMixture::withdraw_component_masses_batch` accepts kilograms per species and
cell. The evaporation adapter now sends its solvent mass transfer through this
boundary. Conversion to the current constant-density volume inventory occurs
inside the mixture owner. Repeated requests consume staged availability; late
errors leave the complete mixture unchanged. Full depletion selects the stored
volume directly to avoid a mass/density round trip exceeding availability.

The receipt reports actual representable transferred mass and volume, not the
requested amount. Existing bulk/species balance and underflow guards still run
before publication. Destination gas/heat stores remain staged by the adapter.

This is an interface migration, not canonical mass storage or variable-density
mechanics. Condensation still enters through the volume deposit API. The film
and its vapor kinetics still use the existing illustrative material model;
IAPWS-95 is not enabled for these cells by this change.

Required checks:

```sh
cargo test -p physics --test surface_film_withdrawal --test surface_film_vapor
```

See the report for executed results and source hashes. Changes remain local.

Executed results: 9 vapor-coupling tests and 8 withdrawal tests passed. The
round-trip regression uses volume 0.1 m³ and density 0.1 kg/m³ as a numerical
fixture, not a physical water material. The old volume request overdraws because
`(volume*density)/density > volume`; the mass interface removes exactly the
stored inventory and returns its actual mass. Production source was unchanged
between the vapor run and the final withdrawal run.
