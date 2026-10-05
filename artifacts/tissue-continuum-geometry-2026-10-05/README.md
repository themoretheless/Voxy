# Shared rounded mesh and continuum shear qualification

The ellipsoid geometry algorithm moved into the existing TetraMesh owner.
The compliant Tissue constructor delegates to it. Cells are positive and the
boundary is outward; the existing topology validator is used before publication.
The FEM and inertial FEM owners accept the same geometry and densities.

Three new tests qualify homogeneous simple-shear energy, nodal virtual work,
zero resultant, shared nodal masses and conforming refinement. The reference
volume is obtained independently from the oriented boundary surface integral.
Three prior ellipsoid tests qualify volume convergence, masses and rejection.
The existing mannequin tests are run because cell winding is now normalized.

```sh
cargo test -p physics --test tissue_continuum_geometry --test tissue_geometry
cargo test -p voxy_app --lib tissue_demo::tests
```

Initial test compilation failed because Element.volume is private. The fixture
was corrected to use an independent boundary-volume calculation, preserving the
FEM API's ownership boundary. Successful results are recorded in report.json.

Moving bone supports remain unsupported by the FEM inertial owner; the moving
mannequin has not been switched to FEM. Affine constitutive checks do not prove
full anatomical calibration or production dynamic qualification. Changes remain
local and the full engine goal remains active.
