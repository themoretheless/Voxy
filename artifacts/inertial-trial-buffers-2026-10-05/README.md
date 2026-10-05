# Inertial trial buffers

The frozen-history support step now stages positions and velocities instead of cloning the complete InertialBody. Trial force/contact evaluation and diagnostics take those explicit buffers; topology, constitutive history, masses, and thermal storage remain borrowed. Both buffers publish only after all path, work and diagnostic guards pass. The surrounding viscoelastic step still owns the full transaction for relaxing histories and depositing heat.

Verification:
- 19 targeted physics tests passed (7 moving supports, 8 viscoelastic inertia, 4 finite inertia).
- Added an analytic plane-contact regression: three prescribed boundary nodes enter the penalty halfspace, and independent quadratic contact energy matches actuator reaction work and total energy change.
- All 14 tissue tests passed, including full imported clip, conduction, failed-frame and failed-second-child rollback/recovery.
- The same full imported 480-frame clip passed separately in 36.35 seconds vs the previous single-test run's 42.77 seconds. Accepted/rejected/deepest-refinement receipts are identical. This is one before/after pair, not a robust general performance estimate.
- Existing 1000-step benchmark, three repeats before and after: medians 0.022285375 and 0.020143208 seconds (9.61 percent reduction). All printed positions, velocities, energies, heat, work and defects match exactly. These are rounded printed records, not a hash of the full internal state.
- git diff --check clean for modified files.

Limits: the two-second imported animation remains far from realtime. No GPU or native visual verification was performed for this CPU state-staging change. No material parameters, timestep targets, energy tolerances, or geometric admission guards were changed.
