//! Host-selected admission budgets; never inferred from checkpoint contents.
use super::{editor::EditorFileLimits, file::CoordinatedFileLimits, save::EditorSavePolicy};
use crate::{app::resource_checkpoint::RestoreLimits, scene_limits};
const MIB: usize = 1024 * 1024;
impl EditorSavePolicy {
    /// Desktop editor profile. Budgets reject oversized state; they never trim it.
    pub(crate) fn desktop() -> Self {
        let liquid = physics::liquid::Config::default();
        let resources = RestoreLimits {
            catalog: crate::import::catalog_checkpoint::Limits {
                assets: Default::default(),
                inputs: voxy_assets::ImportedArchiveLimits {
                    max_inputs: 16384,
                    max_input_bytes: crate::import::MODEL_INPUT_BYTES,
                    max_archive_bytes: 512 * MIB,
                },
                max_bytes: 768 * MIB,
                max_catalog_assets: 4096,
                max_pending: 4096,
                max_source_outputs: 16384,
                max_source_edges: 65536,
            },
            max_bytes: 1024 * MIB,
            max_outputs: 4096,
            max_queued: 4096,
            max_sources: 16384,
            max_file_bytes: crate::import::SOURCE_BYTES,
        };
        let text = crate::ui_text::checkpoint::UiTextArchiveLimits {
            imported: voxy_assets::ImportedArchiveLimits {
                max_inputs: 256,
                max_input_bytes: 16 * MIB,
                max_archive_bytes: 128 * MIB,
            },
            max_payload_bytes: 256 * MIB,
            max_fonts: 256,
            max_runs: 8192,
            max_pixels: 64 * MIB,
            max_glyphs_per_run: 65536,
            max_total_glyphs: 1_000_000,
            font: voxy_text::FontLimits {
                max_font_bytes: 16 * MIB,
                max_glyph_pixels: 1024 * 1024,
                max_size: 1024.0,
            },
        };
        let ui = crate::ui_live::checkpoint::UiLiveCheckpointLimits {
            max_bytes: 512 * MIB,
            max_reload_bytes: 32 * MIB,
            publications: crate::ui_live::UiPublicationLimits {
                max_bytes: 384 * MIB,
                max_layout_bytes: 16 * MIB,
                max_elements: scene_limits::OBJECTS,
                max_input_bytes: 128 * MIB,
                max_pixels: 64 * MIB,
                max_glyphs: 1_000_000,
                max_runs: 8192,
                text,
            },
        };
        Self {
            compatibility: "voxy-editor-desktop-v1".into(),
            resources,
            play: crate::play_session::checkpoint::PlayCheckpointLimits {
                core: voxy_gameplay::GameplayCheckpointLimits {
                    simulation: scene_limits::SIMULATION,
                    scene_capacity: scene_limits::OBJECTS,
                    max_bodies: 128,
                    max_colliders: 128,
                    max_actions: 4096,
                    max_bindings: 16384,
                    max_bytes: 256 * MIB,
                },
                max_angular_motions: scene_limits::OBJECTS,
                max_optical_materials: scene_limits::OBJECTS,
                ui: voxy_gameplay::UiActionCheckpointLimits {
                    max_events: 128,
                    max_handlers: 128,
                    max_commands_per_event: 128,
                    max_bytes: 16 * MIB,
                },
                liquid: voxy_gameplay::SceneLiquidCheckpointLimits {
                    liquid: physics::liquid::LiquidStateCheckpointLimits {
                        core: physics::liquid::LiquidCheckpointLimits {
                            max_particles: liquid.max_particles,
                            max_materials: 128,
                            max_pairs: liquid.max_pairs,
                            max_neighbor_checks: liquid.max_neighbor_checks,
                            max_substeps: liquid.max_substeps,
                        },
                        max_boundary_samples: 65536,
                        max_interfaces: 16384,
                        max_species: 128,
                        max_name_bytes: 65536,
                        max_reaction_attempts: 1_000_000,
                    },
                    max_sources: scene_limits::OBJECTS,
                    max_bodies: 128,
                    max_pulses_per_source: 4096,
                    max_binding_entries: 65536,
                },
                liquid_wire: voxy_gameplay::SceneLiquidWireLimits {
                    max_bytes: 256 * MIB,
                    max_allocation_bytes: 512 * MIB,
                    max_values: 32 * MIB,
                    liquid: physics::liquid::LiquidWireLimits {
                        max_bytes: 256 * MIB,
                        max_allocation_bytes: 512 * MIB,
                        max_values: 32 * MIB,
                    },
                },
            },
            file: EditorFileLimits {
                max_bytes: 2048 * MIB,
                game: CoordinatedFileLimits {
                    max_bytes: 1536 * MIB,
                    max_game_bytes: resources.max_bytes,
                    ui,
                    max_audio_bytes: 256 * MIB,
                    max_input_bytes: 16 * MIB,
                },
                history: voxy_scene::HistoryCheckpointLimits {
                    max_versions: 4096,
                    max_history_bytes: scene_limits::HISTORY_BYTES,
                    max_scene_capacity: scene_limits::OBJECTS,
                    max_archive_bytes: 256 * MIB,
                    max_edits: 4096,
                    max_draft_objects: scene_limits::OBJECTS * 2,
                },
            },
        }
    }
}
