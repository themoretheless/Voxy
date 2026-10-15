use super::*;
use crate::GgxEnergyOracleSample;
include!("../dfg_build_fixture.rs");
fn inputs(d: &wgpu::Device) -> (crate::GgxEnvironmentPrefilter, crate::GgxDfgLut) {
    (
        crate::GgxEnvironmentPrefilter::new(d, 2).unwrap(),
        crate::GgxDfgLut::new(d, 4).unwrap(),
    )
}
#[test]
fn dropped_preparation_does_not_attach_lighting() {
    let (d, _) = wgpu::Device::noop(&Default::default());
    let (env, dfg) = inputs(&d);
    let scene = SceneRenderer::new(&d, wgpu::TextureFormat::Rgba8Unorm);
    let pipeline = scene.world_pipeline.clone();
    let revision = scene.shader_revision();
    let prepared =
        pollster::block_on(scene.prepare_environment_lighting(&d, &env, &dfg, None)).unwrap();
    assert!(scene.environment.is_none());
    assert_eq!(scene.world_pipeline, pipeline);
    assert_eq!(scene.shader_revision(), revision);
    assert_eq!(scene.lighting_revision(), 0);
    drop(prepared);
    assert!(scene.environment.is_none());
    assert_eq!(scene.world_pipeline, pipeline);
}
#[test]
fn invalid_source_preserves_previous_environment() {
    let (d, _) = wgpu::Device::noop(&Default::default());
    let (env, dfg) = inputs(&d);
    let mut scene = SceneRenderer::new(&d, wgpu::TextureFormat::Rgba8Unorm);
    pollster::block_on(scene.enable_environment_lighting(&d, &env, &dfg)).unwrap();
    let group = scene.environment.as_ref().unwrap().group.clone();
    let pipeline = scene.world_pipeline.clone();
    let revision = scene.lighting_revision();
    assert!(
        pollster::block_on(scene.prepare_environment_source(&d, &env, &dfg, None, "invalid WGSL"))
            .is_err()
    );
    assert_eq!(scene.environment.as_ref().unwrap().group, group);
    assert_eq!(scene.world_pipeline, pipeline);
    assert_eq!(scene.lighting_revision(), revision);
}
#[test]
fn same_shader_resource_change_invalidates_old_candidate() {
    let (d, _) = wgpu::Device::noop(&Default::default());
    let (env, dfg) = inputs(&d);
    let env2 = crate::GgxEnvironmentPrefilter::new(&d, 2).unwrap();
    let mut scene = SceneRenderer::new(&d, wgpu::TextureFormat::Rgba8Unorm);
    pollster::block_on(scene.enable_environment_lighting(&d, &env, &dfg)).unwrap();
    let prepared =
        pollster::block_on(scene.prepare_environment_lighting(&d, &env, &dfg, None)).unwrap();
    let shader = scene.shader_revision();
    pollster::block_on(scene.enable_environment_lighting(&d, &env2, &dfg)).unwrap();
    assert_eq!(scene.shader_revision(), shader);
    let group = scene.environment.as_ref().unwrap().group.clone();
    let lighting = scene.lighting_revision();
    assert!(scene.commit_prepared_environment(prepared).is_err());
    assert_eq!(scene.environment.as_ref().unwrap().group, group);
    assert_eq!(scene.lighting_revision(), lighting);
}
#[test]
fn dynamic_intensity_remains_shared_across_preparation() {
    let (d, q) = wgpu::Device::noop(&Default::default());
    let (env, dfg) = inputs(&d);
    let mut scene = SceneRenderer::new(&d, wgpu::TextureFormat::Rgba8Unorm);
    pollster::block_on(scene.enable_environment_lighting(&d, &env, &dfg)).unwrap();
    let buffer = scene.environment.as_ref().unwrap().intensity_buffer.clone();
    let prepared =
        pollster::block_on(scene.prepare_environment_lighting(&d, &env, &dfg, None)).unwrap();
    scene.set_environment_intensity(&q, 3.).unwrap();
    scene.commit_prepared_environment(prepared).unwrap();
    assert_eq!(scene.environment.as_ref().unwrap().intensity, 3.);
    assert_eq!(scene.environment.as_ref().unwrap().intensity_buffer, buffer);
}
#[test]
fn foreign_scene_and_changed_msaa_reject_before_mutation() {
    let (d, _) = wgpu::Device::noop(&Default::default());
    let (env, dfg) = inputs(&d);
    let mut a = SceneRenderer::new(&d, wgpu::TextureFormat::Rgba8Unorm);
    let mut b = SceneRenderer::new(&d, wgpu::TextureFormat::Rgba8Unorm);
    let prepared =
        pollster::block_on(a.prepare_environment_lighting(&d, &env, &dfg, None)).unwrap();
    assert!(b.commit_prepared_environment(prepared).is_err());
    assert!(b.environment.is_none());
    pollster::block_on(a.enable_environment_lighting(&d, &env, &dfg)).unwrap();
    let prepared =
        pollster::block_on(a.prepare_environment_lighting(&d, &env, &dfg, None)).unwrap();
    let group = a.environment.as_ref().unwrap().group.clone();
    pollster::block_on(a.enable_msaa4(&d)).unwrap();
    assert!(a.commit_prepared_environment(prepared).is_err());
    assert!(a.msaa_pipelines.is_some());
    assert_eq!(a.environment.as_ref().unwrap().group, group);
}
#[test]
#[ignore = "requires physical GPU; joint scene/reflection atlas transaction"]
fn scene_and_reflection_commit_together_and_stale_capture_rolls_back_all() {
    use crate::{
        GgxEnergyAtlasBuild, GgxEnergyAtlasSlot, GgxEnergyBuildBudget, GgxEnergyGpuPrecompute,
        GgxEnergyRefinementPlan, GgxEnergyRefinementPolicy,
    };
    let instance = crate::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (d, q) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let gpu = pollster::block_on(GgxEnergyGpuPrecompute::new(&d)).unwrap();
    let (env, legacy) = inputs(&d);
    let diffuse = crate::DiffuseEnvironmentConvolution::new(&d, 2).unwrap();
    let mut encoder = d.create_command_encoder(&Default::default());
    env.encode(&mut encoder);
    diffuse.encode(&mut encoder);
    legacy.encode(&mut encoder);
    q.submit([encoder.finish()]);
    let mut slot = GgxEnergyAtlasSlot::new(legacy, 0);
    let mut scene = SceneRenderer::new(&d, wgpu::TextureFormat::Rgba8Unorm);
    let mut capture = crate::PlanarReflectionCapture::new(&d, 4, 4).unwrap();
    pollster::block_on(scene.enable_full_environment_lighting(&d, &env, slot.current(), &diffuse))
        .unwrap();
    pollster::block_on(capture.enable_full_environment_lighting(&env, slot.current(), &diffuse))
        .unwrap();
    let shadow = crate::ShadowMap::new(&d, 4, 4).unwrap();
    pollster::block_on(scene.enable_shadowed_point_light(
        &d,
        &shadow,
        crate::ShadowSettings {
            light_from_world: glam::Mat4::IDENTITY,
            bias: 0.001,
            enabled: false,
            filter: crate::ShadowFilter::Hard,
        },
    ))
    .unwrap();
    let build = |generation| {
        GgxEnergyAtlasBuild::new(
            &d,
            &q,
            generation,
            GgxEnergyRefinementPlan::regions_from_seeds(&[[1., 0.005]]).unwrap(),
            ORACLE.to_vec(),
            GgxEnergyRefinementPolicy::try_new(1e-5, 8).unwrap(),
            GgxEnergyBuildBudget {
                max_nodes: 4096,
                max_probes: 512,
                max_candidate_atlas_bytes: 2 * 1024 * 1024,
            },
        )
        .unwrap()
    };
    let finish = |mut build: GgxEnergyAtlasBuild, generation| {
        let started = std::time::Instant::now();
        loop {
            if let Some(ready) = build.poll(&gpu, generation).unwrap() {
                break ready;
            }
            assert!(started.elapsed().as_secs() < 30);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    };
    slot.request_generation(1).unwrap();
    let ready = finish(build(1), 1);
    let expected = ready.atlas().output().clone();
    let main_old = scene.environment.as_ref().unwrap().group.clone();
    let capture_old = capture
        .renderer()
        .environment
        .as_ref()
        .unwrap()
        .group
        .clone();
    let main = pollster::block_on(scene.prepare_environment_lighting(
        &d,
        &env,
        ready.atlas(),
        Some(&diffuse),
    ))
    .unwrap();
    let reflection = pollster::block_on(capture.prepare_environment_lighting(
        &env,
        ready.atlas(),
        Some(&diffuse),
    ))
    .unwrap();
    assert_eq!(scene.environment.as_ref().unwrap().group, main_old);
    assert_eq!(
        capture.renderer().environment.as_ref().unwrap().group,
        capture_old
    );
    scene.set_environment_intensity(&q, 2.5).unwrap();
    capture.set_environment_intensity(&q, 3.5).unwrap();
    capture
        .commit_environment_atlas_with_scene(&mut scene, main, reflection, &mut slot, ready)
        .unwrap();
    assert_eq!(slot.current().output(), &expected);
    assert_eq!(slot.installed_generation(), 1);
    assert_eq!(
        scene.environment.as_ref().unwrap().sampling,
        crate::dfg::DfgSampling::EnergyPatches
    );
    assert_eq!(
        capture.renderer().environment.as_ref().unwrap().sampling,
        crate::dfg::DfgSampling::EnergyPatches
    );
    assert_eq!(scene.environment.as_ref().unwrap().intensity, 2.5);
    assert_eq!(
        capture.renderer().environment.as_ref().unwrap().intensity,
        3.5
    );
    slot.request_generation(2).unwrap();
    let ready = finish(build(2), 2);
    let main = pollster::block_on(scene.prepare_environment_lighting(
        &d,
        &env,
        ready.atlas(),
        Some(&diffuse),
    ))
    .unwrap();
    let reflection = pollster::block_on(capture.prepare_environment_lighting(
        &env,
        ready.atlas(),
        Some(&diffuse),
    ))
    .unwrap();
    pollster::block_on(capture.enable_full_environment_lighting(&env, slot.current(), &diffuse))
        .unwrap();
    let main_current = scene.environment.as_ref().unwrap().group.clone();
    let capture_current = capture
        .renderer()
        .environment
        .as_ref()
        .unwrap()
        .group
        .clone();
    let main_rev = scene.lighting_revision();
    let capture_rev = capture.renderer().lighting_revision();
    assert!(
        capture
            .commit_environment_atlas_with_scene(&mut scene, main, reflection, &mut slot, ready)
            .is_err()
    );
    assert_eq!(slot.current().output(), &expected);
    assert_eq!(slot.installed_generation(), 1);
    assert_eq!(scene.environment.as_ref().unwrap().group, main_current);
    assert_eq!(
        capture.renderer().environment.as_ref().unwrap().group,
        capture_current
    );
    assert_eq!(scene.lighting_revision(), main_rev);
    assert_eq!(capture.renderer().lighting_revision(), capture_rev);
    // Wrong DFG identity also rejects the entire transaction before slot install.
    let ready = finish(build(2), 2);
    let wrong = pollster::block_on(scene.prepare_environment_lighting(
        &d,
        &env,
        slot.current(),
        Some(&diffuse),
    ))
    .unwrap();
    let reflection = pollster::block_on(capture.prepare_environment_lighting(
        &env,
        ready.atlas(),
        Some(&diffuse),
    ))
    .unwrap();
    assert!(
        capture
            .commit_environment_atlas_with_scene(&mut scene, wrong, reflection, &mut slot, ready)
            .is_err()
    );
    assert_eq!(slot.current().output(), &expected);
    assert_eq!(scene.lighting_revision(), main_rev);
    assert_eq!(capture.renderer().lighting_revision(), capture_rev);
}
