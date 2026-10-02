# Bind-preserving thumb pivot investigation

The isolated v4 candidate was tested with 22 base-joint pivot offsets. For each
hand, base local translation and inverse bind are updated, and immediate child
translation is compensated. Every candidate first asserts that all bind skin
matrices remain identity within 1e-6. Source positions, weights and authored
local rotations are unchanged. The corrective solver is bypassed. Metrics cover
121 no-object closure values on both hands.

The original pivot gives 1.89117 radians extra crease and minimum area 0.05449.
A -10 mm Y offset improves these to 1.08174 and 0.45658. Combined offsets reduce
crease further: (0,-15,+10) mm gives 0.53692 and area 0.54114. The best area in
the scan, (0,-25,+10) mm, is 0.62529 with crease 0.56410. None meets the 0.30
crease limit, and none establishes suitable anatomical articulation or contact.

The displacement of strongly tip-weighted source vertices at full grasp is
also recorded, rather than asserting unchanged tip/contact behavior. The
original maximum tip motion is 67.64 mm; these latter offsets produce 58.82
and 56.98 mm respectively. These are motion from rest, not contact error or
anatomical correctness metrics.

Pivot location materially affects the collapse, but tuning only this pivot is
insufficient. The separate middle-only articulation already produces 0.57299
radians extra crease, which remains an independent binding/articulation issue.
These numerical candidates are diagnostic and have not been promoted. The
passing tests assert finite metrics and preserved bind pose, not production
acceptance. No authored motion amplitude was reduced to pass the scan.

The generator `tools/prepare_hand_hinge_comparison.py` reproduces filters
`thumb_bind_preserving_pivot_diagnostic` and `thumb_refined_pivot_diagnostic`.
Exact offsets and results are in the accompanying JSON. A repair needs a
consistent anatomical thumb chain and binding, plus continuous shape, contact
and native visual verification; the scanned coordinate offsets are not a
substitute for that model.
