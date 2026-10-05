# Imported attachment contact domains — partial qualification

The CesiumMan snapshot example's --contact path now authors one reference-space attachment domain per FEM pad. Centers reuse the same source bone/reference palette used for supports. Fixed metre half-extents are [0.11,0.10,0.10] for the front regions and [0.12,0.12,0.11] for the rear regions. Source triangles whose AABB overlaps these authored attachment boxes are excluded for that region. The mask is never recomputed from animation or current intersections.

Excluded source triangle counts: 32, 115, 132, 157 of 4672. All remaining triangles participate in force and continuous contact admission. Each regional mask retains original source vertex and face indexing.

Initial binding succeeds. Extraction/domain regression passed: source topology and exact displayed world positions match at phases 0,.25,.5,.75,1; masks persist across poses; the unmasked overlapping setup rejects and the authored setup binds.

The actual Apple M4 Max Metal animation run FAILED with `finite-deformation inertial support work defect`. Five partial frames were written at 0,.05,.10,.15,.20 seconds. No complete two-second clip, successful contact GIF or production-ready behavior is claimed. Frames are partial diagnostic output. The energy admission tolerance and subdivision limit were not weakened.

Next: diagnose the failing moving-contact energy/work quadrature with its actual closest-feature transitions and refinement history; validate or improve the authored region boundary if evidence requires it. Attachment boxes are illustrative authoring, not physiological calibration or a complete deformable surface replacement.

Reproduction identifies rejected frame 52, t=0.216666667 s. Last committed regional maximum depths were 5,5,9,6. Last committed cumulative numerical defects were approximately -2.30e-6,-3.09e-6,-3.04e-6,-4.52e-6 J. These receipts describe the admitted prefix, not the rejected trial. The .20-second partial frame was visually inspected and shows the textured imported character with four cyan FEM regions.
