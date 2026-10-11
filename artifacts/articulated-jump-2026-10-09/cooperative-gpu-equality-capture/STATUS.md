# Cooperative GPU equality encoding

The scalar backend and the independent-operator batch backend now share EqualityTask: original input preparation, GPU QR, equality decode, exact original-f64 residual products, refinement, early/late release directions and validated prefix reuse. Each pending independent task encodes into one command encoder with separate storage. One queue submission and submission-scoped wait service all ready tasks; finished tasks leave subsequent rounds. No worker threads or additional device owners are introduced.

HairLinearSolver::solve_joint_coordinates_batch has a sequential default for existing backends. GpuHairLinearSolver overrides it with the physics-owned cooperative active-set scheduler. Hints and QR prefixes remain scoped to each immutable coordinate operator. Whole output rejects on a cold task failure; malformed later input is preflighted before any equality submission. The caller still owns original physical admission.

Current-source validation on Apple M4 Max / Metal:
- Original 47-row, 1512-coordinate capture: 3 nontrivial independent variants plus one already feasible operator, 156 equality dispatches in 52 submissions. All coordinates and reactions exactly equal serial GPU results. Zero native fallbacks. The underlying scalar physical solve also passes original load admission and native comparison.
- Original 61-row, 2646-coordinate capture with one storage buffer: same independent variants, 264 equality dispatches in 88 submissions, exact serial outputs, zero native fallbacks. Original scalar physical admission/native comparison passes.
- Mixed seeded negative-reaction release and dependent-hint cold retry pass through the batch trait override; exact serial output, correct coordinate call count and fewer submissions than dispatches.
- 341 physics library tests pass, 39 ignored. Focused ordinary joint tests pass. git diff --check passes.

Submission count reduction is not an FPS result. Independent physical island response/refinement owners still execute serially in contact_square_root_projection.rs and do not yet call the batch trait method. Full 720-frame numerical equivalence and >160 rendered FPS remain open. The concurrently running same-input drift audit uses its frozen older binary; it does not include this refactor and was not restarted.
