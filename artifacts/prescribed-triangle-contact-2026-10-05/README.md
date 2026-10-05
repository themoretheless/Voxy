# Prescribed finite triangle contact in FEM dynamics

External triangle geometry is now admitted separately from the solid's volumetric mesh and material history. The immutable PrescribedTriangleSurface retains stable Arc face identity across with_positions updates. No artificial obstacle tetrahedra, extra physical mass or second integrator are introduced.

The shared internal/external triangle-minimum barrier returns potential gradients for both closest features. The existing conservative triangle advancement kernel is also shared. PotentialEvaluation gives named body/contact/actuator fields rather than extending positional tuples. The existing inertial and Maxwell/thermal step paths include the surface potential and gradient, check swept separation, independently book obstacle vertex actuator work, and atomically publish positions, velocities and obstacle pose. Material history/heat admission remains owned by the surrounding viscoelastic transaction.

Tests and evidence:
- 53 integration tests passed across prescribed surface (10), internal surface (10), moving/rotating plane (11), moving support (7), Maxwell inertia (8), fixed contact (3), finite inertia (4).
- 5 shared geometry unit tests passed; one optional historical wall-pruning timing benchmark remains ignored. This includes 10000 trajectory pruning comparisons and closest-feature lower-bound checks.
- All 15 tissue regressions passed, including the full imported clip and whole-frame/history/thermal rollback. Imported accepted/rejected/refinement receipts remain identical to prior evidence.
- Body and obstacle gradients match independent energy finite differences. Combined force and torque sum to zero; a finite triangle does not exert infinite-plane contact outside its extent.
- Imposed obstacle work defects under 0.1/0.05 mm displacement are 9.2896537634e-9 / 1.1560812787e-9 J, consistent with local trapezoidal quadrature refinement.
- In actual inertial dynamics, 1000 fixed/moving obstacle steps are Galilean equivalent. Independent per-step work matches obstacle speed times body momentum change. Moving obstacle work totals 0.0191649758461 J; accumulated energy defect is -2.3943782552e-8 J.
- Co-moving prescribed supports and triangle obstacle retain constant contact potential and opposite reaction/obstacle work, leaving the explicit pin kinetic work. The admitted obstacle getter returns the committed target geometry.
- Disjoint endpoint obstacle poses cannot tunnel through the body. Crossing, closed installation and changed-owner trials preserve the full original owner. Viscoelastic crossing rejection after initial relaxation preserves material history, heat and old obstacle pose; successful held-pose recovery releases positive heat separately from obstacle work.
- git diff --check clean for changed files.

Limits: the existing discrete triangle-minimum law is two-sided, frictionless and uncalibrated with respect to mesh refinement. The obstacle is externally prescribed, not a finite-mass dynamic body. Current cross-pair scans need a scalable spatial index for large animated surfaces; contact exclusions/masks, validated imported character binding, whole-character soft deformation, native/GPU contact visualization and realtime performance remain outstanding. No new collision visualization is claimed by these CPU fixtures.

The engine research scope now explicitly includes UE (Unreal Engine) alongside Stride3D, Unity and Godot. This records the user scope, not a completed UE source review or feature-parity claim.
