# Facial rendering source scan: emberlightstudios/Humentity

Repository: https://github.com/emberlightstudios/Humentity

Pinned source revision: 71af6f483c8c5a2623271124ca8e445ddc23db02

Discovery topic: MakeHuman

Description: Makehuman integration into Bevy Engine

Reported license: NOASSERTION

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"eye": ["assets/body_parts/Eyelashes/false_eyelashes/albedo/false_eyelashes.png", "assets/body_parts/Eyelashes/false_eyelashes/eyelashes04.obj", "assets/body_parts/Eyelashes/false_eyelashes/false_eyelashes.mhclo", "assets/body_parts/Eyelashes/normal_eyelashes/albedo/eyelashes01.png", "assets/body_parts/Eyelashes/normal_eyelashes/albedo/eyelashes02.png", "assets/body_parts/Eyelashes/normal_eyelashes/albedo/eyelashes03.png", "assets/body_parts/Eyelashes/normal_eyelashes/eyelashes.mhclo", "assets/body_parts/Eyelashes/normal_eyelashes/eyelashes01.obj", "assets/body_parts/Eyes/Eyeballs/albedo/blue_eye.png", "assets/body_parts/Eyes/Eyeballs/albedo/bluegreen_eye.png", "assets/body_parts/Eyes/Eyeballs/albedo/brown_eye.png", "assets/body_parts/Eyes/Eyeballs/albedo/brownlight_eye.png", "assets/body_parts/Eyes/Eyeballs/albedo/deepblue_eye.png", "assets/body_parts/Eyes/Eyeballs/albedo/green_eye.png", "assets/body_parts/Eyes/Eyeballs/albedo/grey_eye.png", "assets/body_parts/Eyes/Eyeballs/albedo/ice_eye.png", "assets/body_parts/Eyes/Eyeballs/albedo/lightblue_eye.png", "assets/body_parts/Eyes/Eyeballs/high-poly-eyes.mhclo", "assets/body_parts/Eyes/Eyeballs/high-poly.mtl", "assets/body_parts/Eyes/Eyeballs/high-poly.obj", "assets/body_parts/Eyes/Eyeballs/low-poly-eyes.mhclo", "assets/body_parts/Eyes/Eyeballs/low-poly.obj", "assets/body_parts/eyebrows/eyebrows001/albedo/eyebrow001.png", "assets/body_parts/eyebrows/eyebrows001/eyebrow001.mhclo", "assets/body_parts/eyebrows/eyebrows001/eyebrow001.obj", "assets/body_parts/eyebrows/eyebrows001/normal/eyebrow001.png", "assets/body_parts/eyebrows/eyebrows002/albedo/eyebrow002.png", "assets/body_parts/eyebrows/eyebrows002/eyebrow002.mhclo", "assets/body_parts/eyebrows/eyebrows002/eyebrow002.obj", "assets/body_parts/eyebrows/eyebrows002/normal/eyebrow002.png"], "skin": ["assets/skin_textures/albedo/middleage_african_female.png", "assets/skin_textures/albedo/middleage_african_male.png", "assets/skin_textures/albedo/middleage_asian_female.png", "assets/skin_textures/albedo/middleage_asian_male.png", "assets/skin_textures/albedo/middleage_caucasian_female.png", "assets/skin_textures/albedo/middleage_caucasian_male.png", "assets/skin_textures/albedo/old_african_female.png", "assets/skin_textures/albedo/old_african_male.png", "assets/skin_textures/albedo/old_asian_female.png", "assets/skin_textures/albedo/old_asian_male.png", "assets/skin_textures/albedo/old_caucasian_female.png", "assets/skin_textures/albedo/old_caucasian_male.png", "assets/skin_textures/albedo/young_african_female.png", "assets/skin_textures/albedo/young_african_male.png", "assets/skin_textures/albedo/young_asian_female.png", "assets/skin_textures/albedo/young_asian_male.png", "assets/skin_textures/albedo/young_caucasian_female.png", "assets/skin_textures/albedo/young_caucasian_female2.png", "assets/skin_textures/albedo/young_caucasian_female_special_suit.png", "assets/skin_textures/albedo/young_caucasian_male.png", "assets/skin_textures/albedo/young_caucasian_male2.png", "assets/skin_textures/albedo/young_caucasian_male_special_suit.png", "src/gpu/crowd_skin.wgsl"], "jaw": ["assets/targets/asym/asym-jaw-1-l.target", "assets/targets/asym/asym-jaw-1-r.target", "assets/targets/asym/asym-jaw-2-l.target", "assets/targets/asym/asym-jaw-2-r.target", "assets/targets/asym/asym-jaw-3-l.target", "assets/targets/asym/asym-jaw-3-r.target", "assets/targets/chin/chin-jaw-drop-decr.target", "assets/targets/chin/chin-jaw-drop-incr.target", "assets/targets/expressions/jawForward.target", "assets/targets/expressions/jawLeft.target", "assets/targets/expressions/jawOpen.target", "assets/targets/expressions/jawRight.target"], "mouth": ["assets/targets/asym/asym-mouth-1-l.target", "assets/targets/asym/asym-mouth-1-r.target", "assets/targets/asym/asym-mouth-2-l.target", "assets/targets/asym/asym-mouth-2-r.target", "assets/targets/expressions/mouthClose.target", "assets/targets/expressions/mouthDimpleLeft.target", "assets/targets/expressions/mouthDimpleRight.target", "assets/targets/expressions/mouthFrownLeft.target", "assets/targets/expressions/mouthFrownRight.target", "assets/targets/expressions/mouthFunnel.target", "assets/targets/expressions/mouthLeft.target", "assets/targets/expressions/mouthLowerDownLeft.target", "assets/targets/expressions/mouthLowerDownRight.target", "assets/targets/expressions/mouthPressLeft.target", "assets/targets/expressions/mouthPressRight.target", "assets/targets/expressions/mouthPucker.target", "assets/targets/expressions/mouthRight.target", "assets/targets/expressions/mouthRollLower.target", "assets/targets/expressions/mouthRollUpper.target", "assets/targets/expressions/mouthShrugLower.target", "assets/targets/expressions/mouthShrugUpper.target", "assets/targets/expressions/mouthSmileLeft.target", "assets/targets/expressions/mouthSmileRight.target", "assets/targets/expressions/mouthStretchLeft.target", "assets/targets/expressions/mouthStretchRight.target", "assets/targets/expressions/mouthUpperUpLeft.target", "assets/targets/expressions/mouthUpperUpRight.target", "assets/targets/mouth/mouth-angles-down.target", "assets/targets/mouth/mouth-angles-up.target", "assets/targets/mouth/mouth-cupidsbow-decr.target"], "teeth": ["assets/body_parts/Teeth/basic_teeth/Teeth-base.mhclo", "assets/body_parts/Teeth/basic_teeth/albedo/teeth.png", "assets/body_parts/Teeth/basic_teeth/teeth01.mhclo", "assets/body_parts/Teeth/basic_teeth/teeth02.mhclo", "assets/body_parts/Teeth/basic_teeth/teeth03.mhclo", "assets/body_parts/Teeth/basic_teeth/teeth04.mhclo", "assets/body_parts/Teeth/basic_teeth/teeth05.mhclo", "assets/body_parts/Teeth/basic_teeth/teeth_base.obj", "assets/body_parts/Teeth/basic_teeth/teeth_shape01.obj", "assets/body_parts/Teeth/basic_teeth/teeth_shape02.obj", "assets/body_parts/Teeth/basic_teeth/teeth_shape03.obj", "assets/body_parts/Teeth/basic_teeth/teeth_shape04.obj", "assets/body_parts/Teeth/basic_teeth/teeth_shape05.obj"], "tongue": ["assets/body_parts/Tongue/tongue/tongue.mhclo", "assets/body_parts/Tongue/tongue/tongue01.obj", "assets/body_parts/Tongue/tongue/tongue01.thumb", "assets/body_parts/Tongue/tongue/tongue01_diffuse.png", "assets/targets/expressions/tongueOut.target"], "rig": ["assets/rigs/rig.default.json", "assets/rigs/rig.game_engine.json", "assets/rigs/rig.mixamo.json", "assets/rigs/weights.default.json", "assets/rigs/weights.game_engine.json", "assets/rigs/weights.mixamo.json", "assets/targets/expressions/browDownRight.target", "assets/targets/expressions/browOuterUpRight.target", "assets/targets/expressions/cheekSquintRight.target", "assets/targets/expressions/eyeBlinkRight.target", "assets/targets/expressions/eyeLookDownRight.target", "assets/targets/expressions/eyeLookInRight.target", "assets/targets/expressions/eyeLookOutRight.target", "assets/targets/expressions/eyeLookUpRight.target", "assets/targets/expressions/eyeSquintRight.target", "assets/targets/expressions/eyeWideRight.target", "assets/targets/expressions/jawRight.target", "assets/targets/expressions/mouthDimpleRight.target", "assets/targets/expressions/mouthFrownRight.target", "assets/targets/expressions/mouthLowerDownRight.target", "assets/targets/expressions/mouthPressRight.target", "assets/targets/expressions/mouthRight.target", "assets/targets/expressions/mouthSmileRight.target", "assets/targets/expressions/mouthStretchRight.target", "assets/targets/expressions/mouthUpperUpRight.target", "assets/targets/expressions/noseSneerRight.target", "src/loaders/reference_rig.rs", "src/loaders/rig_config.rs", "src/loaders/rig_weights.rs", "src/rigs.rs"], "lip": ["assets/targets/mouth/mouth-lowerlip-ext-down.target", "assets/targets/mouth/mouth-lowerlip-ext-up.target", "assets/targets/mouth/mouth-lowerlip-height-decr.target", "assets/targets/mouth/mouth-lowerlip-height-incr.target", "assets/targets/mouth/mouth-lowerlip-middle-down.target", "assets/targets/mouth/mouth-lowerlip-middle-up.target", "assets/targets/mouth/mouth-lowerlip-volume-decr.target", "assets/targets/mouth/mouth-lowerlip-volume-incr.target", "assets/targets/mouth/mouth-lowerlip-width-decr.target", "assets/targets/mouth/mouth-lowerlip-width-incr.target", "assets/targets/mouth/mouth-upperlip-ext-down.target", "assets/targets/mouth/mouth-upperlip-ext-up.target", "assets/targets/mouth/mouth-upperlip-height-decr.target", "assets/targets/mouth/mouth-upperlip-height-incr.target", "assets/targets/mouth/mouth-upperlip-middle-down.target", "assets/targets/mouth/mouth-upperlip-middle-up.target", "assets/targets/mouth/mouth-upperlip-volume-decr.target", "assets/targets/mouth/mouth-upperlip-volume-incr.target", "assets/targets/mouth/mouth-upperlip-width-decr.target", "assets/targets/mouth/mouth-upperlip-width-incr.target"]}

## Source file: src/gpu/crowd_skin.wgsl

{
  "path": "src/gpu/crowd_skin.wgsl",
  "blob_sha": "709ad901aa24e1a98035aae5f4340be552199337",
  "sha256": "2d6373039b474543ff6d509a621e875e69a76a6271b2f961645413cc1603a5f0",
  "lines": 110,
  "bytes": 4444,
  "selection_score": 27,
  "keyword_line_counts": {
    "skin": 23,
    "shader": 1,
    "rig": 1,
    "lip": 2
  },
  "url": "https://github.com/emberlightstudios/Humentity/blob/71af6f483c8c5a2623271124ca8e445ddc23db02/src/gpu/crowd_skin.wgsl",
  "function_signature_candidates": [
    {
      "line": 43,
      "text": "fn crowd_skin_pose("
    },
    {
      "line": 103,
      "text": "fn crowd_skin_world_tangent("
    }
  ]
}

```text

#define_import_path humentity::crowd_skin

#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_functions
#import bevy_pbr::morph
#import bevy_pbr::view_transformations

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<storage, read> crowd_joints: array<mat4x4f>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> crowd_pose: CrowdSkinUniform;

struct CrowdSkinUniform {
    num_bones: u32,
    pad_a: u32,
    pad_b: u32,
    pad_c: u32,
};

// Posed-vertex data with no `forward_io` / `prepass_io` types, so every entry
// can consume it and build its own output.
struct CrowdSkinnedVertex {
    clip_position: vec4<f32>,
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    skin_matrix: mat4x4f,
    mesh_world: mat4x4f,
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    visibility_range_dither: i32,
#endif
};

// Poses one vertex from the GPU joint buffer: morph targets first (Bevy's
// `mesh.wgsl` order: morph, then skin), then the four influencing joints for
// this character's crowd row, blended. `instance_index` is the render batch
// slot (world transform, normals); the crowd row comes from `MeshTag` via
// `get_tag`, so batch order never has to match compute order.
// `vertex_index` is the render `@builtin(vertex_index)` of this vertex: the
// morph helpers index by position in the vertex buffer, and the mesh vertex
// shader runs before indexing effects, so the raw buffer index is the right
// key.
fn crowd_skin_pose(
    instance_index: u32,
    vertex_index: u32,
    position: vec3<f32>,
    normal: vec3<f32>,
    joint_indices: vec4<u32>,
    joint_weights: vec4<f32>,
) -> CrowdSkinnedVertex {
    var skinned: CrowdSkinnedVertex;
#ifdef MORPH_TARGETS
    let first_vertex = mesh[instance_index].first_vertex_index;
    let morph_vertex = vertex_index - first_vertex;
    var morphed_position = position;
    var morphed_normal = normal;
    let weight_count = morph::layer_count(instance_index);
    for (var i: u32 = 0u; i < weight_count; i++) {
        let morph_weight = morph::weight_at(i, instance_index);
        if (morph_weight == 0.0) {
            continue;
        }
        morphed_position += morph_weight * morph::morph_position(morph_vertex, i, instance_index);
        morphed_normal += morph_weight * morph::morph_normal(morph_vertex, i, instance_index);
    }
#else
    let morphed_position = position;
    let morphed_normal = normal;
#endif
    let crowd_index = mesh_functions::get_tag(instance_index);
    let joint_row_base = crowd_index * crowd_pose.num_bones;
    let joint_matrix_0 = crowd_joints[joint_row_base + joint_indices.x];
    let joint_matrix_1 = crowd_joints[joint_row_base + joint_indices.y];
    let joint_matrix_2 = crowd_joints[joint_row_base + joint_indices.z];
    let joint_matrix_3 = crowd_joints[joint_row_base + joint_indices.w];
    let skin_matrix =
        joint_matrix_0 * joint_weights.x + joint_matrix_1 * joint_weights.y +
        joint_matrix_2 * joint_weights.z + joint_matrix_3 * joint_weights.w;
    let mesh_world = mesh_functions::get_world_from_local(instance_index);
    let world_from_local = mesh_world * skin_matrix;
    skinned.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(morphed_position, 1.0)
    );
    skinned.clip_position = view_transformations::position_world_to_clip(skinned.world_position.xyz);
    let posed_normal = (skin_matrix * vec4<f32>(morphed_normal, 0.0)).xyz;
    skinned.world_normal = mesh_functions::mesh_normal_local_to_world(posed_normal, instance_index);
    skinned.skin_matrix = skin_matrix;
    skinned.mesh_world = mesh_world;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    skinned.instance_index = instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    skinned.visibility_range_dither =
        mesh_functions::get_visibility_range_dither_level(instance_index, mesh_world[3]);
#endif
    return skinned;
}

// Poses a tangent from already-posed data for entries that carry tangents.
// Crowd meshes carry none, so this only keeps the prepass compiling under
// `VERTEX_TANGENTS`.
fn crowd_skin_world_tangent(
    skinned: CrowdSkinnedVertex,
    tangent: vec4<f32>,
    instance_index: u32,
) -> vec4<f32> {
    let world_from_local = skinned.mesh_world * skinned.skin_matrix;
    return mesh_functions::mesh_tangent_local_to_world(world_from_local, tangent, instance_index);
}


```

## Source file: src/rigs.rs

{
  "path": "src/rigs.rs",
  "blob_sha": "eded36346b5470bea2bd3d8401d1aa24561abce6",
  "sha256": "5f3ba7597b14198efbae08992615ce91292fbef5cd55a65541050036f30dd5f0",
  "lines": 533,
  "bytes": 19081,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 6,
    "shader": 1,
    "rig": 115,
    "face": 1,
    "lip": 3
  },
  "url": "https://github.com/emberlightstudios/Humentity/blob/71af6f483c8c5a2623271124ca8e445ddc23db02/src/rigs.rs",
  "function_signature_candidates": [
    {
      "line": 53,
      "text": "pub(crate) fn skeleton_rear_offset_meters(rig_name: &str) -> f32 {"
    },
    {
      "line": 84,
      "text": "fn default() -> Self {"
    },
    {
      "line": 100,
      "text": "pub fn bone_index(&self, bone_name: &str) -> Option<usize> {"
    },
    {
      "line": 107,
      "text": "pub fn reference_rig(&self) -> &ReferenceRigAsset {"
    },
    {
      "line": 122,
      "text": "pub(crate) fn sync_and_build_rig_data("
    },
    {
      "line": 181,
      "text": "pub(crate) fn build_rig_scenes(world: &mut World) {"
    },
    {
      "line": 225,
      "text": "pub(crate) fn build_skeleton_scene("
    },
    {
      "line": 365,
      "text": "pub(crate) fn get_model_space_skeleton_transforms("
    },
    {
      "line": 387,
      "text": "pub(crate) fn fitted_model_space_bindposes("
    },
    {
      "line": 425,
      "text": "fn toe_test_pose() -> ("
    },
    {
      "line": 452,
      "text": "fn kept_toe_moves_to_average_toe_position() {"
    },
    {
      "line": 463,
      "text": "fn other_rigs_keep_original_toe_positions() {"
    },
    {
      "line": 470,
      "text": "fn missing_toe_bones_skip_without_panic() {"
    },
    {
      "line": 478,
      "text": "pub(crate) fn get_bone_transform("
    },
    {
      "line": 494,
      "text": "fn get_bone_position(bone: &BoneTransformSpec, vg: &VertexGroups, helpers: &[Vec3]) -> Vec3 {"
    }
  ]
}

```text

use ahash::AHashMap;
use bevy::{
    animation::AnimationTargetId,
    ecs::intern::Internable,
    ecs::system::SystemState,
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    prelude::*,

};
use std::sync::Arc;

use crate::{
    basemesh::VertexGroups,
    loaders::{ReferenceRigAsset, RigConfigAsset, RigWeightsAsset},
    prelude::*,
    skeleton_lod::{
        RigBundle, SkeletonLodConfig, average_default_rig_toe_positions, build_lod_data,
    },
};

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SkeletalBone;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct RootBone;

/// Cached data for the root bone of each skeleton.
/// Stores the root bone entity and a scale factor used to adjust root bone Y translation
/// for different human proportions during retargeted animation playback.
#[derive(Component, Debug)]
pub struct SkeletonRootBone {
    pub entity: Entity,
    pub root_scale: f32,
    pub bind_pose_y: f32,
    /// Reference-rig root Y the clip was authored against. Offsets are
    /// measured from here, then scaled onto the fitted bind pose.
    pub reference_bind_pose_y: f32,
}

/// Rearward shift for the default rig's skeleton root, in reference-rig meters.
/// The default rig's root bone sits at the rear of the pelvis, so without this
/// fitted characters ride slightly forward of their capsule collider. Scales
/// with `CharacterScale` on the CPU; the GPU pose shader applies the same shift
/// from its uniforms (see `pose.wgsl`). Characters face -Z, so rearward is +Z.
/// Other rigs get no shift.
pub(crate) const DEFAULT_RIG_REAR_OFFSET_METERS: f32 = 0.05;

/// Skeleton-root rearward shift for a rig: the default-rig shift, or zero for
/// any other rig. Shared by the CPU skeleton root (`spawn_skeleton`) and the
/// GPU pose uniforms (`gpu::bake`).
pub(crate) fn skeleton_rear_offset_meters(rig_name: &str) -> f32 {
    if rig_name == "default" {
        DEFAULT_RIG_REAR_OFFSET_METERS
    } else {
        0.0
    }
}

/// The single rig bundle, populated by `build_rig_scenes`.
/// Holds the full skeleton scene (used for spawning) plus the per-LOD merge data
/// (used for mesh painting / part skinning).
#[derive(Resource, Default)]
pub(crate) struct RigBundleRes {
    /// Full skeleton scene (all bones), spawned once per character.
    pub scene: Option<Handle<DynamicWorld>>,
    /// Per-LOD merge data for the rig.
    pub bundle: Option<RigBundle>,
}

/// Stores the single loaded rig specification.
#[derive(Resource)]
pub struct RigData(pub Option<RigSpec>);

#[derive(Clone)]
pub struct RigSpec {
    pub(crate) weights: Arc<RigWeightsAsset>,
    pub(crate) config: Arc<RigConfigAsset>,
    pub(crate) reference_rig: Arc<ReferenceRigAsset>,
}

impl Default for RigData {
    fn default() -> Self {
        Self::new()
    }
}

impl RigData {
    pub const fn new() -> Self {
        Self(None)
    }

    pub const fn is_loaded(&self) -> bool {
        self.0.is_some()
    }
}

impl RigSpec {
    pub fn bone_index(&self, bone_name: &str) -> Option<usize> {
        self.reference_rig
            .bone_name_to_index
            .get(bone_name)
            .copied()
    }

    pub fn reference_rig(&self) -> &ReferenceRigAsset {
        &self.reference_rig
    }
}

/// Tracks which rig assets have loaded, for event-driven sync.
#[derive(Default)]
pub(crate) struct RigLoadTracker {
    config: Option<RigConfigAsset>,
    weights: Option<RigWeightsAsset>,
    reference_rig: Option<ReferenceRigAsset>,
}

/// Syncs rig assets reactively as they load. Once all 3 asset types have reported
/// a `LoadedWithDependencies` event, matches them by `rig_name` and inserts into `RigData`.
pub(crate) fn sync_and_build_rig_data(
    mut rig_data: ResMut<RigData>,
    config_assets: Res<Assets<RigConfigAsset>>,
    weights_assets: Res<Assets<RigWeightsAsset>>,
    reference_rig_assets: Res<Assets<ReferenceRigAsset>>,
    mut config_events: MessageReader<AssetEvent<RigConfigAsset>>,
    mut weights_events: MessageReader<AssetEvent<RigWeightsAsset>>,
    mut ref_rig_events: MessageReader<AssetEvent<ReferenceRigAsset>>,
    mut tracker: Local<RigLoadTracker>,
) {
    if rig_data.is_loaded() {
        return;
    }

    for ev in config_events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = ev
            && let Some(asset) = config_assets.get(*id)
        {
            tracker.config = Some(asset.clone());
        }
    }
    for ev in weights_events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = ev
            && let Some(asset) = weights_assets.get(*id)
        {
            tracker.weights = Some(asset.clone());
        }
    }
    for ev in ref_rig_events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = ev
            && let Some(asset) = reference_rig_assets.get(*id)
        {
            tracker.reference_rig = Some(asset.clone());
        }
    }

    let (Some(config), Some(weights), Some(reference_rig)) = (
        tracker.config.as_ref(),
        tracker.weights.as_ref(),
        tracker.reference_rig.as_ref(),
    ) else {
        return;
    };

    if config.rig_name == weights.rig_name && config.rig_name == reference_rig.rig_name {
        rig_data.0 = Some(RigSpec {
            weights: Arc::new(weights.clone()),
            config: Arc::new(config.clone()),
            reference_rig: Arc::new(reference_rig.clone()),
        });
    }
}

/// Tracks which rigs have already been built so we don't rebuild every frame.
#[derive(Resource, Default)]
pub(crate) struct BuiltRigs;

/// Builds skeleton scenes and LOD variants for the loaded rig.
/// Runs once (deduplicated via [`BuiltRigs`]).
pub(crate) fn build_rig_scenes(world: &mut World) {
    // Collect rig data and config
    let (reference_rig, weights, lod_config) = {
        let mut state: SystemState<(
            Res<RigData>,
            Option<Res<SkeletonLodConfig>>,
        )> = SystemState::new(world);
        match state.get(world) {
            Ok((rig_data, lod_config)) => {
                let Some(spec) = &rig_data.0 else {
                    return;
                };
                (
                    Arc::clone(&spec.reference_rig),
                    Arc::clone(&spec.weights),
                    lod_config.map(|c| c.clone()),
                )
            }
            Err(_) => return,
        }
    };

    // Build full skeleton scene (index 0)
    let scene = build_skeleton_scene(&reference_rig, world);

    let lod_config = lod_config.unwrap_or_default();

    // Use merge configs from SkeletonLodConfig (or full skeleton)
    let merge_configs = &lod_config.0[..lod_config.1];

    // Build per-LOD merge data (no separate skeleton scenes are built)
    let lod_data = build_lod_data(&reference_rig, &weights, merge_configs);

    // Store in registry
    let mut bundle_res = world.resource_mut::<RigBundleRes>();
    bundle_res.scene = Some(scene);
    bundle_res.bundle = Some(RigBundle { lod_data });

    // Mark as built
    world.insert_resource(BuiltRigs);
}


/// Builds skeleton scene from reference rig.
pub(crate) fn build_skeleton_scene(
    reference_rig: &Arc<ReferenceRigAsset>,
    world: &mut World,
) -> Handle<DynamicWorld> {
    let bone_order = &reference_rig.bone_names;
    let ref_bone_parents = &reference_rig.bone_parents;
    let ref_local_bindpose = &reference_rig.local_bindpose;

    // Set up some convenient data structures for tracking joints/bones and entities
    let mut bone_entities = AHashMap::<&'static str, Entity>::default();

    // Start scene world with rig entity
    let registry = world.resource::<AppTypeRegistry>();
    let mut scene_world = World::new();
    scene_world.insert_resource(registry.clone());

    let rig_entity = scene_world
        .spawn((
            Name::new("Human.rig"),
            Transform::IDENTITY,
            // Not really a "bone" per se but useful when finding local bone transforms
            SkeletalBone,
        ))
        .id();

    // Spawn all bone entities
    for (i, &name) in bone_order.iter().enumerate() {
        let mut path = Vec::<Name>::new();
        path.push(Name::from(name));

        let mut current_parent = ref_bone_parents.get(name).cloned().unwrap_or_default();
        while !current_parent.is_empty() {
            path.push(Name::new(current_parent.clone()));
            let next = ref_bone_parents
                .get(&NAME_INTERNER.intern(&current_parent).leak())
                .cloned()
                .unwrap_or_default();
            current_parent = next;
        }

        let entity = scene_world
            .spawn((
                Name::new(name),
                AnimationTargetId::from_names(path.iter().rev()),
                SkeletalBone,
            ))
            .id();

        if i == 0 {
            scene_world.entity_mut(entity).insert(RootBone);
        }
        bone_entities.insert(name, entity);
    }

    // Wire up parent-child relationships.
    for &name in bone_order.iter() {
        let &child = bone_entities.get(&name).unwrap_or_else(|| {
            panic!(
                "reference rig bone '{name}' was never spawned: bone_order and spawned entities diverged (check the reference rig GLB joint list)"
            )
        });
        let parent_name = ref_bone_parents.get(name).cloned().unwrap_or_default();
        if !parent_name.is_empty()
            && let Some(&parent) = bone_entities.get(NAME_INTERNER.intern(&parent_name).leak())
        {
            scene_world.entity_mut(parent).add_child(child);
        }
    }

    // Attach the single root to the rig entity. MPFB exports exactly one bone
    // under `Human.rig`: 0 means the rig name is wrong, 2+ means the armature
    // is not single-root. Either way attaching silently would desync clip
    // TargetIds from skeleton joints.
    let mut rig_root_children = bone_order
        .iter()
        .filter(|bone_name| {
            ref_bone_parents
                .get(*bone_name)
                .is_some_and(|rig_parent_name| rig_parent_name == "Human.rig")
        })
        .peekable();
    let Some(&root_bone_name) = rig_root_children.next() else {
        panic!(
            "reference rig '{}' has no bone parented to 'Human.rig': the MPFB rig is single-root under Human.rig (check the reference rig GLB joint list)",
            reference_rig.rig_name,
        );
    };
    if let Some(&second_root_name) = rig_root_children.next() {
        panic!(
            "reference rig '{}' has multiple bones parented to 'Human.rig' ('{root_bone_name}' and '{second_root_name}'): the MPFB rig is single-root (check the reference rig GLB joint list)",
            reference_rig.rig_name,
        );
    }
    let &root_bone = bone_entities.get(&root_bone_name).unwrap_or_else(|| {
        panic!(
            "reference rig root bone '{root_bone_name}' was never spawned: bone_order and spawned entities diverged (check the reference rig GLB joint list)"
        )
    });
    scene_world.entity_mut(rig_entity).add_child(root_bone);

    let (global_transforms, local_transforms) = {
        let local = ref_local_bindpose.clone();
        let model_space = reference_rig.model_space_bindpose.clone();

        (model_space, local)
    };

    // compute inverse bindposes
    let mut inverse_bindposes = Vec::with_capacity(bone_order.len());
    for &name in bone_order.iter() {
        let entity = bone_entities[&name];
        let local = local_transforms[&name];
        let global = global_transforms[&name];

        scene_world.entity_mut(entity).insert(local);

        // Inverse bindpose is always from global transform
        inverse_bindposes.push(global.to_matrix().inverse());
    }

    // Setup SkinnedMesh component
    let mut inverse_bindpose_assets = world.resource_mut::<Assets<SkinnedMeshInverseBindposes>>();
    let inverse_bindposes = inverse_bindpose_assets.add(inverse_bindposes);
    // only need to return bindposes.  entities will have to be mapped manually after spawning scene
    let joint_entities = bone_order
        .iter()
        .map(|n| bone_entities[n])
        .collect::<Vec<_>>();

    let skinned_mesh = SkinnedMesh {
        inverse_bindposes,
        joints: joint_entities,
    };
    scene_world.entity_mut(rig_entity).insert(skinned_mesh);

    let mut ds = world.resource_mut::<Assets<DynamicWorld>>();

    ds.add(DynamicWorld::from_world(&scene_world))
}

pub(crate) fn get_model_space_skeleton_transforms(
    bone_order: &Vec<&'static str>,
    helpers: &[Vec3],
    rig_spec: &RigSpec,
    vg: &VertexGroups,
) -> AHashMap<&'static str, Transform> {
    let mh_config = &rig_spec.config;
    // Compute global transforms from basemesh + roll in config (no GLB reference)
    let mut global_transforms = AHashMap::<&'static str, Transform>::default();
    for &name in bone_order.iter() {
        let bone = &mh_config.bones[name];
        global_transforms.insert(name, get_bone_transform(bone, vg, helpers));
    }
    global_transforms
}

/// Model-space bindpose with reference rotations restored (CPU fit pass 1).
///
/// Positions come from the morphed helpers; every rotation is replaced with the
/// reference rig's model-space rotation, so retargeted clips (authored for
/// reference rotations) play correctly on any shape. Shared by the CPU fit
/// (`fit_skeleton_to_shape`) and the GPU shape fit (`fit_shape_skeleton`).
pub(crate) fn fitted_model_space_bindposes(
    helpers: &[Vec3],
    rig: &RigSpec,
    vg: &VertexGroups,
) -> AHashMap<&'static str, Transform> {
    let mut model_space =
        get_model_space_skeleton_transforms(&rig.reference_rig.bone_names, helpers, rig, vg);
    let bone_config = &rig.config;
    // Pass 1: replace all model-space rotations with reference rig rotations.
    // This must happen before any local computation so parent lookups are correct.
    for &bone in &rig.reference_rig.bone_names {
        if bone_config.bones.contains_key(bone) {
            let old_global = model_space[bone];
            let reference_rot = rig.reference_rig.model_space_bindpose[bone].rotation;
            model_space.insert(
                bone,
                Transform {
                    translation: old_global.translation,
                    rotation: reference_rot,
                    scale: old_global.scale,
                },
            );
        }
    }
    average_default_rig_toe_positions(
        &rig.reference_rig().rig_name,
        &rig.reference_rig().bone_parents,
        &mut model_space,
    );
    model_space
}

#[cfg(test)]
mod default_rig_toe_average_tests {
    use crate::skeleton_lod::average_default_rig_toe_positions;

    use super::*;

    fn toe_test_pose() -> (
        AHashMap<&'static str, String>,
        AHashMap<&'static str, Transform>,
    ) {
        let bone_parents: AHashMap<&'static str, String> = [
            ("foot.L", "lowerleg02.L".to_string()),
            ("toe1-1.L", "foot.L".to_string()),
            ("toe1-2.L", "toe1-1.L".to_string()),
            ("toe2-1.L", "foot.L".to_string()),
        ]
        .into_iter()
        .collect();
        let model_space: AHashMap<&'static str, Transform> = [
            ("foot.L", Vec3::ZERO),
            ("toe1-1.L", Vec3::ZERO),
            ("toe1-2.L", Vec3::new(0.0, 0.0, 2.0)),
            ("toe2-1.L", Vec3::new(3.0, 0.0, 0.0)),
        ]
        .into_iter()
        .map(|(toe_bone_name, toe_position)| {
            (toe_bone_name, Transform::from_translation(toe_position))
        })
        .collect();
        (bone_parents, model_space)
    }

    #[test]
    fn kept_toe_moves_to_average_toe_position() {
        let (bone_parents, mut model_space) = toe_test_pose();
        average_default_rig_toe_positions("default", &bone_parents, &mut model_space);
        let kept_toe_translation = model_space["toe1-1.L"].translation;
        assert!(
            (kept_toe_translation - Vec3::new(1.0, 0.0, 2.0 / 3.0)).length() < 1e-6,
            "kept toe must sit at the toe centroid, got {kept_toe_translation:?}"
        );
    }

    #[test]
    fn other_rigs_keep_original_toe_positions() {
        let (bone_parents, mut model_space) = toe_test_pose();
        average_default_rig_toe_positions("mixamo", &bone_parents, &mut model_space);
        assert_eq!(model_space["toe1-1.L"].translation, Vec3::ZERO);
    }

    #[test]
    fn missing_toe_bones_skip_without_panic() {
        let (bone_parents, _) = toe_test_pose();
        let mut model_space: AHashMap<&'static str, Transform> = AHashMap::default();
        average_default_rig_toe_positions("default", &bone_parents, &mut model_space);
        assert!(model_space.is_empty());
    }
}

pub(crate) fn get_bone_transform(
    bone: &BoneJsonConfig,
    vg: &VertexGroups,
    helpers: &[Vec3],
) -> Transform {
    let start = get_bone_position(&bone.head, vg, helpers);
    let end = get_bone_position(&bone.tail, vg, helpers);

    let orientation = (end - start).normalize();
    // Align bone axis (Y in local) with head->tail, then apply roll around that axis (Blender convention)
    let r_align = Quat::from_rotation_arc(Vec3::Y, orientation);
    let base_rot = r_align * Quat::from_rotation_y(bone.roll);

    Transform::from_translation(start).with_rotation(base_rot)
}

fn get_bone_position(bone: &BoneTransformSpec, vg: &VertexGroups, helpers: &[Vec3]) -> Vec3 {
    if bone.strategy == "MEAN" {
        let indices = bone.vertex_indices.as_ref().unwrap_or_else(|| {
            panic!(
                "rig config bone uses MEAN strategy but has no vertex_indices: each MEAN bone needs exactly 2 helper vertex ids (check the rig config asset)"
            )
        });
        let (v1, v2) = (indices[0], indices[1]);
        (helpers[v2 as usize] + helpers[v1 as usize]) / 2.
    } else if bone.strategy == "CUBE" {
        let joint = bone.cube_name.as_ref().unwrap_or_else(|| {
            panic!(
                "rig config bone uses CUBE strategy but has no cube_name: each CUBE bone must name a vertex group (check the rig config asset)"
            )
        });
        let group = vg.get(joint).unwrap_or_else(|| {
            panic!(
                "rig config CUBE bone names vertex group '{joint}' which is missing from the vertex-groups asset"
            )
        });
        let (v1, v2) = (group[0][0] as u16, group[0][1] as u16);
        let mut pos = Vec3::ZERO;
        for v in v1..v2 + 1 {
            pos += helpers[v as usize];
        }
        pos / (v2 - v1 + 1) as f32
    } else if bone.strategy == "VERTEX" {
        let index = bone.vertex_index.unwrap_or_else(|| {
            panic!(
                "rig config bone uses VERTEX strategy but has no vertex_index: each VERTEX bone needs one helper vertex id (check the rig config asset)"
            )
        });
        helpers[index as usize]
    } else {
        unimplemented!(
            "rig config bone has unrecognized strategy '{}': expected MEAN, CUBE, or VERTEX (check the rig config asset)",
            bone.strategy
        )
    }
}


```

## Source file: src/loaders/rig_config.rs

{
  "path": "src/loaders/rig_config.rs",
  "blob_sha": "2b6d7cb92efff64105bf7c25072b4324ae7f2323",
  "sha256": "75078eda9575023f8bbd1025ba1ef3a6dea0328b3b031c4be68af796faea27ba",
  "lines": 94,
  "bytes": 2557,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 13
  },
  "url": "https://github.com/emberlightstudios/Humentity/blob/71af6f483c8c5a2623271124ca8e445ddc23db02/src/loaders/rig_config.rs",
  "function_signature_candidates": [
    {
      "line": 54,
      "text": "async fn load("
    },
    {
      "line": 89,
      "text": "fn extensions(&self) -> &[&str] {"
    }
  ]
}

```text

use ahash::AHashMap;
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    ecs::intern::Internable,
    prelude::*,
};
use serde::Deserialize;

use crate::NAME_INTERNER;

#[derive(Clone, Debug, Deserialize)]
pub struct BoneTransformSpec {
    pub cube_name: Option<String>,
    pub strategy: String,
    pub vertex_indices: Option<Vec<u16>>,
    pub vertex_index: Option<u16>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BoneJsonConfig {
    pub parent: String,
    pub head: BoneTransformSpec,
    pub tail: BoneTransformSpec,
    #[serde(default)]
    pub roll: f32,
}

#[derive(Asset, TypePath, Clone, Debug)]
pub struct RigConfigAsset {
    pub bones: AHashMap<&'static str, BoneJsonConfig>,
    pub rig_name: String,
}

#[derive(Deserialize)]
struct MixamoRigConfigAsset {
    bones: AHashMap<String, BoneJsonConfig>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RigConfigFile {
    Direct(AHashMap<String, BoneJsonConfig>),
    Mixamo(MixamoRigConfigAsset),
}

#[derive(Default, TypePath)]
pub struct RigConfigAssetLoader;

impl AssetLoader for RigConfigAssetLoader {
    type Asset = RigConfigAsset;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let parsed = serde_json::from_slice::<RigConfigFile>(&bytes)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;

        let bones = match parsed {
            RigConfigFile::Direct(bones) => bones,
            RigConfigFile::Mixamo(mixamo) => mixamo.bones,
        };

        let path = load_context.path().to_string();
        let rig_name = if path.contains("mixamo") {
            "mixamo".to_string()
        } else if path.contains("game_engine") {
            "game_engine".to_string()
        } else if path.contains("default") {
            "default".to_string()
        } else {
            "unknown".to_string()
        };

        let bones = bones
            .into_iter()
            .map(|(name, config)| (NAME_INTERNER.intern(&name).leak(), config))
            .collect();

        Ok(RigConfigAsset { bones, rig_name })
    }

    fn extensions(&self) -> &[&str] {
        // No extension: see TargetManifestAssetLoader — JSON is claimed by four
        // loaders, so typed loads are required.
        &[]
    }
}


```

## Source file: src/loaders/rig_weights.rs

{
  "path": "src/loaders/rig_weights.rs",
  "blob_sha": "7e7312b95b2da31546d103fc1d1de4858e9e77a1",
  "sha256": "93629108898991b58181c23b6c0e33e989f7a4e682fd451f6750dc74a9d0bf57",
  "lines": 71,
  "bytes": 2060,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 7
  },
  "url": "https://github.com/emberlightstudios/Humentity/blob/71af6f483c8c5a2623271124ca8e445ddc23db02/src/loaders/rig_weights.rs",
  "function_signature_candidates": [
    {
      "line": 30,
      "text": "async fn load("
    },
    {
      "line": 66,
      "text": "fn extensions(&self) -> &[&str] {"
    }
  ]
}

```text

use ahash::AHashMap;
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    ecs::intern::Internable,
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::NAME_INTERNER;

#[derive(Asset, TypePath, Clone, Debug)]
pub struct RigWeightsAsset {
    pub weights: AHashMap<&'static str, AHashMap<u16, f32>>,
    pub rig_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct WeightsFormat {
    pub weights: AHashMap<String, Vec<(u16, f32)>>,
}

#[derive(Default, TypePath)]
pub struct RigWeightsAssetLoader;

impl AssetLoader for RigWeightsAssetLoader {
    type Asset = RigWeightsAsset;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let weights = serde_json::from_slice::<WeightsFormat>(&bytes)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;

        let path = load_context.path().to_string();
        let rig_name = if path.contains("mixamo") {
            "mixamo".to_string()
        } else if path.contains("game_engine") {
            "game_engine".to_string()
        } else if path.contains("default") {
            "default".to_string()
        } else {
            "unknown".to_string()
        };

        let weights = weights
            .weights
            .into_iter()
            .map(|(k, v)| {
                (
                    NAME_INTERNER.intern(&k).leak(),
                    v.iter().copied().collect::<AHashMap<_, _>>(),
                )
            })
            .collect::<AHashMap<_, _>>();

        Ok(RigWeightsAsset { weights, rig_name })
    }

    fn extensions(&self) -> &[&str] {
        // No extension: see TargetManifestAssetLoader — JSON is claimed by four
        // loaders, so typed loads are required.
        &[]
    }
}


```
