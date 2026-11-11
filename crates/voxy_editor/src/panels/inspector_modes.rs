//! Numeric inspector modes (transform, physics, audio, mixer, material, behavior) and action buttons.

use glam::Vec2;
use voxy_render::SpriteBatch;
use voxy_scene::SceneObject;

use super::{Action, Panels};
use crate::InspectorMode;

pub(super) fn collect_mode_values(
    object: &SceneObject,
    inspector: InspectorMode,
    import_config: Option<voxy_gameplay::AudioImportConfig>,
) -> Vec<(String, f64)> {
    let (x, y, z) = glam::Quat::from_array(object.rotation).to_euler(glam::EulerRot::XYZ);
    let transform_values = [
        object.translation[0],
        object.translation[1],
        object.translation[2],
        x.to_degrees(),
        y.to_degrees(),
        z.to_degrees(),
        object.scale[0],
        object.scale[1],
        object.scale[2],
    ];
    let transform_labels = [
        "Pos X", "Pos Y", "Pos Z", "Rot X", "Rot Y", "Rot Z", "Scale X", "Scale Y", "Scale Z",
    ];
    let mut values: Vec<(String, f64)> = transform_labels
        .into_iter()
        .zip(transform_values)
        .map(|(label, value)| (label.into(), f64::from(value)))
        .collect();
    match inspector {
        InspectorMode::Physics => {
            values.clear();
            if let Some(body) = object.components.get("game.character.v1") {
                for (index, label) in ["Half X", "Half Y", "Half Z"].into_iter().enumerate() {
                    values.push((
                        label.into(),
                        body["half_extents"][index].as_f64().unwrap_or(0.0),
                    ));
                }
                for (field, label) in [
                    ("speed", "Speed"),
                    ("gravity", "Gravity"),
                    ("jump_speed", "Jump speed"),
                ] {
                    values.push((label.into(), body[field].as_f64().unwrap_or(0.0)));
                }
            } else if let Some(collider) = object.components.get("game.box.v1") {
                for (index, label) in ["Half X", "Half Y", "Half Z"].into_iter().enumerate() {
                    values.push((
                        label.into(),
                        collider["half_extents"][index].as_f64().unwrap_or(0.0),
                    ));
                }
            }
            if let Some(trigger) = object.components.get("game.box-trigger.v1") {
                for (index, label) in ["Trigger half X", "Trigger half Y", "Trigger half Z"]
                    .into_iter()
                    .enumerate()
                {
                    values.push((
                        label.into(),
                        trigger["half_extents"][index].as_f64().unwrap_or(0.),
                    ));
                }
            }
            let layers = object.components.get("game.collision-layers.v1");
            let defaults = voxy_gameplay::CollisionLayers::default();
            for (field, label, default) in [
                ("membership", "Layer bits", defaults.membership),
                ("mask", "Mask bits", defaults.mask),
            ] {
                let value = layers
                    .and_then(|layers| layers[field].as_f64())
                    .unwrap_or(f64::from(default));
                values.push((label.into(), value));
            }
            if let Some(body) = object.components.get("game.liquid-body.v1") {
                values.push(("Mass (kg)".into(), body["mass_kg"].as_f64().unwrap_or(0.)));
                for (axis, label) in ["Velocity X", "Velocity Y", "Velocity Z"]
                    .into_iter()
                    .enumerate()
                {
                    values.push((
                        label.into(),
                        body["initial_velocity_m_s"][axis].as_f64().unwrap_or(0.),
                    ));
                }
            }
            if let Some(body) = object.components.get("game.kinematic-body.v1") {
                for (axis, label) in ["Velocity X", "Velocity Y", "Velocity Z"]
                    .into_iter()
                    .enumerate()
                {
                    values.push((
                        label.into(),
                        body["velocity_m_s"][axis].as_f64().unwrap_or(0.),
                    ));
                }
            }
        }
        InspectorMode::ImportSettings => {
            values.clear();
            if let Some(config) = import_config {
                values.extend([
                    ("Bytes".into(), config.max_input_bytes as f64),
                    ("Frames".into(), config.max_frames as f64),
                    ("Filter work".into(), config.max_filter_evaluations as f64),
                ]);
            }
        }
        InspectorMode::Mixer => {
            values.clear();
            if let Some(bus) = object.components.get("game.audio-bus.v1") {
                for (key, label) in [("bus", "Bus 0..15"), ("gain", "Gain 0..1")] {
                    values.push((label.into(), bus[key].as_f64().unwrap_or(0.)));
                }
            }
        }
        InspectorMode::Audio => {
            values.clear();
            if let Some(source) = object.components.get("game.audio-source.v1") {
                for (field, label) in [("gain", "Gain"), ("bus", "Bus")] {
                    values.push((label.into(), source[field].as_f64().unwrap_or(0.)));
                }
                for (field, label) in [("looping", "Loop 0/1"), ("spatial", "Spatial 0/1")] {
                    values.push((
                        label.into(),
                        if source[field].as_bool().unwrap_or(false) {
                            1.
                        } else {
                            0.
                        },
                    ));
                }
                for (field, label) in [("near", "Near"), ("far", "Far")] {
                    values.push((label.into(), source[field].as_f64().unwrap_or(0.)));
                }
            }
        }
        InspectorMode::Behavior => {
            values.clear();
            if let Some(motion) = object.components.get("game.angular-motion.v1") {
                for (index, label) in ["Axis X", "Axis Y", "Axis Z"].into_iter().enumerate() {
                    values.push((label.into(), motion["axis"][index].as_f64().unwrap_or(0.)));
                }
                values.push((
                    "Radians/sec".into(),
                    motion["radians_per_second"].as_f64().unwrap_or(0.),
                ));
            }
        }
        InspectorMode::Material => {
            values.clear();
            if let Some(material) = object.components.get("editor.material.v1") {
                for (index, label) in ["Tint R", "Tint G", "Tint B", "Tint A"]
                    .into_iter()
                    .enumerate()
                {
                    values.push((label.into(), material["tint"][index].as_f64().unwrap_or(1.)));
                }
                values.push((
                    "Lit 0/1".into(),
                    if material["lit"].as_bool().unwrap_or(true) {
                        1.
                    } else {
                        0.
                    },
                ));
            }
        }
        InspectorMode::Transform
        | InspectorMode::Components(_)
        | InspectorMode::Collections(_, _) => {}
    }
    values
}

impl Panels {
    #[expect(
        clippy::too_many_arguments,
        reason = "inspector footer buttons depend on current inspector mode, playing state, and column layout"
    )]
    pub(super) fn build_inspector_footer(
        &mut self,
        batch: &mut SpriteBatch,
        object: &SceneObject,
        inspector: InspectorMode,
        playing: bool,
        field: Option<(usize, &str)>,
        rx: f32,
        right: f32,
        size: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let audio_mode = inspector == InspectorMode::Audio;
        for (row, label, action) in [
            (
                5,
                if inspector == InspectorMode::Physics {
                    if object.components.contains_key("game.box-trigger.v1") {
                        "Remove box trigger"
                    } else {
                        "Add box trigger"
                    }
                } else if object.components.contains_key("game.follow-camera.v1") {
                    "Remove follow camera"
                } else {
                    "Add follow camera"
                },
                if inspector == InspectorMode::Physics {
                    Action::Trigger
                } else {
                    Action::FollowCamera(crate::camera_authoring::request_token(object)?)
                },
            ),
            (
                3,
                if self.retarget_draft {
                    "Add bone pair"
                } else if inspector == InspectorMode::Physics {
                    if object.components.contains_key("game.liquid-body.v1") {
                        "Remove dynamic body"
                    } else {
                        "Add dynamic body"
                    }
                } else if matches!(
                    inspector,
                    InspectorMode::Audio | InspectorMode::ImportSettings
                ) {
                    "Open/reload import settings"
                } else if matches!(inspector, InspectorMode::Components(_)) {
                    if object.components.contains_key("editor.model-animation.v1") {
                        "Remove animation"
                    } else {
                        "Add animation"
                    }
                } else if object.components.contains_key("game.angular-motion.v1") {
                    "Remove angular motion"
                } else {
                    "Add angular motion"
                },
                if matches!(
                    inspector,
                    InspectorMode::Audio | InspectorMode::ImportSettings
                ) {
                    Action::AudioSettingsLoad
                } else if self.retarget_draft {
                    Action::RetargetPair(true)
                } else if inspector == InspectorMode::Physics {
                    Action::DynamicBody
                } else if matches!(inspector, InspectorMode::Components(_)) {
                    Action::Animation
                } else {
                    Action::Motion
                },
            ),
            (
                4,
                if self.retarget_draft {
                    "Remove last bone pair"
                } else {
                    match inspector {
                        InspectorMode::Behavior | InspectorMode::ImportSettings => "Audio fields",
                        InspectorMode::Audio => "Mixer fields",
                        InspectorMode::Mixer => "Component fields",
                        InspectorMode::Components(_) => "Collections",
                        InspectorMode::Collections(_, _) => "Transform fields",
                        _ => "Behavior fields",
                    }
                },
                if self.retarget_draft {
                    Action::RetargetPair(false)
                } else {
                    Action::Behavior
                },
            ),
            (
                0,
                if self.retarget_draft {
                    "Apply retarget profile"
                } else if inspector == InspectorMode::ImportSettings {
                    "Save import settings"
                } else if inspector == InspectorMode::Mixer {
                    if object.components.contains_key("game.audio-bus.v1") {
                        "Remove audio bus"
                    } else {
                        "Add audio bus"
                    }
                } else if audio_mode {
                    if object.components.contains_key("game.audio-source.v1") {
                        "Remove audio source"
                    } else {
                        "Add audio source"
                    }
                } else if object.components.contains_key("game.character.v1") {
                    "Remove character (C)"
                } else {
                    "Add character (C)"
                },
                if self.retarget_draft {
                    Action::RetargetApply
                } else if inspector == InspectorMode::ImportSettings {
                    Action::AudioSettingsSave
                } else if inspector == InspectorMode::Mixer {
                    Action::AudioBus
                } else if audio_mode {
                    Action::AudioSource
                } else {
                    Action::Character
                },
            ),
            (
                1,
                if self.retarget_draft {
                    "Cancel profile edit"
                } else if matches!(inspector, InspectorMode::Components(_)) {
                    "Edit retarget profile"
                } else if audio_mode {
                    if object.components.contains_key("game.audio-listener.v1") {
                        "Remove audio listener"
                    } else {
                        "Add audio listener"
                    }
                } else if object.components.contains_key("game.box.v1") {
                    "Remove collider (B)"
                } else {
                    "Add collider (B)"
                },
                if self.retarget_draft {
                    Action::RetargetCancel
                } else if matches!(inspector, InspectorMode::Components(_)) {
                    Action::Retarget
                } else if audio_mode {
                    Action::AudioListener
                } else {
                    Action::Collider
                },
            ),
            (
                2,
                if self.retarget_draft {
                    "Remove retarget profile"
                } else if inspector == InspectorMode::Physics {
                    "Transform fields (I)"
                } else {
                    "Physics fields (I)"
                },
                if self.retarget_draft {
                    Action::RetargetRemove
                } else {
                    Action::Physics
                },
            ),
        ] {
            if row == 5
                && (self.retarget_draft
                    || !matches!(
                        inspector,
                        InspectorMode::Components(_) | InspectorMode::Physics
                    ))
            {
                continue;
            }
            let y = 354.0
                + if matches!(inspector, InspectorMode::Components(_)) && !self.retarget_draft {
                    64.0
                } else {
                    0.0
                }
                + row as f32 * 24.0
                + if matches!(inspector, InspectorMode::Collections(_, _)) {
                    30.0
                } else {
                    0.0
                };
            self.text(
                batch,
                label,
                Vec2::new(rx + 8.0, y + 17.0),
                right - 12.0,
                size,
            )?;
            self.regions
                .push(([rx + 4.0, y, right - 8.0, 23.0], action));
        }
        if inspector == InspectorMode::Physics {
            let label = if object.components.contains_key("game.kinematic-body.v1") {
                "Remove kinematic body"
            } else {
                "Add kinematic body"
            };
            let y = 498.0;
            self.text(
                batch,
                label,
                Vec2::new(rx + 8.0, y + 17.0),
                right - 12.0,
                size,
            )?;
            self.regions
                .push(([rx + 4.0, y, right - 8.0, 23.0], Action::KinematicBody));
        }
        self.text(
            batch,
            if playing {
                "Authoring values"
            } else if field.is_none() {
                "Tab / Shift+Tab focus"
            } else {
                "Type, Enter applies"
            },
            Vec2::new(
                rx + 8.0,
                if matches!(inspector, InspectorMode::Collections(_, _)) {
                    521.0
                } else if matches!(inspector, InspectorMode::Components(_)) && !self.retarget_draft
                {
                    579.0
                } else if inspector == InspectorMode::Physics && !self.retarget_draft {
                    539.0
                } else {
                    491.0
                },
            ),
            right - 12.0,
            size,
        )?;
        self.text(
            batch,
            if playing {
                "Stop to edit"
            } else if field.is_none() {
                "Enter / Space activate"
            } else {
                "Escape cancels"
            },
            Vec2::new(
                rx + 8.0,
                if matches!(inspector, InspectorMode::Collections(_, _)) {
                    544.0
                } else if matches!(inspector, InspectorMode::Components(_)) && !self.retarget_draft
                {
                    602.0
                } else if inspector == InspectorMode::Physics && !self.retarget_draft {
                    538.0
                } else {
                    514.0
                },
            ),
            right - 12.0,
            size,
        )?;
        Ok(())
    }
}
