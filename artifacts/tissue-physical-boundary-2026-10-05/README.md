# Skeletal motion with a rounded physical tissue boundary

The existing body-motion demo now uses the rounded ellipsoid constructor with
level-one refinement: 32 physical tetrahedra per region, density 1000 kg/m³,
unchanged illustrative compliance parameters and stable bone pins 3 and 6.
These dimensions and presets are not an anatomical calibration.

Render triangles linearly subdivide the actual tetrahedral boundary twice.
They retain the physical dimensions and bind through EmbeddedSurface; the
previous smaller Loop shell has been replaced. Piecewise-linear physical
geometry remains visibly faceted and is not a smooth anatomical asset.

Eight tests cover boundary dimensions, affine deformation, skeleton hierarchy,
pins, secondary response, volume drift below five percent, settling and atomic
rollback. The snapshot CSV records center offsets and physical cell volumes.
Center offsets include gravity sag, not only oscillation amplitude.

```sh
cargo test -p voxy_app --lib tissue_demo::tests
cargo run -p voxy_app --example body_motion_snapshot -- artifacts/tissue-physical-boundary-2026-10-05/poses.png artifacts/tissue-physical-boundary-2026-10-05/frames
```

Final rendering measurements and source hashes are in report.json. All changes
remain local; the broader engine goal is incomplete.
