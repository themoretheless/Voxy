# Finite water contact conduction

`exchange_water_contact_heat` solves q = G dt (T_reservoir_after - T_water_after), with the IAPWS equilibrium decoder for water and constant finite reservoir capacity. Water mass and volume are fixed. Contact parameters borrow the existing energy owners; no second inventory system is introduced. Admission checks heat budgets and both stores publish through the existing prescribed-transfer transaction.

The monotone implicit heat root is extracted from the existing liquid latent-phase conduction path into one shared pure solver. Legacy phase conduction and water contact now use the same root kernel. Original phase mechanics, energy reference and constitutive models remain unchanged; targeted phase/transport tests are part of validation.

New fixtures check heating and cooling, the implicit heat law, total energy, monotone temperatures/no overshoot, invalid-control rollback, along with existing water full-vapor and sub-resolution transactions. See report/log for actual executed status. The conductance and solid capacity must be supplied/calibrated. Backward Euler is first-order in time; accuracy requires refinement, and no production performance claim is made. Finite-rate liquid/vapor interface kinetics, mechanical motion, pressure work for changing volumes and interactive demo integration remain unfinished.

The initial contact validation completed successfully: 13 legacy phase tests,
7 legacy transport tests and 4 water equilibrium/contact tests (`tests.log`).
These results apply to the original source hashes in `report.json`. Subsequent
kernel guards and temporal convergence checks have separate execution status
in the report; the initial pass does not validate those later edits.
