# Anatomical tetrahedra

Derived from the HRA female v1.10 GLB subsets in the parent directory. Original source/creators/CC BY 4.0 attribution are in the parent README and each JSON sidecar. These are reference anatomical shapes, not one patient's calibrated tissues. World coordinates remain in metres; no registration to the Blender character has been performed.

| Mesh | Source node | Points | Tetrahedra |
|---|---:|---:|---:|
| Left ovary | 475 | 462 | 1718 |
| Right ovary | 476 | 433 | 1588 |
| Right crystalline lens | 43 | 2759 | 12034 |
| Left crystalline lens | 70 | 2734 | 11845 |
| Anterolateral papillary muscle | 724 | 4044 | 16860 |
| Posteromedial papillary muscle | 727 | 2879 | 11890 |

Reproduce using a Python environment with `numpy` and `tetgen==0.8.3`:

```sh
python tools/export_anatomical_tetrahedra.py assets/anatomy/hra-female/ovaries.glb 475 assets/anatomy/hra-female/tetrahedra/left-ovary.vxtet
python tools/export_anatomical_tetrahedra.py assets/anatomy/hra-female/ovaries.glb 476 assets/anatomy/hra-female/tetrahedra/right-ovary.vxtet
python tools/export_anatomical_tetrahedra.py assets/anatomy/hra-female/eyes.glb 43 assets/anatomy/hra-female/tetrahedra/right-lens.vxtet
python tools/export_anatomical_tetrahedra.py assets/anatomy/hra-female/eyes.glb 70 assets/anatomy/hra-female/tetrahedra/left-lens.vxtet
cargo run --release -p physics --example anatomy_tet
cargo run --release -p physics --example anatomy_tet -- --swell
```

The constrained mesher preserves the original boundary, with refinement but no facet merging, repair, smoothing or convex-hull replacement. See sidecars for actual geometry/volume audits, source hashes and tool versions. Materials are assigned by the caller. Current examples use synthetic coefficients and numerical anchors, not experimentally fitted organ physics.

VXTM/1 is little-endian: `VXTM`, version u32, point count u32, tetrahedron count u32, boundary-face count u32; then xyz float64 points, four u32 indices per positively oriented tetrahedron and three u32 indices per outward boundary triangle. The Rust reader verifies size, indices, finite/nondegenerate positive cell geometry and exact boundary incidence/orientation. The JSON sidecar is required for scientific provenance; it is not embedded in the binary geometry.

## Cardiac material assignment

The two papillary muscle volumes retain the actual atlas boundaries. `Body::set_myocardium_batch` assigns independently specified orthotropic laws to many cells, validating the entire field before mutation and rebuilding the stiffness diagonal once. Unlisted cell histories remain intact.

```sh
python tools/export_anatomical_tetrahedra.py assets/anatomy/hra-female/heart.glb 724 assets/anatomy/hra-female/tetrahedra/papillary-anterolateral.vxtet
python tools/export_anatomical_tetrahedra.py assets/anatomy/hra-female/heart.glb 727 assets/anatomy/hra-female/tetrahedra/papillary-posteromedial.vxtet
cargo run --release -p physics --example anatomy_tet -- assets/anatomy/hra-female/tetrahedra/papillary-posteromedial.vxtet --myocardium
```

The example uses a uniform fiber direction along the longest bounding-box axis, a numerical clamp on its lower 5%, synthetic passive coefficients and prescribed activation 0.01 times nominal tension 1000 Pa. This is a mechanical diagnostic, not measured myocardial fibers, anatomical chordae/attachments or electrophysiology. The refined mesh requires more than the earlier 4000-iteration budget; the force tolerance remains 1e-7 N. Septum export exceeded the 250,000-cell import limit and produced no accepted asset. Open chamber/valve surfaces need explicit wall and cavity domains before volumetric cardiac simulation.

`--myocardium --cycle` additionally removes activation and solves relaxation, reporting the maximum displacement from the reference shape. A three-cycle single-element regression checks force convergence, recovery of reference geometry/energy and reproducible contraction. This verifies elastic cycling only; calcium kinetics and viscous cardiac hysteresis remain absent. The previous 32,000-iteration anatomical contraction run failed (residual 1.93427e-6 N versus 1e-7 N). With a mesh-size-dependent restart period (32–256), the same contraction converged in 10,356 iterations at residual 8.744993292044e-8 N, retaining the original force tolerance and mesh/material/activation. Initial length was 0.03059899806976 m and contracted length 0.03055541355105 m. Relaxation converged in 18,182 iterations at residual 8.901924082891e-8 N; maximum displacement from the original reference shape was 2.906876363314e-6 m. Force convergence alone does not establish exact reference-shape recovery on the refined mesh. This observation preceded the subsequent near-rest constitutive-energy precision correction.

Myocardial exponential energies now use `exp_m1`; the matrix invariant excess near rest is evaluated from the isochoric metric deviation using its second invariant and determinant. A regression checks the analytic quadratic shear limit at strains 1e-6, 1e-8 and 1e-10. This numerical correction does not constitute physiological calibration.

After the near-rest energy correction, the same posteromedial contraction converged in 7465 iterations at residual 8.202769082014e-8 N, with length 0.03055654056248 m and volume 4.401195674189e-6 m³. Relaxation converged in 24,261 iterations at residual 9.500824667527e-8 N; maximum reference-shape displacement was 2.821551804651e-6 m. A populated-memory regression also verifies that assigning cardiac laws does not reset neighboring viscoelastic tissue memory or advance it prematurely.

Use `--export-dir=/absolute/output/path` to write `reference.obj`, and (after converged cardiac solves) `contracted.obj` and `relaxed.obj`. Snapshots retain atlas metre coordinates and the same outward boundary connectivity, allowing inspection of actual FEM deformation. The directory is supplied explicitly; source assets are not overwritten. A round-trip check on the ovary snapshot preserved all 462 double-precision positions and 664 oriented faces exactly.

The same export option writes legacy ASCII VTK unstructured grids (`reference.vtk`, `contracted.vtk`, `relaxed.vtk`, or `swollen.vtk` for `--swell`). Fields are unsmoothed constant-strain cell data: full spatial Cauchy tensor, pressure (positive in compression), von Mises stress, current/reference volume ratio and reference cell volume. Units are Pa and metres. These diagnostic fields are not damage or tissue failure criteria. Rest-state validation checks exact cell/point geometry, complete field lengths, zero stress and summed reference volume.

## Composite organ boundaries

The exporter accepts repeated `--include-node` options for explicitly selected source boundary patches. Only exactly coincident vertices are welded. It rejects open, inconsistent, duplicate, nonmanifold or disconnected assemblies before meshing. The liver capsule (533) plus bare area (534) form a single closed oriented surface with 17,951 welded vertices and 35,898 triangles, without geometric repair. The accepted liver envelope has 37,098 points/162,807 tetrahedra and volume 0.0021381984638402943 m³, with zero measured relative volume/area discrepancies and boundary distance below 5.1e-17 m. This outer envelope does not resolve vascular/biliary lumina, internal lobular regions or capsule thickness. The initially inspected lung patch assemblies remain open and are not accepted as tissue volumes.

The right middle-lobe envelope uses nodes 918, 919, 920: 5280 points, 21,696 tetrahedra, volume 0.000213489017940728 m³. Relative volume discrepancy is zero; maximum audited boundary distance is 3.98e-17 m. The entire right lung has disconnected lobes and is rejected as one volume; separate domains are required. The upper-lobe assembly (922–925) is rejected by TetGen for source self-intersections. Left-lung split patches retain four open edges. No rejected structure has been repaired or labeled as a completed volume. Lung envelopes do not explicitly model air spaces or gas exchange.

The accepted right lower-lobe envelope (911–916) contains 23,144 points/100,650 tetrahedra, volume 0.0007138658672533887 m³, relative volume discrepancy 1.52e-16 and boundary distance below 1.68e-16 m. All three new envelopes are included in direct Rust FEM import/rest-volume/rest-stress verification; meshing preservation is separate from tissue constitutive calibration.

Spatial Darcy diagnostics on the accepted envelopes use synthetic isotropic permeability 1e-12 m², viscosity 0.001 Pa·s and prescribed pressure gradient 10,000 Pa/m, with all solid nodes fixed and external flux sealed. Liver: 304,906 internal flux faces, 80,659,776 operator bytes, 114 iterations, pressure residual 2.450781799723e-10 Pa. Right middle lobe: 40,057 faces, 10,457,800 bytes, 110 iterations, residual 2.377413821364e-10 Pa. These are pressure-driven instantaneous diagnostic flows; nonzero local divergence is not claimed to be a steady physiological perfusion state. The export directory additionally receives cell pressure/outflow/centroid velocity and oriented face-flux CSVs. Reconstructing divergence independently from the exported face inventory matched every cell outflow and pressure-work/dissipation balance on the middle-lobe mesh.
