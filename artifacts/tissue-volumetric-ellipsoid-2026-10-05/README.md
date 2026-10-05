# Rounded physical tissue geometry

`physics::tissue::ellipsoid` builds rounded tetrahedral volumes for the existing
XPBD owner. It refines a shared octahedral boundary on the unit sphere and maps
it to the requested ellipsoid. Radial cells form an inscribed approximation.
Density times actual cell volume determines lumped nodal mass. Stable axis-node
identities support the existing bone attachment scheme; selected axes are pins.

Qualification checks analytic-volume convergence, mass conservation, axis
identity, sphere-boundary location, closed manifold topology, boundary embedding,
uniform free fall, pins and rejection of unrepresentable inputs and total mass.
Existing tissue and surface tests are also run.

This constructor does not calibrate the edge compliance against resolution and
is not yet connected to the body-motion demo. The previous rounded render shell
remains in that demo until material and attachment response are qualified with
the new physical geometry. No anatomical or production-readiness claim.

```sh
cargo test -p physics --test tissue_geometry --test tissue_surface --test tissues
```

Final-source results and hashes are recorded in report.json after verification.
All changes are local; the full engine goal remains active.
