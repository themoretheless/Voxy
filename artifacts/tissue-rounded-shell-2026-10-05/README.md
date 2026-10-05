# Rounded neutral tissue render shell

The body-motion example uses three Loop subdivision steps on each closed tissue
boundary in rest space. All subdivision weights are convex combinations, so the
rounded shell remains inside the convex octahedral sample. The existing
EmbeddedSurface binds it to the same tetrahedral simulation, without outside
extrapolation, a separate motion law, or changes to solver parameters.

Subdivision shrinks the render shell. It is an inscribed visualization, not the
collision boundary and not a calibrated anatomical model. Piecewise affine
embedding can still expose derivative discontinuities across simulation cells.

Eight tissue-demo tests pass, including closed-shell binding and affine motion,
bone hierarchy, pin motion, secondary deformation, settling and atomic rollback.

Commands:

```sh
cargo test -p voxy_app --lib tissue_demo::tests
cargo run -p voxy_app --example body_motion_snapshot -- artifacts/tissue-rounded-shell-2026-10-05/poses.png artifacts/tissue-rounded-shell-2026-10-05/frames
```

Changes are local; the broader engine goal remains incomplete.
