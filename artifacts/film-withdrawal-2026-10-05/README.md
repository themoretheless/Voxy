# Selective film component transfer

`FilmMixture::withdraw_components_batch` stages the complete batch before
publishing either the per-cell bulk volume or species inventories. Repeated
cell requests spend the same staged stock. The returned `FilmWithdrawal`
contains actual representable bulk and species volumes and masses in SI units.
A receiver must credit these quantities and stage its own transaction.

Nonvolatile species can remain while a selected solvent is withdrawn. Complete
withdrawal leaves exactly dry cells that can be wetted again. Bulk and species
balances agree at floating-point roundoff scale; they are not bitwise identical.
Transfers below species resolution return zero. A representable species change
below bulk resolution, or a mass underflow, rejects the batch without mutation.

Six regression tests cover selective removal, residue, dry/rewet behavior,
repeated requests, late invalid requests, signed zero, sub-ULP requests, mass
underflow and a ghost parcel below bulk resolution. Existing mixture and film
advection suites are run alongside them.

This is the inventory-transfer foundation for drying. Film temperature,
enthalpy, evaporation kinetics, latent heat and coupling to a vapor receiver
remain unimplemented here. The existing liquid vapor solver cannot be connected
by simply deleting film volume: the film needs a consistent thermal state first.

Validation command:

```sh
cargo test -p physics --test surface_film_withdrawal --test surface_film_mixture --test surface_film_advection
```

Final validation: all 22 tests passed; Cargo exited successfully. `tests.log`
contains the complete run and `report.json` pins the verified source hashes.
Changes remain local.
