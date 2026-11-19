//! GPU resources of the character scene: body and hair geometry, the resident
//! compute paths and the complexion atlas. Recreated after a device loss.
use super::FemaleScene;
use crate::female_complexion::AtlasUpdate;
use crate::female_demo::{FemaleDemo, resident_body::ResidentBody};
use crate::full_model_worker::{Controls, FullModelWorker, partition_hair};
use crate::gpu_hair_solver::GpuHairLinearSolver;
use glam::{Mat4, Vec3};
use voxy_render::{SceneGeometry, SceneMesh, SceneTexture};
use voxy_scene_host::{DemoGpu, DemoResult};

#[derive(Debug)]
pub(super) struct ResidentHair {
    pub(super) input: voxy_render::FiberSurfaceInput,
    pub(super) job: voxy_render::ComputeJob,
    pub(super) transfer: voxy_render::FiberSurfaceTransfer,
    pub(super) indices: Vec<u32>,
    pub(super) dirty: bool,
}

/// Optional asynchronous tail audit; absent from the normal rendering path.
#[derive(Debug)]
pub(super) struct SurfaceLightAudit {
    pub(super) buffer: wgpu::Buffer,
    pub(super) pending: Option<std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
}

impl SurfaceLightAudit {
    pub(super) fn poll(&mut self, device: &wgpu::Device) -> DemoResult {
        device.poll(wgpu::PollType::Poll)?;
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    result?;
                    let data = self.buffer.slice(..).get_mapped_range()?;
                    let h: [u32; 8] = std::array::from_fn(|i| {
                        u32::from_le_bytes(
                            data[i * 4..i * 4 + 4]
                                .try_into()
                                .expect("four header bytes"),
                        )
                    });
                    eprintln!(
                        "GPU SURFACE LIGHT AUDIT geometry={} admitted={} has_accepted={} generation={} error_estimate={:.8} relative_squared={:.8e}",
                        h[0],
                        h[1],
                        h[2],
                        h[3],
                        f32::from_bits(h[4]),
                        f32::from_bits(h[5])
                    );
                    drop(data);
                    self.buffer.unmap();
                    self.pending = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
    pub(super) fn begin(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        self.pending = Some(rx);
    }
}

#[derive(Debug)]
pub(super) struct FemaleGpu {
    pub(super) cube: SceneGeometry,
    pub(super) hair_geometry: Option<SceneGeometry>,
    pub(super) gpu_hair: Option<ResidentHair>,
    pub(super) gpu_secondary: Option<voxy_render::SurfaceDeformation>,
    pub(super) gpu_body: Option<ResidentBody>,
    pub(super) gpu_surface_light: Option<voxy_render::SurfaceLightFrame>,
    pub(super) gpu_surface_colors: Option<voxy_render::SurfaceColorTransfer>,
    pub(super) gpu_surface_light_audit: Option<SurfaceLightAudit>,
    /// The tail audit copy was encoded into the frame being rendered.
    pub(super) surface_light_audit_encoded: bool,
    pub(super) gpu_secondary_reference: Vec<[f32; 4]>,
    pub(super) gpu_secondary_last_controls: Vec<[f32; 4]>,
    pub(super) gpu_jump_last_palette: Vec<Mat4>,
    /// Controls and palette of the frame being rendered; kept once it is presented.
    pub(super) gpu_controls: Option<Vec<[f32; 4]>>,
    pub(super) gpu_jump_palette: Option<Vec<Mat4>>,
    pub(super) film_layer: Option<SceneGeometry>,
    pub(super) optical_frames: u64,
    pub(super) film_mass_checks: u64,
    pub(super) film_mass_max_relative_error: f64,
    pub(super) overlay: SceneGeometry,
    pub(super) legend_labels: Vec<[&'static str; 3]>,
    pub(super) material_creases: Vec<f32>,
    pub(super) atlas_update: AtlasUpdate,
    pub(super) last_view_position: Option<Vec3>,
}

/// Which resident GPU paths this run uses, from the environment and the model.
#[derive(Clone, Copy, Debug)]
struct Plan {
    secondary: bool,
    async_full: bool,
    body: bool,
    surface_light: bool,
    hair: bool,
}

impl Plan {
    fn new(female: &FemaleDemo, secondary: bool) -> DemoResult<Self> {
        let async_full = std::env::var_os("VOXY_ASYNC_FULL").is_some();
        let body = std::env::var_os("VOXY_GPU_BODY").is_some();
        let surface_light = std::env::var_os("VOXY_GPU_SURFACE_LIGHT").is_some();
        if body && (async_full || secondary || surface_light) {
            return Err(
                "GPU body currently requires synchronous physical stance with CPU surface lighting"
                    .into(),
            );
        }
        if surface_light && (async_full || secondary) {
            return Err("authored light-normal frame ownership for async/GPU-secondary combinations is not qualified yet".into());
        }
        if surface_light
            && std::env::var("VOXY_FACE_EXPERIMENTAL_TRANSMISSION").as_deref() == Ok("1")
        {
            return Err(
                "GPU surface lighting additive experimental transmission is not qualified yet"
                    .into(),
            );
        }
        let hair = (body || async_full || std::env::var_os("VOXY_RESIDENT_HAIR").is_some())
            && std::env::var_os("VOXY_DISABLE_GPU_HAIR").is_none()
            && female.face_parameters == crate::face_parameters::FaceParameters::default()
            && female.film.is_none()
            && female.simulate_hair;
        Ok(Self {
            secondary,
            async_full,
            body,
            surface_light,
            hair,
        })
    }
}

#[derive(Debug)]
struct BodyGeometry {
    cube: SceneGeometry,
    hair_geometry: Option<SceneGeometry>,
    gpu_hair: Option<ResidentHair>,
}

fn resident_hair_geometry(
    female: &FemaleDemo,
    plan: Plan,
    gpu: &DemoGpu<'_>,
    mesh: &SceneMesh,
) -> DemoResult<BodyGeometry> {
    let (_, indices) = partition_hair(mesh).map_err(std::io::Error::other)?;
    let mut hair = gpu.scene.upload_compute_tangent_mesh(
        gpu.device,
        mesh,
        &female.gpu_hair_vertex_tangents(mesh)?,
    )?;
    hair.update_index_partition(gpu.queue, if female.show_hair { &indices } else { &[] })?;
    pollster::block_on(gpu.scene.set_geometry_opaque_shader(
        gpu.device,
        &mut hair,
        include_str!("../female_hair_material_tangent.wgsl"),
    ))?;
    let input = female.gpu_hair_surface_input()?;
    let base = mesh
        .vertices()
        .iter()
        .position(|v| v.uv[0] == -7.)
        .ok_or("missing full hair surface")?;
    let program = pollster::block_on(voxy_render::ComputeProgram::new(
        gpu.device,
        voxy_render::FIBER_SURFACE_SHADER,
    ))?;
    let job = program.create_job(gpu.device, input.bytes())?;
    let transfer = pollster::block_on(voxy_render::FiberSurfaceTransfer::new_with_tangents(
        gpu.device,
        &hair,
        job.buffer(),
        &input,
        base as u32,
    ))?;
    eprintln!(
        "FULL MODEL GPU HAIR initialized vertices={} resident=true",
        input.vertex_count()
    );
    let cube = if plan.surface_light || plan.body {
        gpu.scene
            .upload_compute_mesh(gpu.device, &female.mesh_without_hair()?)?
    } else {
        gpu.scene
            .upload_mesh(gpu.device, &female.mesh_without_hair()?)?
    };
    Ok(BodyGeometry {
        cube,
        hair_geometry: Some(hair),
        gpu_hair: Some(ResidentHair {
            input,
            job,
            transfer,
            indices,
            dirty: true,
        }),
    })
}

fn async_partitions(plan: Plan, gpu: &DemoGpu<'_>, mesh: &SceneMesh) -> DemoResult<BodyGeometry> {
    let (body, hair) = partition_hair(mesh).map_err(std::io::Error::other)?;
    let mut partitions = if plan.surface_light {
        gpu.scene
            .upload_compute_shared_mesh_partitions(gpu.device, mesh)?
    } else {
        gpu.scene.upload_shared_mesh_partitions(gpu.device, mesh)?
    };
    let mut hair_owner = partitions.pop().unwrap();
    let mut body_owner = partitions.pop().unwrap();
    body_owner.update_index_partition(gpu.queue, &body)?;
    hair_owner.update_index_partition(gpu.queue, &hair)?;
    pollster::block_on(gpu.scene.set_geometry_opaque_shader(
        gpu.device,
        &mut hair_owner,
        crate::FEMALE_HAIR_MATERIAL_SHADER,
    ))?;
    Ok(BodyGeometry {
        cube: body_owner,
        hair_geometry: Some(hair_owner),
        gpu_hair: None,
    })
}

fn body_geometry(
    female: &FemaleDemo,
    plan: Plan,
    gpu: &DemoGpu<'_>,
    mesh: &SceneMesh,
) -> DemoResult<BodyGeometry> {
    if plan.hair {
        return resident_hair_geometry(female, plan, gpu, mesh);
    }
    if plan.async_full {
        return async_partitions(plan, gpu, mesh);
    }
    let cube = if plan.secondary {
        gpu.scene
            .upload_mesh(gpu.device, &SceneMesh::quad([1.; 4]))?
    } else if plan.surface_light || plan.body {
        gpu.scene.upload_compute_mesh(gpu.device, mesh)?
    } else {
        gpu.scene.upload_mesh(gpu.device, mesh)?
    };
    Ok(BodyGeometry {
        cube,
        hair_geometry: None,
        gpu_hair: None,
    })
}

fn resident_body(
    female: &FemaleDemo,
    plan: Plan,
    gpu: &DemoGpu<'_>,
    mesh: &SceneMesh,
    cube: &SceneGeometry,
) -> DemoResult<Option<ResidentBody>> {
    if !plan.body {
        return Ok(None);
    }
    let separated = if plan.hair {
        Some(female.mesh_without_hair()?)
    } else {
        None
    };
    let body = ResidentBody::new(
        gpu.device,
        gpu.queue,
        gpu.scene,
        female,
        separated.as_ref().unwrap_or(mesh),
        cube,
    )?;
    eprintln!(
        "FULL MODEL GPU BODY initialized: resident rig + shell; CPU face details and physics retained"
    );
    Ok(Some(body))
}

fn secondary_deformation(
    female: &FemaleDemo,
    plan: Plan,
    gpu: &DemoGpu<'_>,
    mesh: &SceneMesh,
) -> DemoResult<(Option<voxy_render::SurfaceDeformation>, Vec<[f32; 4]>)> {
    if !plan.secondary {
        return Ok((None, Vec::new()));
    }
    let (offsets, weights, count) = female.gpu_secondary_binding(mesh.vertices().len())?;
    eprintln!(
        "GPU SECONDARY preview: {} vertices; neutral initial facial pose and baked initial irradiance; same nonlinear tissue simulation",
        mesh.vertices().len()
    );
    let mut deformation = gpu
        .scene
        .prepare_surface_deformation(gpu.device, mesh, &offsets, &weights, count)?;
    deformation.set_deforming_normal_prefix(female.gpu_secondary_body_vertices())?;
    let palette = female.gpu_jump_palette();
    deformation.enable_rigid_skinning(
        gpu.device,
        mesh.authored_normals()
            .ok_or("jump GPU source lacks normals")?,
        &female.gpu_secondary_jump_weights(mesh.vertices().len()),
        palette.len() as u32,
        Some(3),
    )?;
    deformation.upload_rigid_pose(gpu.queue, &palette)?;
    let reference = female.gpu_secondary_controls();
    deformation.update_controls(gpu.queue, &vec![[0.; 4]; reference.len()])?;
    Ok((Some(deformation), reference))
}

fn surface_light(
    female: &FemaleDemo,
    plan: Plan,
    gpu: &DemoGpu<'_>,
    mesh: &SceneMesh,
    target: &SceneGeometry,
) -> DemoResult<Option<voxy_render::SurfaceLightFrame>> {
    if !plan.surface_light {
        return Ok(None);
    }
    let count = female.gpu_secondary_body_vertices() as usize;
    let triangles: Vec<_> = mesh
        .indices()
        .chunks_exact(3)
        .filter(|t| t.iter().all(|i| (*i as usize) < count))
        .map(|t| [t[0], t[1], t[2]])
        .collect();
    let settings = voxy_render::SurfaceLightFrameSettings {
        key_direction: crate::female_transmission::key_direction().to_array(),
        key_weight: 0.6,
        fill_direction: [0.8, 0.2, 0.5],
        fill_weight: 0.2,
        ambient: 0.2,
        albedo: [0.72, 0.46, 0.34, 1.],
        surface_fraction: female
            .face_parameters
            .value("skin_surface_light")
            .unwrap_or(0.35),
        radii: [0.002, 0.001, 0.0005],
    };
    let light = pollster::block_on(voxy_render::SurfaceLightFrame::new_with_reference_normals(
        gpu.device,
        target,
        &triangles,
        settings,
        &female.surface_light_normals(),
    ))?;
    eprintln!(
        "GPU SURFACE LIGHT initialized vertices={count} triangles={} CPU_authored_normals=true CPU_reference_retained={}",
        triangles.len(),
        std::env::var_os("VOXY_GPU_SURFACE_LIGHT_CPU_REFERENCE").is_some()
    );
    Ok(Some(light))
}

fn surface_colors(
    female: &FemaleDemo,
    plan: Plan,
    gpu: &DemoGpu<'_>,
    cube: &SceneGeometry,
    inherited_colors: &[voxy_render::SurfaceColorBinding],
) -> DemoResult<Option<voxy_render::SurfaceColorTransfer>> {
    if !plan.surface_light || inherited_colors.is_empty() {
        return Ok(None);
    }
    let transfer = pollster::block_on(voxy_render::SurfaceColorTransfer::new(
        gpu.device,
        cube,
        female.gpu_secondary_body_vertices(),
        inherited_colors,
    ))?;
    eprintln!(
        "GPU SURFACE COLORS inherited_bindings={}",
        inherited_colors.len()
    );
    Ok(Some(transfer))
}

fn surface_light_audit(device: &wgpu::Device, enabled: bool) -> Option<SurfaceLightAudit> {
    (enabled && std::env::var_os("VOXY_GPU_SURFACE_LIGHT_AUDIT").is_some()).then(|| {
        SurfaceLightAudit {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("surface light tail audit"),
                size: 32,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            pending: None,
        }
    })
}

fn complexion_atlas(female: &FemaleDemo, gpu: &DemoGpu<'_>) -> DemoResult<SceneTexture> {
    Ok(gpu.scene.upload_texture_with_sampling(
        gpu.device,
        gpu.queue,
        crate::female_complexion::WIDTH,
        crate::female_complexion::SIZE,
        &crate::female_complexion::atlas_for_parameters(&female.face_parameters),
        voxy_render::TextureSampling {
            min_filter: voxy_render::TextureFilter::Linear,
            mag_filter: voxy_render::TextureFilter::Linear,
            ..Default::default()
        },
    )?)
}

impl FemaleScene {
    /// Creates every GPU resource of the scene and returns the complexion atlas.
    pub(super) fn create_gpu(&mut self, gpu: DemoGpu<'_>) -> DemoResult<SceneTexture> {
        if self.use_gpu_secondary {
            eprintln!(
                "GPU SECONDARY resolution={}x{} MSAA4={}",
                gpu.size[0],
                gpu.size[1],
                std::env::var_os("VOXY_GPU_SECONDARY_MSAA").is_some()
            );
        }
        // The render host owns the device; physics receives its existing backend.
        // Keep qualification explicit until coupled dynamic GPU admission passes.
        if std::env::var_os("VOXY_GPU_HAIR_PHYSICS").is_some() {
            self.hair_solver = Some(pollster::block_on(GpuHairLinearSolver::new_coupled(
                gpu.device, gpu.queue,
            ))?);
        }
        pollster::block_on(
            gpu.scene
                .reload_shader(gpu.device, crate::female_eyes::MATERIAL_SHADER),
        )?;
        let female = &self.female;
        let plan = Plan::new(female, self.use_gpu_secondary)?;
        let mut inherited_colors = Vec::new();
        let mesh = if plan.surface_light {
            let (mesh, bindings) =
                female.mesh_with_gpu_light_bindings(female.show_hair || plan.hair)?;
            inherited_colors = bindings;
            mesh
        } else if plan.hair {
            female.mesh_with_full_hair()?
        } else {
            female.mesh()?
        };
        let BodyGeometry {
            cube,
            hair_geometry,
            gpu_hair,
        } = body_geometry(female, plan, &gpu, &mesh)?;
        let gpu_body = resident_body(female, plan, &gpu, &mesh, &cube)?;
        let (gpu_secondary, gpu_secondary_reference) =
            secondary_deformation(female, plan, &gpu, &mesh)?;
        let light_target = gpu_secondary
            .as_ref()
            .map_or(&cube, |deformation| deformation.geometry());
        let gpu_surface_light = surface_light(female, plan, &gpu, &mesh, light_target)?;
        let gpu_surface_colors = surface_colors(female, plan, &gpu, &cube, &inherited_colors)?;
        let gpu_surface_light_audit = surface_light_audit(gpu.device, gpu_surface_light.is_some());
        let legend_labels = female.diagnostic_legend_labels();
        let overlay = gpu
            .scene
            .upload_mesh(gpu.device, &crate::diagnostic_legend::mesh(&legend_labels)?)?;
        let texture = complexion_atlas(female, &gpu)?;
        self.gpu = Some(FemaleGpu {
            cube,
            hair_geometry,
            gpu_hair,
            gpu_secondary,
            gpu_body,
            gpu_surface_light,
            gpu_surface_colors,
            gpu_surface_light_audit,
            surface_light_audit_encoded: false,
            gpu_secondary_last_controls: vec![[0.; 4]; gpu_secondary_reference.len()],
            gpu_jump_last_palette: Vec::new(),
            gpu_secondary_reference,
            gpu_controls: None,
            gpu_jump_palette: None,
            film_layer: None,
            optical_frames: 0,
            film_mass_checks: 0,
            film_mass_max_relative_error: 0.,
            overlay,
            legend_labels,
            material_creases: female.face_parameters.material_signature(),
            atlas_update: AtlasUpdate::default(),
            last_view_position: None,
        });
        self.start_full_model_worker(gpu.paused)?;
        Ok(texture)
    }

    /// Moves the simulation to the background worker and keeps a view replica here.
    fn start_full_model_worker(&mut self, paused: bool) -> DemoResult {
        if self.full_model_worker.is_some() || std::env::var_os("VOXY_ASYNC_FULL").is_none() {
            return Ok(());
        }
        let view = self.female.background_view_replica()?;
        let controls = Controls::capture(&view, paused);
        let simulation = std::mem::replace(&mut self.female, view);
        self.full_model_worker = Some(FullModelWorker::new(
            simulation,
            controls,
            self.gpu.as_ref().is_some_and(|gpu| gpu.gpu_hair.is_some()),
            self.hair_solver.take(),
        )?);
        Ok(())
    }
}
