//! CPU pose/certificate/camera diagnostics for an actual skeletal LOD archive.
//! Usage: profile_skinned_lod BASE.glb CERTIFICATE.lod OUTPUT.csv
use glam::{Mat4, Vec3};
use std::{
    error::Error,
    io::{Read, Write},
    path::Path,
    time::Instant,
};
use voxy_render::{
    LodArchiveLimits, LodPolicy, ModelAsset, ModelGeometry, ModelLimits, SceneCamera,
    SceneProjection,
};
fn read(path: &Path, cap: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut data = Vec::new();
    std::fs::File::open(path)?
        .take(cap + 1)
        .read_to_end(&mut data)?;
    if data.len() as u64 > cap {
        return Err("input limit exceeded".into());
    }
    Ok(data)
}
fn camera(span: f32) -> SceneCamera {
    SceneCamera {
        eye: Vec3::new(0., 0., 10.),
        target: Vec3::ZERO,
        up: Vec3::Y,
        projection: SceneProjection::Orthographic {
            left: -span / 2.,
            right: span / 2.,
            bottom: -span / 2.,
            top: span / 2.,
            near: 0.1,
            far: 100.,
        },
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: profile_skinned_lod BASE.glb CERTIFICATE.lod OUTPUT.csv".into());
    }
    let model = ModelAsset::parse(
        &read(Path::new(&args[0]), 64 * 1024 * 1024)?,
        &[],
        ModelLimits::default(),
    )?;
    if model.primitives.len() != 1 || model.animations.is_empty() {
        return Err("one animated skeletal primitive required".into());
    }
    let ModelGeometry::Skinned(mesh) = &model.primitives[0].geometry else {
        return Err("skin required".into());
    };
    let limits = LodArchiveLimits {
        bytes: 16 * 1024 * 1024,
        positions: 65_536,
        levels: 8,
        indices: 1_572_864,
        cells: 262_144,
    };
    let lod = voxy_render::decode_skinned_lod_archive(
        std::sync::Arc::new(mesh.clone()),
        &read(Path::new(&args[1]), 16 * 1024 * 1024)?,
        limits,
    )?;
    if lod.indices(1).is_none() || lod.indices(1).unwrap().len() >= mesh.indices().len() {
        return Err("strictly reduced level required".into());
    }
    let mut output = std::fs::File::create(&args[2])?;
    writeln!(
        output,
        "world,sample,time,geometric_bound,near_level,far_level,pose_us,certificate_us,selection_us"
    )?;
    let mut durations = Vec::new();
    let mut near_base = 0;
    let mut far_reduced = 0;
    let mut different = 0;
    let mut min_error = f64::INFINITY;
    let mut max_error = 0.0_f64;
    let policy = LodPolicy {
        target_pixels: 1.,
        hysteresis: 0.15,
    };
    for (world_index, world) in [Mat4::IDENTITY, Mat4::from_scale(Vec3::new(1.2, 0.8, 2.))]
        .into_iter()
        .enumerate()
    {
        let mut near_history = None;
        let mut far_history = None;
        for sample in 0..=128 {
            let time = model.animations[0].duration() * sample as f32 / 128.;
            let start = Instant::now();
            let pose = model.sample_pose(Some(0), time)?;
            let palette = model.skin_matrices(&pose)?;
            let pose_us = start.elapsed().as_secs_f64() * 1e6;
            let start = Instant::now();
            let prepared = lod.prepare(&palette, world)?;
            let certificate_us = start.elapsed().as_secs_f64() * 1e6;
            let error = prepared.levels()[1].object_error;
            if !error.is_finite() {
                return Err("nonfinite geometric error".into());
            }
            min_error = min_error.min(error);
            max_error = max_error.max(error);
            let start = Instant::now();
            let near = prepared
                .select_for_camera(camera(2.), [600, 600], policy, near_history)
                .map_err(|e| format!("near camera: {e:?}"))?;
            let far = prepared
                .select_for_camera(camera(200.), [600, 600], policy, far_history)
                .map_err(|e| format!("far camera: {e:?}"))?;
            let selection_us = start.elapsed().as_secs_f64() * 1e6;
            near_history = Some(near);
            far_history = Some(far);
            near_base += usize::from(near == 0);
            far_reduced += usize::from(far == 1);
            different += usize::from(near != far);
            durations.push(certificate_us);
            writeln!(
                output,
                "{world_index},{sample},{time},{error},{near},{far},{pose_us},{certificate_us},{selection_us}"
            )?;
        }
    }
    if near_base == 0 || far_reduced == 0 || different == 0 {
        return Err("camera selection did not distinguish the reduced rig".into());
    }
    durations.sort_by(f64::total_cmp);
    println!(
        "RIG_LOD_PROFILE build={} vertices={} base_indices={} reduced_indices={} samples={} near_base={} far_reduced={} different_views={} geometric_bound_min={} geometric_bound_max={} certificate_median_us={} certificate_p95_us={}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        mesh.vertices().len(),
        mesh.indices().len(),
        lod.indices(1).unwrap().len(),
        durations.len(),
        near_base,
        far_reduced,
        different,
        min_error,
        max_error,
        durations[durations.len() / 2],
        durations[durations.len() * 95 / 100]
    );
    Ok(())
}
