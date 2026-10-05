# Bulk-volume material and unbiased static load

The existing Tissue owner accepts a physical bulk modulus K in Pa. Per-cell
volume compliance is |V_rest|/K, so volumetric energy scales with cell volume.
None restores legacy uniform compliance; invalid configuration is atomic.
The body-motion demo selects an illustrative 1 MPa bulk modulus.

Velocity damping now decays stored momentum before adding external acceleration.
This removes acceleration attenuation and the associated static-load bias.
The existing edge-load test was tightened from 7 mm to 0.1 mm. The new pressure
fixture checks strain p/K at two sizes and two time steps, with a pinned base
and consistent generalized pressure load at the apex. It isolates the bulk
term and does not qualify the entire edge-based constitutive material.

Commands:

```sh
cargo test -p physics --test tissue_bulk_modulus --test tissues --test tissue_geometry --test tissue_surface
cargo test -p voxy_app --lib tissue_demo::tests
```

No anatomical calibration or full production qualification. The broader engine
goal remains incomplete, and changes remain local. Results are recorded after
process completion in report.json.
