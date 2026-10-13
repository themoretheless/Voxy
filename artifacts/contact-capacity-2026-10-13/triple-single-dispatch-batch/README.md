# Single-dispatch triple contact batch qualification

The experimental renderer transport now dispatches one workgroup per independent complete operator in one GPU command. Each operator keeps its own header, QR workspace, scaling and candidate decoder. The immutable offset table and all original per-operator coefficients are checked before returning the whole result. A late rejected operator returns no candidate prefix.

Validation: 16 regular triple renderer tests pass; actual GPU batch test passes three heterogeneous small systems and cold/seeded copies of the complete captured 91-row, 882-coordinate operator, with a scalar system at different offsets. Duplicate results match exactly. Captured original coordinate inequalities and active-row residuals are checked with compensated products including the bound. The existing single-operator GPU factor reuse regression also passes.

A synthetic CPU fixture initially wrote unscaled output values; its second operator decoded to four instead of two. The fixture now writes values using each operator's actual state/reaction scaling. No decoder tolerance was changed.

Scope: renderer experimental transport only. App integration, native original physical admission for this batched dispatch, complete jump trajectory, isolated performance comparison and >160 rendered FPS remain unproven. Existing long full-cycle processes use earlier frozen binaries and do not qualify this change.
