# Original breed studies — work in progress

These six stationary neutral studies are a new geometry path, independent of
`voxy_app::animals`. They are **not finished realistic animals** and are not
installed as the gameplay companions. Fur, skin deformation, facial detail,
contact physics and visual acceptance remain outstanding.

The first set contains Labrador, German Shepherd, Siberian Husky, domestic
shorthair, British Shorthair and Maine Coon. Domestic shorthair describes a
population type, not a pedigree breed. Each profile changes geometry; all use
the same neutral material. Dimensions are metres, with authored trunk, skull,
muzzle, ear, limb and tail sections. Components currently intersect; a unified
surface and eyelids, nose, lips, digits and ear conchae still need sculpting.

Export:

```sh
cargo run --release -p voxy_model --example export_animal_studies -- artifacts/animal-studies
# Existing native viewer:
target/release/voxy_app --model artifacts/animal-studies/gallery.glb
```

The gallery retains real relative scale. No external meshes or textures are
used. GLB metadata identifies each primitive and the unfinished study stage.

Proportion references (not sources of mesh assets):

- FCI Labrador standard: https://www.fci.be/Nomenclature/Standards/122g08-en.pdf
- FCI German Shepherd standard: https://www.fci.be/Nomenclature/Standards/166g01-en.pdf
- FCI Siberian Husky standard: https://www.fci.be/Nomenclature/Standards/270g05-en.pdf
- CFA British Shorthair: https://cfa.org/breed/british-shorthair/
- CFA Maine Coon: https://cfa.org/breed/maine-coon-cat/

Only dog shoulder targets use the standard's adult male height interval.
Other numerical measurements are authored study parameters, not certified
anatomical measurements or breed-standard requirements.

## Verification 2026-10-15

- Six standalone GLBs plus gallery exported from the new Rust generator.
- Two isolated tests passed, compiling current animal sources against cached
  dependencies/importer: deterministic export, distinct geometry, valid finite
  positions, unit normals, valid indices, all six imports and gallery import,
  invalid input rejection. This is not a full current-workspace build.
- Existing native `voxy_app --model` loaded the gallery after reducing geometry
  to fit the editor's 196608-index budget. The window showed grey silhouettes;
  this does not establish realistic visual quality or gameplay integration.
- Full `cargo test -p voxy_model animals:: --lib` failed in concurrently edited
  `voxy_render/src/archive_scene.rs` with an unclosed delimiter. No unrelated
  renderer changes were modified to work around it.


## Active gameplay revision

`build_sculpt` now blends rounded elliptical section volumes and extracts a
unified surface independently of the neutral GLB studies above. Gameplay uses
this new surface through `animals/breed_asset.rs`, with breed-specific groom,
48 three-particle guides, terrain sole sweeps and two-link foot IK. Neutral GLBs
remain the earlier study exports; they are not the current gameplay mesh.
See README.md for current runtime evidence and unresolved visual quality.


The later release build now succeeds; the earlier renderer build failures above
are historical evidence from the concurrent editing session. Three model tests
and seven native-animal tests pass. Native Metal captures cover all six breeds;
visual realism remains unaccepted.
