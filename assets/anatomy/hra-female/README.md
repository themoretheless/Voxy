# Female reference organ assembly

Source: **3D Reference Organ Set for Female, v1.10**, Kristen Browne and Heidi Schlehlein, HuBMAP / Human Reference Atlas (2026). [DOI](https://doi.org/10.48539/HBM637.DWBM.744), [dataset](https://lod.humanatlas.io/ref-organ/united-female/v1.10), [official metadata](https://cdn.humanatlas.io/digital-objects/ref-organ/united-female/v1.10/metadata.json).

License: [Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/), explicitly recorded in the official dataset metadata. Based on the National Library of Medicine Visible Human Dataset; the brain group also carries Allen brain anatomical identifiers in the source assembly.

Adaptations: selected twenty-two anatomical group subtrees; baked source node transforms into the original atlas world frame; retained original triangles and semantic metadata; packed position/index buffers and assigned display colors. Positions are rounded to binary32 after baking transforms. No remeshing or anatomical repair is performed. Original node names, ontology identifiers, source node indices, topology diagnostics and SHA-256 hashes are preserved in `manifest.json`. GLB primitive extras retain per-structure metadata.

Reproduction:

```sh
curl -L https://cdn.humanatlas.io/digital-objects/ref-organ/united-female/v1.10/assets/3d-vh-f-united.glb -o /tmp/hra-female.glb
python3 tools/export_female_organs.py /tmp/hra-female.glb assets/anatomy/hra-female
cargo run --release -p voxy_app --example organs_render
```

The render example loads seven organ GLBs by default; `--all` loads all twenty-two groups through Voxy's importer, reconstructs display normals, and writes `/tmp/voxy-organs-preview.png` with two camera views. Its centering/scaling is for visualization only. Asset geometry remains in the source coordinate frame. It is **not registered to the Blender mannequin**.

## Geometry audit

Counts are per source primitive after exact-position welding. Overlapping segment/surface annotations are retained; these are not watertight organ volumes.

| Group | Structures | Triangles | Boundary edges | Nonmanifold edges | Closed oriented edge-manifold primitives |
|---|---:|---:|---:|---:|---:|
| Heart | 14 | 85,914 | 1,236 | 0 | 5 / 14 |
| Liver | 26 | 93,303 | 2,895 | 0 | 0 / 26 |
| Lungs | 27 | 36,764 | 2,596 | 0 | 0 / 27 |
| Brain | 283 | 656,268 | 178 | 8,067 | 227 / 283 |
| Small intestine | 9 | 43,752 | 504 | 0 | 1 / 9 |
| Colon | 10 | 20,421 | 843 | 0 | 0 / 10 |
| Partial skeleton | 81 | 1,102,345 | 237 | 2 | 78 / 81 |

Closed edge incidence alone does not prove absence of self-intersection, correct orientation, positive volume, or suitability for tetrahedralization. The audit does not detect distinct overlapping primitives. For FEM these assets need an explicit anatomical region/outer-wall reconstruction, cavity definitions and a validated volume mesh.

This is a reference assembly, not a complete skeleton or a single-person scan. Anal sphincters are absent; the small-intestine group contains the hepatopancreatic sphincter, which must not be substituted for anal anatomy. These assets supply geometry only; they do not supply measured tissue properties, physiological activity, attachment conditions or organ contact.

## Extended groups

The reproducible export now also includes:

- `eyes`, `optic-nerves`, `spinal-cord`, `teeth`.
- `vasculature-partial`: regional blood vessels, not a complete vascular graph.
- `lymphatic-organs`: spleen, thymus and reference lymph-node geometry; no complete lymphatic vessel network.
- `muscles-partial`: eye and knee muscles only.
- `subcutaneous-fat-partial`: three sampled abdominal regions.
- `visceral-fat-partial`: omentum and epiploic-appendage regions.

All structure-level edge audits and source IDs are recorded in the manifest. These additions do not change the incomplete anatomical coverage described above. Blood/lymph fluids and neural signaling are not supplied by the atlas surfaces.

Use `python3 tools/export_female_organs.py source.glb output --groups eyes teeth` for selected groups. Selected-group export preserves a prior manifest only if it has the same source hash. Rendering selected groups: `cargo run --release -p voxy_app --example organs_render -- eyes teeth`; render all groups with `--all`.

## Mammary and reproductive anatomy

Six additional groups bring the export to 22:

- `mammary-glands`: bilateral interlobar adipose, lobes, main lactiferous ducts/sinuses, suspensory ligaments, nipples and areolar structures, with original anatomical labels.
- `vagina`, `uterus`, `ovaries`, `fallopian-tubes`, `reproductive-ligaments`.

Pregnancy/placental reference structures are not included in these groups. Coverage is not complete external genital anatomy. These are reference surfaces; they do not yet define validated breast/reproductive FEM volumes, tissue parameters or registration inside the animated Blender mannequin. The mammary geometry includes regional breast fat and suspensory ligament surfaces, not a calibrated breast biomechanics model.
