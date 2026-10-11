# Exact physical contact aliases

Old row deduplication merged Jacobians whose gradient distance was <= 1e-12. Two normals [1,0,0] and [1,5e-13,0] at free velocity [0,-10,0] demonstrate why this is not safe: deleting the second inequality leaves a -5e-12 original residual although the requested tolerance is 1e-14. The regression fails on the old implementation with original-inequality violation.

Only equal canonical gradients and matching rod/point/mobility now alias. Near-parallel rows retain independent owners. Exact aliases still retain the tighter bound. No convergence tolerances, contact radius, material parameters or physical rows are softened. This removes a reproduced row-loss defect; its contribution to the historical full-jump drift is not established. The currently running velocity-input capture uses the frozen pre-fix binary and must not be described as validating this correction.

## Tangent-model row ownership

A second reproduced defect was in `ContactModel::observe_rejected_candidate`: endpoint Jacobians within 1e-12 shared one model row. The regression isolates this owner using two explicitly installed rows and an independent point admitted by one plane but excluded by the other. It fails before correction and passes after requiring equal endpoint Jacobians. Existing stricter-offset and candidate-observation thresholds remain unchanged. Both model-specific tests pass. The saved 21/21 integration and small physical GPU scene results apply to the first velocity-contact alias fix, before this additional model change; do not extrapolate them to the latest source. The geometric contact recorder has a separate approximate identity rule that is still under review.

## Geometric recorder

A third regression shows the old recorder replacing a normal [1,0,0] with a nearby unit normal rotated approximately 1e-7 rad. A centimetre-scale independent probe violates the lost original plane by 5e-10 m but is admitted by its replacement. The pre-fix test fails with explicit lost-plane admission. Recorder aliases now require equal canonical segment fractions, equal normals, equal plane offsets and unit physical metric. Tangentially shifted witnesses of exactly the same plane still alias; scaled physical envelope witnesses remain separate. Shared endpoint canonicalization is unchanged. This is a reproduced contact-recording defect, not proof that the complete jump has been corrected.
