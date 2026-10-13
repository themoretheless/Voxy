//! Inspect authored foot trajectories with the engine's native f64 sampler.
//! This emits measurements, not inferred physical contacts or a second sampler.
use serde_json::json;
use voxy_model::{ModelAsset, ModelLimits};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut model = ModelAsset::parse(
        include_bytes!(
            "../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.gltf"
        ),
        &[include_bytes!(
            "../../../assets/animation/quaternius-universal/glTF/AnimationLibrary_Godot_Standard.bin"
        )],
        ModelLimits::default(),
    )?;
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("--inspect-rig-pivots") => return inspect_rig_pivots(&model),
        Some("--bake-contacts") => return bake_contacts(&mut model, false, false),
        Some("--bake-toe-contacts") => return bake_contacts(&mut model, true, false),
        Some("--analyze-contact-thresholds") => return bake_contacts(&mut model, true, true),
        Some("--analyze-ankle-thresholds") => return bake_contacts(&mut model, false, true),
        Some(_) => {
            return Err("choose trajectories, --bake-contacts or --bake-toe-contacts".into());
        }
    }
    let names = [
        "DEF-hips",
        "DEF-foot.L",
        "DEF-toe.L",
        "DEF-foot.R",
        "DEF-toe.R",
    ];
    let joints = names
        .map(|name| model.resolve_joint_name(name).map(usize::from))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mut clips = Vec::new();
    for name in ["Idle_Loop", "Walk_Loop", "Jog_Fwd_Loop", "Sprint_Loop"] {
        let index = model
            .animations
            .iter()
            .position(|clip| clip.name() == name)
            .ok_or_else(|| format!("missing authored clip {name}"))?;
        let mut samples = Vec::new();
        for tick in 0..=240 {
            let phase = f64::from(tick) / 240.;
            let globals = model
                .sample_pose_phase64(Some(index), phase)?
                .global_matrices(&model.skeleton)?;
            let positions: Vec<_> = joints
                .iter()
                .map(|&joint| globals[joint].w_axis.truncate().to_array())
                .collect();
            samples.push(json!({"phase":phase,"positions":positions}));
        }
        clips.push(
            json!({"name":name,"duration_s":model.animations[index].duration(),"samples":samples}),
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "source_revision":"e24c23cf2a1323488a3faa226ea7ea21f644b73e",
            "coordinates":"imported model space; no root extraction or IK",
            "joints":names,"contacts_inferred":false,"clips":clips,
        }))?
    );
    Ok(())
}

fn bake_contacts(
    model: &mut ModelAsset,
    toes: bool,
    analyze: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::{collections::BTreeMap, sync::Arc};
    use voxy_animation::{ContactBakeSettings, bake_periodic_foot_contacts};
    use voxy_editor::FootContactKey;
    let recipe: serde_json::Value = serde_json::from_str(include_str!(
        "../../../assets/animation/quaternius-universal/GAIT_PREPARATION.json"
    ))?;
    let clip_name = recipe["clip"].as_str().ok_or("missing preparation clip")?;
    let index = model
        .animations
        .iter()
        .position(|c| c.name() == clip_name)
        .ok_or("missing preparation clip")?;
    let tail = recipe["tail_seconds"]
        .as_f64()
        .ok_or("missing closing interval")?;
    model.animations[index] =
        Arc::new(model.animations[index].append_loop_frame(&model.skeleton, tail as f32)?);
    let settings = ContactBakeSettings {
        height_threshold: 0.06,
        vertical_speed_threshold: 0.3,
    };
    let sample_rate = 24.;
    let mut maps: [BTreeMap<String, Vec<FootContactKey>>; 2] =
        std::array::from_fn(|_| BTreeMap::new());
    let mut diagnostics = Vec::new();
    let mut calibration = Vec::new();
    let names = if toes {
        ["DEF-toe.L", "DEF-toe.R"]
    } else {
        ["DEF-foot.L", "DEF-foot.R"]
    };
    let feet = names
        .map(|name| model.resolve_joint_name(name).map(usize::from))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    for name in ["Idle_Loop", "Walk_Loop", "Jog_Fwd_Loop", "Sprint_Loop"] {
        let index = model
            .animations
            .iter()
            .position(|c| c.name() == name)
            .ok_or("missing gait")?;
        let duration = f64::from(model.animations[index].duration());
        let samples = (duration * sample_rate).round() as usize;
        let mut heights: [Vec<f64>; 2] = std::array::from_fn(|_| Vec::with_capacity(samples));
        let mut positions: [Vec<[f64; 3]>; 2] =
            std::array::from_fn(|_| Vec::with_capacity(samples));
        for sample in 0..samples {
            let globals = model
                .sample_pose_phase64(Some(index), sample as f64 / samples as f64)?
                .global_matrices(&model.skeleton)?;
            for side in 0..2 {
                heights[side].push(globals[feet[side]].w_axis.y);
                positions[side].push(globals[feet[side]].w_axis.truncate().to_array());
            }
        }
        for side in 0..2 {
            let baked = bake_periodic_foot_contacts(&heights[side], duration, settings)?;
            if analyze {
                let dt = duration / samples as f64;
                let velocities: Vec<[f64; 3]> = (0..samples)
                    .map(|i| {
                        let previous = positions[side][(i + samples - 1) % samples];
                        let next = positions[side][(i + 1) % samples];
                        std::array::from_fn(|axis| (next[axis] - previous[axis]) / (2. * dt))
                    })
                    .collect();
                let mut trials = Vec::new();
                for height_threshold in [0.02, 0.04, 0.06, 0.08, 0.12] {
                    for vertical_speed_threshold in [0.15, 0.3, 0.6, 1.2, 2.4] {
                        let trial = bake_periodic_foot_contacts(
                            &heights[side],
                            duration,
                            ContactBakeSettings {
                                height_threshold,
                                vertical_speed_threshold,
                            },
                        )?;
                        let transitions = (0..samples)
                            .filter(|&i| {
                                trial.contact[i] != trial.contact[(i + samples - 1) % samples]
                            })
                            .count();
                        let active: Vec<_> = velocities
                            .iter()
                            .zip(&trial.contact)
                            .filter_map(|(v, &plant)| plant.then_some(v))
                            .collect();
                        let planar_fit = if active.is_empty() {
                            None
                        } else {
                            let mean: [f64; 2] = [0, 2].map(|axis| {
                                active.iter().map(|v| v[axis]).sum::<f64>() / active.len() as f64
                            });
                            let residual = (active
                                .iter()
                                .map(|v| (v[0] - mean[0]).powi(2) + (v[2] - mean[1]).powi(2))
                                .sum::<f64>()
                                / active.len() as f64)
                                .sqrt();
                            Some(
                                json!({"canceling_actor_velocity_xz_m_s":[-mean[0],-mean[1]],
                                "remaining_planar_velocity_rms_m_s":residual}),
                            )
                        };
                        trials.push(json!({"height_threshold_m":height_threshold,
                            "vertical_speed_threshold_m_s":vertical_speed_threshold,
                            "plant_samples":trial.contact.iter().filter(|p| **p).count(),
                            "transition_count":transitions,"constant_actor_translation_fit":planar_fit}));
                    }
                }
                calibration.push(json!({"clip":name,"side":side,"duration_s":duration,
                    "height_m":heights[side],"vertical_speed_m_s":baked.vertical_speed,
                    "position_m":positions[side],"velocity_m_s":velocities,
                    "trials":trials}));
            }
            let mut keys: Vec<_> = baked
                .contact
                .iter()
                .enumerate()
                .map(|(i, &plant)| FootContactKey {
                    phase: i as f32 / samples as f32,
                    weight: if plant { 1. } else { 0. },
                })
                .collect();
            keys.push(FootContactKey {
                phase: 1.,
                weight: keys[0].weight,
            });
            maps[side].insert(name.into(), keys);
            let planted = baked.contact.iter().filter(|p| **p).count();
            let changes: Vec<_> = (0..samples)
                .filter(|&i| baked.contact[i] != baked.contact[(i + samples - 1) % samples])
                .collect();
            diagnostics.push(json!({"clip":name,"side":if side==0 {"left"} else {"right"},
                "duration_s":duration,"samples_in_cycle":samples,"minimum_site_height_m":baked.minimum_height,
                "candidate_plant_percent":100.*planted as f64/samples as f64,"transition_samples":changes}));
        }
    }
    if analyze {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema":"voxy.contact-threshold-analysis.v1",
                "source_revision":"e24c23cf2a1323488a3faa226ea7ea21f644b73e",
                "preparation_recipe":recipe["recipe"],"sample_rate_hz":sample_rate,
                "sampled_bones":names,"samples_exclude_repeated_endpoint":true,
                "physical_contacts_verified":false,"thresholds_selected":false,
                "measurements":calibration
            }))?
        );
        return Ok(());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"voxy.named-foot-contact-candidates.v1",
            "status":"offline_candidates_require_author_review",
            "source_revision":"e24c23cf2a1323488a3faa226ea7ea21f644b73e",
            "preparation_recipe":recipe["recipe"],"sample_rate_hz":sample_rate,
            "up_axis":"model-space Y","sampled_bones":names,
            "height_threshold_m":settings.height_threshold,"vertical_speed_threshold_m_s":settings.vertical_speed_threshold,
            "curves":{"left":maps[0],"right":maps[1]},"diagnostics":diagnostics,
            "physical_contacts_verified":false
        }))?
    );
    Ok(())
}

fn inspect_rig_pivots(model: &ModelAsset) -> Result<(), Box<dyn std::error::Error>> {
    let mut sites = Vec::new();
    for side in ["L", "R"] {
        let ankle = usize::from(model.resolve_joint_name(&format!("DEF-foot.{side}"))?);
        let toe = usize::from(model.resolve_joint_name(&format!("DEF-toe.{side}"))?);
        let mut records = Vec::new();
        let mut reference = None;
        let mut max_local_drift = 0f64;
        let mut max_reconstruction_error = 0f64;
        for name in ["Idle_Loop", "Walk_Loop", "Jog_Fwd_Loop", "Sprint_Loop"] {
            let index = model
                .animations
                .iter()
                .position(|clip| clip.name() == name)
                .ok_or("missing gait")?;
            for sample in 0..=240 {
                let phase = sample as f64 / 240.;
                let globals = model
                    .sample_pose_phase64(Some(index), phase)?
                    .global_matrices(&model.skeleton)?;
                let inverse = globals[ankle].inverse();
                if !inverse.is_finite() {
                    return Err("singular ankle transform".into());
                }
                let position = globals[toe].w_axis.truncate();
                let local = inverse.transform_point3(position);
                let baseline = *reference.get_or_insert(local);
                max_local_drift = max_local_drift.max(local.distance(baseline));
                max_reconstruction_error = max_reconstruction_error
                    .max(globals[ankle].transform_point3(local).distance(position));
                records.push(json!({"clip":name,"phase":phase,"toe_in_ankle":local.to_array()}));
            }
        }
        sites.push(json!({"side":side,"ankle_joint":ankle,"toe_joint":toe,
            "toe_parent_is_ankle":model.skeleton.joints()[toe].parent==Some(ankle as u16),
            "reference_toe_in_ankle":reference.unwrap().to_array(),
            "max_local_drift_m":max_local_drift,
            "max_roundtrip_error_m":max_reconstruction_error,"samples":records}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":"voxy.source-rig-pivot-inspection.v1",
            "source_revision":"e24c23cf2a1323488a3faa226ea7ea21f644b73e",
            "raw_source_clips":true,"physical_surface_pivots_verified":false,
            "units":"imported model meters; ankle local coordinates",
            "sites":sites
        }))?
    );
    Ok(())
}
