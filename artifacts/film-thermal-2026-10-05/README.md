# Owned film sensible energy

`ThermalFilmMixture` owns a `FilmMixture`, constant per-species specific heat
capacities and per-cell sensible energy in joules. Mixture access is read only.
Wet-cell temperature is derived from extensive species inventories and energy;
dry cells contain zero energy and return no temperature.

Implemented operations stage all coupled state before publishing:

- Selective withdrawal carries each donor species' sensible heat.
- Advection and gravity-driven flow carry energy with the same limited donor
  volume/species flux, including filling previously dry cells.
- Species diffusion carries sensible heat at the component donor temperature.
  This is not Fourier conduction.
- Deposits add species at prescribed incoming temperatures. Mixing is weighted
  by thermal capacity. Nonrepresentable bulk/species/energy changes reject the
  batch; complete dry cells can be wetted again.
- Signed applied heat returns the actual representable net energy increment.
  The external reservoir must book its opposite. Heating dry cells or cooling a
  wet cell to an invalid temperature rejects the complete batch.

The later film/vapor adapter now connects latent heat and a finite vapor
receiver using the shared solution solver. Its separate validation is in
`../film-vapor-2026-10-05`. Viscous heating and substrate heat-transfer laws
remain absent. Complete film drying remains open. The original nonthermal
APIs remain available separately.

## Validation

- Initial owned-energy and withdrawal tests: two passed (`initial-tests.log`).
- Advection version: 25 tests passed (`advection-tests.log`).
- Diffusion version: four thermal tests and fourteen mixture regressions passed
  (`diffusion-tests.log`).
- Deposit version: all five thermal tests passed (`deposit-tests.log`).
- Applied-heat extension: all six thermal tests passed (`applied-heat-tests.log`).

`report.json` pins the source snapshot for each run. Earlier pass results do not
prove later extensions. Changes remain local; the full engine goal is incomplete.

Current command: `cargo test -p physics --test surface_film_thermal`.
