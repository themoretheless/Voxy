# Imported rig tissue support admission reproduction

Actual pinned CesiumMan asset sampled through ModelAsset. Named torso and hip
joints are assigned to existing mannequin support groups. Relative bone palettes
are formed against imported phase 0, then passed to existing continuum supports.
Accepted steps check exact pin targets and finite embedded boundary geometry.
At step 11 of the attempted phase sequence the existing mechanical work defect
admission rejects. Full debug state before/after rejection is equal: earlier
accepted regions in the candidate do not leak into the published demo.

Regression captures this rejection rather than widening numerical admission.
13 tissue tests passed, zero failed/ignored. This does NOT prove successful
character/tissue integration: arbitrary mannequin anchors/material frames have
not been mapped into the imported character. Need correctly placed material
reference, bone attachment frames and qualified prescribed-motion integration.
No character surface FEM coupling or anatomical calibration is claimed.
Broad objective remains active; local uncommitted changes.
