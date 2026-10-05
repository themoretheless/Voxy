# Neutral bone-driven tissue demo: surface lighting

The existing body-motion example now lights each deformed tissue triangle using
its actual solved normal. Outward orientation is corrected against the solved
center because the tetrahedral boundary has mixed triangle winding. Front and
rear views use the same light direction as the mannequin.

`frames/secondary-motion.csv` records the free center's world-space offset from
the corresponding bone's rigid transform, in metres. This includes gravity sag
and dynamic response; it is not a measure of oscillation amplitude alone.

The solver, attachment layout and material parameters are unchanged. Geometry
remains a coarse tetrahedral sample, not a calibrated anatomical model.

Reproduce with:

```sh
cargo test -p voxy_app --lib tissue_demo::tests
cargo run -p voxy_app --example body_motion_snapshot -- artifacts/tissue-surface-lighting-2026-10-05/poses.png artifacts/tissue-surface-lighting-2026-10-05/frames
```

Verification results and source hashes are recorded in `report.json` after the
test and rendering processes finish. Changes remain local.
