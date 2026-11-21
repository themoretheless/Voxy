//! Catalog and codec verification of all 30 registered Voxy scene/gameplay component schemas.

use serde::Serialize;
use serde_json::{Value, json};
use voxy_gameplay::{
    AngularMotion, AudioBus, AudioListener, AudioSource, BoxCollider, BoxTrigger, CharacterBody,
    CollisionLayers, FollowCamera, KinematicBody, LiquidBody, LiquidMassDistribution,
    LiquidMassPart, LiquidSource, NpcDestination, NpcRoute, PhysicsJoint, PhysicsJointKind,
    RigidPointForce, RigidPointLoads, UiElement,
};
use voxy_scene::ObjectId;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SchemaEntry {
    pub schema: &'static str,
    pub category: &'static str,
    pub summary: &'static str,
    pub sample: Value,
}

pub(crate) fn all_schemas() -> Vec<SchemaEntry> {
    let mut entries = editor_and_scene_schemas();
    entries.extend(physics_and_liquid_schemas());
    entries.extend(gameplay_motion_and_nav_schemas());
    entries.extend(audio_and_ui_schemas());
    entries
}

pub(crate) fn find_schema(name: &str) -> Option<SchemaEntry> {
    all_schemas().into_iter().find(|s| s.schema == name)
}

pub(crate) fn filter_schemas(category: Option<&str>, query: Option<&str>) -> Vec<SchemaEntry> {
    all_schemas()
        .into_iter()
        .filter(|e| category.is_none_or(|c| e.category.eq_ignore_ascii_case(c)))
        .filter(|e| {
            query.is_none_or(|q| {
                let ql = q.to_ascii_lowercase();
                e.schema.to_ascii_lowercase().contains(&ql)
                    || e.summary.to_ascii_lowercase().contains(&ql)
            })
        })
        .collect()
}

fn entry<T: Serialize>(
    schema: &'static str,
    category: &'static str,
    summary: &'static str,
    sample: &T,
) -> SchemaEntry {
    SchemaEntry {
        schema,
        category,
        summary,
        sample: serde_json::to_value(sample).expect("component sample serializes"),
    }
}

fn editor_and_scene_schemas() -> Vec<SchemaEntry> {
    vec![
        entry(
            "editor.model.v1",
            "editor",
            "Logical model asset ID string.",
            &"model".to_owned(),
        ),
        entry(
            "editor.model-part.v1",
            "editor",
            "Imported mesh sub-node index within a model asset.",
            &crate::ModelPart { node: 0 },
        ),
        entry(
            "editor.model-animation.v1",
            "editor",
            "Skeletal playback clip, speed, looping, morph weights, and animation events.",
            &crate::ModelAnimation::default(),
        ),
        entry(
            "editor.model-locomotion.v1",
            "editor",
            "Automatic idle/move/rise/fall animation state machine clips.",
            &crate::ModelLocomotion {
                idle: "idle".into(),
                moving: "walk".into(),
                rising: "jump".into(),
                falling: "fall".into(),
                speed_threshold: 0.1,
                transition_seconds: 0.15,
            },
        ),
        SchemaEntry {
            schema: "editor.foot-placement.v1",
            category: "editor",
            summary: "Two-bone foot IK contact bindings.",
            sample: json!({ "feet": [] }),
        },
        SchemaEntry {
            schema: "editor.model-retarget.v1",
            category: "editor",
            summary: "Skeleton retargeting source model and joint mappings.",
            sample: json!({ "source": "walk.glb", "joints": [{ "source": "hip", "target": "pelvis" }] }),
        },
        entry(
            "editor.material.v1",
            "render",
            "Surface base RGBA tint and lit shading toggle.",
            &crate::SceneMaterial::default(),
        ),
        entry(
            "editor.light.v1",
            "render",
            "Scene directional light direction, color, and intensity.",
            &crate::DirectionalLight::default(),
        ),
        entry(
            "editor.directional-shadow.v1",
            "render",
            "Orthographic shadow frustum, resolution, bias, and PCF filter.",
            &crate::DirectionalShadow::default(),
        ),
        entry(
            "editor.opaque-shadow-caster.v1",
            "render",
            "Marks an opaque object as a directional shadow caster.",
            &crate::OpaqueShadowCaster::default(),
        ),
        entry(
            "scene.fog.v1",
            "scene",
            "Uniform volumetric fog box with extinction, albedo, and anisotropy.",
            &voxy_scene::FogVolume::default(),
        ),
        entry(
            "editor.camera.v1",
            "editor",
            "Orbit viewport camera target, distance, yaw/pitch, and projection.",
            &crate::EditorCamera::default(),
        ),
    ]
}

fn physics_and_liquid_schemas() -> Vec<SchemaEntry> {
    let target = ObjectId("target".into());
    vec![
        entry(
            "game.follow-camera.v1",
            "gameplay",
            "Smoothed presentation camera following a target ObjectId.",
            &FollowCamera {
                target: target.clone(),
                offset: [0.0, 1.5, 4.0],
                half_life: 0.15,
            },
        ),
        entry(
            "game.physics-joint.v1",
            "physics",
            "Fixed, hinge, or distance-spring constraint linking owner to target ObjectId.",
            &PhysicsJoint {
                target,
                target_anchor: [0.0, 0.0, 0.0],
                local_anchor: [0.0, 0.0, 0.0],
                kind: PhysicsJointKind::Fixed {
                    relative_rotation: [0.0, 0.0, 0.0, 1.0],
                },
            },
        ),
        entry(
            "game.rigid-point-loads.v1",
            "physics",
            "Material-point world forces and force rates applied to a rigid body.",
            &RigidPointLoads {
                points: vec![RigidPointForce {
                    root_point_m: [0.0, 0.0, 0.0],
                    world_force_n: [0.0, 0.0, 0.0],
                    world_force_rate_n_s: [0.0, 0.0, 0.0],
                }],
            },
        ),
        entry(
            "game.liquid-body.v1",
            "physics",
            "Coupled fluid/rigid-body mass and initial linear velocity.",
            &LiquidBody {
                mass_kg: 10.0,
                initial_velocity_m_s: [0.0, 0.0, 0.0],
            },
        ),
        entry(
            "game.liquid-mass.v1",
            "physics",
            "Explicit cuboid inertial constituent boxes for a coupled liquid body.",
            &LiquidMassDistribution {
                parts: vec![LiquidMassPart {
                    mass_kg: 10.0,
                    center_m: [0.0, 0.0, 0.0],
                    half_edges_m: [[0.5, 0.0, 0.0], [0.0, 0.5, 0.0], [0.0, 0.0, 0.5]],
                }],
            },
        ),
        entry(
            "game.liquid-source.v1",
            "physics",
            "Fluid nozzle emitter with pulse schedule and material asset reference.",
            &LiquidSource {
                pulses: Vec::new(),
                density_kg_m3: 1000.0,
                particle_volume_m3: 1e-4,
                nozzle_radius_m: 0.02,
                direction: [0.0, -1.0, 0.0],
                material_asset: "water-material".into(),
            },
        ),
        entry(
            "game.box.v1",
            "physics",
            "Axis-aligned static or body-attached box collider half-extents.",
            &BoxCollider::default(),
        ),
        entry(
            "game.collision-layers.v1",
            "physics",
            "Collision layer membership and interaction bitmasks.",
            &CollisionLayers::default(),
        ),
        entry(
            "game.box-trigger.v1",
            "physics",
            "Axis-aligned trigger volume half-extents.",
            &BoxTrigger {
                half_extents: [0.5, 0.5, 0.5],
            },
        ),
    ]
}

fn gameplay_motion_and_nav_schemas() -> Vec<SchemaEntry> {
    vec![
        entry(
            "game.kinematic-body.v1",
            "physics",
            "Constant-velocity kinematic solid root.",
            &KinematicBody {
                velocity_m_s: [0.0, 0.0, 0.0],
            },
        ),
        entry(
            "game.character.v1",
            "gameplay",
            "Character controller box half-extents, speed, gravity, and jump speed.",
            &CharacterBody::default(),
        ),
        SchemaEntry {
            schema: "game.npc-route.v1",
            category: "gameplay",
            summary: "Fixed world-space waypoint sequence for NPC navigation.",
            sample: serde_json::to_value(
                NpcRoute::new(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]).expect("valid sample route"),
            )
            .expect("route serializes"),
        },
        entry(
            "game.npc-destination.v1",
            "gameplay",
            "Navmesh grid bounds, clearance, and goal coordinates for NPC navigation.",
            &NpcDestination {
                origin: [-2.0, -2.0],
                cells: [8, 8],
                cell_size: 0.5,
                floor_y: [0.0, 0.0],
                max_step_height: 0.3,
                max_slope_tan: 1.0,
                min_clearance: 0.4,
                goal: [1.0, 1.0],
            },
        ),
        entry(
            "game.angular-motion.v1",
            "gameplay",
            "Continuous fixed-step rotation around a local axis.",
            &AngularMotion {
                axis: [0.0, 1.0, 0.0],
                radians_per_second: 1.0,
            },
        ),
    ]
}

fn audio_and_ui_schemas() -> Vec<SchemaEntry> {
    vec![
        entry(
            "game.audio-bus.v1",
            "audio",
            "Audio mixer bus index (0..16) and linear gain [0..1].",
            &AudioBus { bus: 0, gain: 0.8 },
        ),
        entry(
            "game.audio-listener.v1",
            "audio",
            "Spatial audio listener attached to an object transform.",
            &AudioListener {},
        ),
        entry(
            "game.audio-source.v1",
            "audio",
            "WAV audio asset emitter with bus, gain, loop, and attenuation.",
            &AudioSource {
                asset: "sound".into(),
                import_settings: None,
                bus: 0,
                gain: 1.0,
                looping: false,
                spatial: true,
                near: 1.0,
                far: 20.0,
            },
        ),
        entry(
            "game.ui-element.v1",
            "ui",
            "Screen-space UI quad/text widget with layer, color, and action ID.",
            &UiElement {
                origin: [16.0, 16.0],
                size: [160.0, 40.0],
                color: [0.2, 0.4, 0.8, 1.0],
                layer: 0,
                enabled: true,
                action: None,
                text: None,
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use voxy_scene::{SceneDocument, SceneObject};

    #[test]
    fn catalog_covers_and_roundtrips_all_thirty_registered_schemas() {
        let registry = crate::editor_component_registry().unwrap();
        let schemas = all_schemas();
        assert_eq!(schemas.len(), 30);
        for entry in schemas {
            let obj_target = SceneObject {
                id: ObjectId("target".into()),
                parent: None,
                name: "target".into(),
                active: true,
                translation: [0.0, 0.0, 0.0],
                precise_translation_bits: None,
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
                components: BTreeMap::new(),
            };
            let mut components = BTreeMap::new();
            components.insert(entry.schema.to_owned(), entry.sample.clone());
            let obj_owner = SceneObject {
                id: ObjectId("owner".into()),
                parent: None,
                name: "owner".into(),
                active: true,
                translation: [1.0, 0.0, 0.0],
                precise_translation_bits: None,
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
                components,
            };
            let doc = SceneDocument {
                version: 1,
                objects: vec![obj_target, obj_owner],
            };
            let loaded = doc
                .load(&registry, 8)
                .unwrap_or_else(|e| panic!("schema {} failed codec roundtrip: {e}", entry.schema));
            let saved = loaded.capture(&registry).unwrap();
            let owner = saved
                .objects
                .iter()
                .find(|o| o.id.0 == "owner")
                .expect("owner object captured");
            assert!(
                owner.components.contains_key(entry.schema),
                "schema {} did not survive capture()",
                entry.schema
            );
        }
    }
}
