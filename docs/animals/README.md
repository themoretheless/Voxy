# Native dogs and cats

The default native world spawns a dog and a cat beside the player:

```sh
cargo run --release -p voxy_app --bin voxy_app
```

All anatomy, geometry, rigging, coat patterns and grooming are original Rust
implementations. No downloaded animal models, textures or animation clips are
used. `src/animals/asset.rs` builds continuous implicit surfaces with a welded
marching-tetrahedra mesh: ribcage, pelvis, neck, skull, muzzle, cheeks, articulated
digitigrade legs, separate toes, ears and curved tapered tails. Eyes, pupils,
corneal highlights, nose, inner ears and tapered whiskers are separate geometry.
The cat has a tabby coat and the dog has a brown short coat.
The initial visual result was rejected as cartoon-like; the current anatomical
revision is under visual review and is not a claimed photorealistic asset release.

Each quadruped uses ten body bones: root, four upper legs, four lower legs and
tail. Continuous skin weights join the shoulders and knees. Actual travelled
distance drives the diagonal gait; turns have a bounded angular speed. Native
swept body physics supplies gravity, grounding, sliding and limited steps.
Companion sweeps also include the other companion and the player body.
Body collision is a conservative upright AABB; it is not articulated ragdoll
collision. The player controller itself continues to use its terrain backend.

Each coat has 30,000 fine tapered, two-segment rendered fibers and 48 physical guide
strands with three particles each. Area-weighted surface sampling and farthest
point guide selection cover the skin. The Rust Verlet solver pins roots to the
posed body, rotates the authored rest directions, retains particle inertia and
applies gravity, damping and length constraints. Local skin colliders prevent
inward guide penetration. Rendered fibers follow the two simulated guide nodes
through the GPU skin palette. Followers share guide motion; they are not 30,000
independent physical strands. There is no full strand self-collision or fur
collision with arbitrary terrain/other animals. Fine paw IK, path planning and
persistent animal authoring remain separate work.

The native skinned shaders support an explicit packed-linear-color mode: UV.x
stores a 24-bit RGB integer, UV.y stores `-10000 - roughness`. Colors are decoded
before raster interpolation. This convention is reserved for native generated
meshes; imported texture coordinates retain the ordinary texture path. CleanCam
uses the existing linear HDR PBR renderer with coat colors and roughness,
without applying terrain relief or rain puddles to fur. Generated animal
materials use a filtered fine coat grain. Twelve posed sphere proxies provide
analytic soft ground contact occlusion; this is an approximation, not mesh shadow mapping. The ordinary voxel view
also preserves animal coat color.

## Window verification

```sh
cargo run --release -p voxy_app --bin voxy_app -- --animals-check
```

This is the actual native voxel game window using the same models, controller,
fur simulation and renderer as normal play. It hides the placeholder humanoid,
builds a dry level inspection pad, provides moving follow targets, and takes a group view, a dog close-up and a cat
close-up through the game's HDR pipeline. After 540 fixed ticks it requires both
animals to be grounded, to have travelled, and to have all 288 simulated guide
particles. The command-line check closes after three captures. The packaged inspection
window remains open on a stationary close-up after collecting its captures. Results are saved under
`artifacts/animals-native/`; screenshots still require visual review.

Focused regressions cover valid skinning, dense groom geometry, grounding,
following, rejection of invalid ticks, attachment and bounded guide lengths,
continuous body collisions and oriented strand rest poses.
