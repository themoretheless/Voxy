# Full jump after exact contact alias corrections

All 469 guides and original 720-frame accuracy gates. Three reproduced row-loss defects were corrected without changing physical tolerances: constraint gradient aliases, rejected-candidate model endpoint Jacobians and geometric recorder plane identity. Prelaunch unit tests: 348 passed / 39 ignored; hair integration: 21 passed; real GPU small physical scene: exact serial poses/orientations and zero fallback.

Velocity island snapshots target rod 227 with tolerance >= 1e-9, limit 1024 per backend. New sidecars record ordered global rod IDs; indices across backend files still do not establish row identity. The separate pre-fix velocity-input-capture process is preserved. This run is not a rendered FPS measurement. Read qualification.log and process state for actual progress; launch alone does not qualify the trajectory.
