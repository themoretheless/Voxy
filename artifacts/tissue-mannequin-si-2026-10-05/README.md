# SI-scale neutral mannequin continuum regions

Body mode no longer borrows dimensions from the abstract tissue gallery.
Four ellipsoid regions have explicit radii in metres: two (0.08,0.065,0.05),
two (0.09,0.09,0.065). Reference density remains 1000 kg/m3. Integrated
piecewise tetrahedral mass totals 4.6290386224085855 kg. Regression checks each
axis extent and mass against signed cell volume times density. These are
illustrative mannequin dimensions, not anatomical or constitutive calibration.

Three noncollinear prescribed attachment points replace the two-point hinge;
all three receive the existing bone position/velocity support transaction.
Two-point support left a rigid rotational mode which material viscosity need
not damp. Existing walk/jump/rest qualification failed at corrected scale with
two supports; with three supports it passes without relaxing its motion,
volume, heat or energy assertions. No global numerical damping introduced.

12 tissue demo tests passed, zero failed/ignored (6.01s release), including
rotation response, frame rollback/recovery, conduction independence, body
walk/jump/settle and mass/geometry. Native visual and updated recording pending.
Existing GIFs and running examples predate this change. Local uncommitted work.
Broad engine/hardware/research/liquid/realism objective remains incomplete.

Updated visual: 241 offscreen Metal frames (Apple M4 Max) over 12 seconds
at 20 fps. poses.png inspected: corrected regions appear small on the abstract
mannequin; limbs remain separate rigid forms. motion.gif assembled from actual
PNG readbacks. CSV and motion-summary.json contain measured displacement and
energy receipts. GPU validation scope clean, example exit 0. This proves the
current render/simulation path, not anatomical realism or realtime 20 fps.
