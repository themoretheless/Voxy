//! All-branch medium radiance using existing compute/storage/readback ownership.
use crate::{ComputeError, CpuMediumTransportScene, MediumGeometryRay, OpticalMediumId};
pub const MEDIUM_PATH_SHADER: &str = concat!(
    include_str!("medium_geometry_common.wgsl"),
    include_str!("medium_transport.wgsl"),
    include_str!("dielectric_boundary.wgsl"),
    include_str!("medium_path_compute.wgsl")
);
#[derive(Clone, Copy, Debug)]
pub struct MediumTransportCameraRay {
    geometry: MediumGeometryRay,
    medium: OpticalMediumId,
}
impl MediumTransportCameraRay {
    pub fn new(
        origin: [f32; 3],
        direction: [f32; 3],
        medium: OpticalMediumId,
    ) -> Result<Self, ComputeError> {
        Ok(Self {
            geometry: MediumGeometryRay::new(origin, direction, 0., 1.)?,
            medium,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct GpuMediumTransportBudget {
    pub max_rays: u32,
    pub max_pending: u32,
    pub absolute_tail_rgb: [f32; 3],
    pub max_bytes: usize,
    pub max_triangle_tests: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct GpuMediumTransportEstimate {
    pub radiance: [f32; 3],
    pub unresolved_upper_bound: [f32; 3],
    pub traced_rays: u32,
    pub charged_triangle_tests: usize,
}
#[derive(Debug)]
pub struct MediumPathComputeInput {
    words: Vec<u32>,
    rays: usize,
    color_offset: usize,
    budget: GpuMediumTransportBudget,
    triangles: usize,
}
fn admit(x: f64) -> Result<u32, ComputeError> {
    let v = x as f32;
    if !v.is_finite() || (x != 0. && v.abs() < f32::MIN_POSITIVE) {
        Err(ComputeError::InvalidBuffer)
    } else {
        Ok(v.to_bits())
    }
}
impl MediumPathComputeInput {
    /// The storage ABI includes candidate RGBA and diagnostics. Private shader
    /// frontier has a fixed 64-branch capacity; its logical use is admitted here.
    /// # Errors
    /// Work/storage budget, invalid ray/medium, or unrepresentable f32 profile.
    pub fn new(
        scene: &CpuMediumTransportScene<'_>,
        rays: &[MediumTransportCameraRay],
        budget: GpuMediumTransportBudget,
    ) -> Result<Self, ComputeError> {
        if rays.is_empty()
            || budget.max_rays == 0
            || budget.max_pending == 0
            || budget.max_pending > 64
            || budget
                .absolute_tail_rgb
                .iter()
                .any(|x| !x.is_finite() || *x < 0.)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let triangles = scene.gpu_triangle_count();
        let work = triangles
            .checked_mul(rays.len())
            .and_then(|n| n.checked_mul(budget.max_rays as usize))
            .ok_or(ComputeError::WorkBudget)?;
        if work > budget.max_triangle_tests {
            return Err(ComputeError::WorkBudget);
        }
        let ids: Vec<_> = scene.gpu_medium_ids().collect();
        let config = triangles
            .checked_mul(24)
            .and_then(|n| n.checked_add(4))
            .ok_or(ComputeError::InvalidBuffer)?;
        let media = config.checked_add(24).ok_or(ComputeError::InvalidBuffer)?;
        let ray_offset = ids
            .len()
            .checked_mul(12)
            .and_then(|n| n.checked_add(media))
            .ok_or(ComputeError::InvalidBuffer)?;
        let color_offset = rays
            .len()
            .checked_mul(8)
            .and_then(|n| n.checked_add(ray_offset))
            .ok_or(ComputeError::InvalidBuffer)?;
        let total = rays
            .len()
            .checked_mul(12)
            .and_then(|n| n.checked_add(color_offset))
            .ok_or(ComputeError::InvalidBuffer)?;
        if total > u32::MAX as usize || total.checked_mul(4).is_none_or(|n| n > budget.max_bytes) {
            return Err(ComputeError::MemoryBudget);
        }
        let (lower, upper, environment, maximum) = scene.gpu_transport_parameters();
        let mut words = Vec::with_capacity(total);
        words.extend([
            triangles as u32,
            rays.len() as u32,
            ray_offset as u32,
            color_offset as u32,
        ]);
        scene.append_gpu_geometry(&mut words)?;
        words.extend([
            ids.len() as u32,
            budget.max_rays,
            budget.max_pending,
            media as u32,
        ]);
        for values in [lower, upper, environment, maximum] {
            for x in values {
                words.push(admit(x)?);
            }
            words.push(0);
        }
        words.extend(budget.absolute_tail_rgb.map(f32::to_bits));
        words.push(0);
        for m in scene.gpu_medium_parameters() {
            for x in m {
                words.push(admit(x)?);
            }
        }
        let lower32 = lower.map(|x| x as f32);
        let upper32 = upper.map(|x| x as f32);
        if (0..3).any(|i| lower32[i] >= upper32[i]) {
            return Err(ComputeError::InvalidBuffer);
        }
        for ray in rays {
            let (origin, direction) = ray.geometry.components();
            if (0..3).any(|i| origin[i] < lower32[i] || origin[i] > upper32[i]) {
                return Err(ComputeError::InvalidBuffer);
            }
            let medium = ids
                .iter()
                .position(|id| *id == ray.medium)
                .ok_or(ComputeError::InvalidBuffer)?;
            for x in origin {
                words.push(admit(f64::from(x))?);
            }
            words.push(medium as u32);
            for x in direction {
                words.push(admit(f64::from(x))?);
            }
            words.push(0);
        }
        if words.len() != color_offset {
            return Err(ComputeError::InvalidBuffer);
        }
        words.resize(total, 0);
        Ok(Self {
            words,
            rays: rays.len(),
            color_offset,
            budget,
            triangles,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        [self.rays as u32, 1, 1]
    }
    /// Reject the entire candidate batch on any failed ray. Tail bounds exclude
    /// f32 roundoff, geometry uncertainty and modelling error, as on CPU.
    pub fn decode(&self, bytes: &[u8]) -> Result<Vec<GpuMediumTransportEstimate>, ComputeError> {
        if bytes.len() != self.words.len() * 4
            || bytes[..self.color_offset * 4] != self.bytes()[..self.color_offset * 4]
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let words: Vec<_> = bytes
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let mut result = Vec::with_capacity(self.rays);
        for i in 0..self.rays {
            let color = &words[self.color_offset + 4 * i..self.color_offset + 4 * (i + 1)];
            let d = &words[self.color_offset + 4 * self.rays + 8 * i
                ..self.color_offset + 4 * self.rays + 8 * (i + 1)];
            match d[4] {
                0 => {}
                3 => return Err(ComputeError::WorkBudget),
                4 => return Err(ComputeError::MemoryBudget),
                _ => {
                    return Err(ComputeError::Validation(
                        "GPU optical transport failed geometry/occupancy/numeric admission".into(),
                    ));
                }
            }
            let radiance = std::array::from_fn(|k| f32::from_bits(color[k]));
            let tail = std::array::from_fn(|k| f32::from_bits(d[k]));
            if color[3] != 1_f32.to_bits()
                || d[3] > self.budget.max_rays
                || d[5] > self.budget.max_pending
                || d[6] != 0
                || d[7] != 0
                || radiance
                    .iter()
                    .chain(&tail)
                    .any(|x| !x.is_finite() || *x < 0.)
                || (0..3).any(|k| tail[k] > self.budget.absolute_tail_rgb[k])
            {
                return Err(ComputeError::InvalidBuffer);
            }
            result.push(GpuMediumTransportEstimate {
                radiance,
                unresolved_upper_bound: tail,
                traced_rays: d[3],
                charged_triangle_tests: self.triangles * d[3] as usize,
            });
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HomogeneousOpticalMedium, MediumBoundaryMesh, MediumTransportBudget, OpaqueRadianceMesh,
    };
    const AIR: OpticalMediumId = OpticalMediumId(u64::MAX);
    const WATER: OpticalMediumId = OpticalMediumId(17);
    fn budget() -> GpuMediumTransportBudget {
        GpuMediumTransportBudget {
            max_rays: 128,
            max_pending: 64,
            absolute_tail_rgb: [1e-5; 3],
            max_bytes: 1 << 20,
            max_triangle_tests: 1 << 20,
        }
    }
    fn boundary() -> MediumBoundaryMesh {
        MediumBoundaryMesh::from_scene_mesh(
            &crate::medium_geometry::tests::box_mesh(false, false),
            WATER,
            AIR,
            1.,
            12,
        )
        .unwrap()
    }
    fn plane(z: f32) -> OpaqueRadianceMesh {
        let vertices = [
            [-11., -11., z],
            [11., -11., z],
            [11., 11., z],
            [-11., 11., z],
        ]
        .into_iter()
        .map(|position| crate::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        })
        .collect();
        let mesh = crate::SceneMesh::new(vertices, vec![0, 1, 2, 0, 2, 3]).unwrap();
        OpaqueRadianceMesh::from_scene_mesh(&mesh, 1., 2, [2., 3., 4.]).unwrap()
    }
    #[test]
    fn transport_abi_limits_and_shader_validation() {
        let media = [HomogeneousOpticalMedium::new(AIR, 1., [0.; 3], [0.; 3]).unwrap()];
        let scene = CpuMediumTransportScene::new(&media, &[], [-1.; 3], [1.; 3], [1.; 3]).unwrap();
        let rays = [MediumTransportCameraRay::new([0.; 3], [0., 0., -1.], AIR).unwrap()];
        assert!(
            MediumPathComputeInput::new(
                &scene,
                &rays,
                GpuMediumTransportBudget {
                    max_pending: 65,
                    ..budget()
                }
            )
            .is_err()
        );
        assert!(
            MediumPathComputeInput::new(
                &scene,
                &rays,
                GpuMediumTransportBudget {
                    max_bytes: 1,
                    ..budget()
                }
            )
            .is_err()
        );
        assert!(MediumPathComputeInput::new(&scene, &[], budget()).is_err());
        let input = MediumPathComputeInput::new(&scene, &rays, budget()).unwrap();
        assert!(input.decode(input.bytes()).is_err());
        let module = naga::front::wgsl::parse_str(MEDIUM_PATH_SHADER).unwrap();
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        assert_eq!(module.global_variables.len(), 1);
        use naga::{back::glsl, proc::BoundsCheckPolicies};
        let mut output = String::new();
        glsl::Writer::new(
            &mut output,
            &module,
            &info,
            &glsl::Options {
                version: glsl::Version::new_gles(310),
                ..Default::default()
            },
            &glsl::PipelineOptions {
                shader_stage: naga::ShaderStage::Compute,
                entry_point: "cs_main".into(),
                multiview: None,
            },
            BoundsCheckPolicies::default(),
        )
        .unwrap()
        .write()
        .unwrap();
        assert!(output.starts_with("#version 310 es"));
    }
    fn execute(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        program: &crate::ComputeProgram,
        input: &MediumPathComputeInput,
    ) -> Result<Vec<GpuMediumTransportEstimate>, ComputeError> {
        let job = program.create_job(device, input.bytes())?;
        let mut encoder = device.create_command_encoder(&Default::default());
        let dispatch = job.encode(&mut encoder, input.workgroups())?;
        queue.submit([encoder.finish()]);
        let mut pending = dispatch.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        input.decode(&pending.try_read()?.ok_or(ComputeError::Consumed)?)
    }
    #[test]
    #[ignore = "requires physical GPU; all-branch medium transport parity"]
    fn gpu_all_branches_match_cpu_with_opacity_and_budget_rejection() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        println!("MEDIUM PATH GPU {:?}", adapter.get_info());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits {
                max_storage_buffers_per_shader_stage: 1,
                ..Default::default()
            },
            ..Default::default()
        }))
        .unwrap();
        let program =
            pollster::block_on(crate::ComputeProgram::new(&device, MEDIUM_PATH_SHADER)).unwrap();
        let boundaries = [boundary()];
        let rays = [
            MediumTransportCameraRay::new([0., 0., 2.], [0., 0., -1.], AIR).unwrap(),
            MediumTransportCameraRay::new(
                [0.125, 0.17, 2.],
                [0.2_f32.sin(), 0., -0.2_f32.cos()],
                AIR,
            )
            .unwrap(),
            MediumTransportCameraRay::new([0., 0., 0.5], [0., 0., -1.], WATER).unwrap(),
        ];
        let mut checks = 0;
        let mut profiles = 0;
        for source in [false, true] {
            let sigma = if source { [0.4, 0.7, 1.1] } else { [0.; 3] };
            let q = if source { [0.04, 0.08, 0.12] } else { [0.; 3] };
            let media = [
                HomogeneousOpticalMedium::new(AIR, 1., [0.1, 0.2, 0.3], [0.; 3]).unwrap(),
                HomogeneousOpticalMedium::new(WATER, 1.5, sigma, q).unwrap(),
            ];
            for opaque_z in [None, Some(1.5), Some(0.), Some(-2.)] {
                let opaque: Vec<_> = opaque_z.into_iter().map(plane).collect();
                let scene = CpuMediumTransportScene::with_opaque(
                    &media,
                    &boundaries,
                    &opaque,
                    [-12., -12., -3.],
                    [12., 12., 3.],
                    [1.; 3],
                )
                .unwrap();
                let input = MediumPathComputeInput::new(&scene, &rays, budget()).unwrap();
                let estimates = execute(&device, &queue, &program, &input).unwrap();
                for (ray, gpu) in rays.iter().zip(estimates) {
                    let (origin, direction) = ray.geometry.components();
                    let cpu = scene
                        .estimate(
                            origin.map(f64::from),
                            glam::DVec3::from_array(direction.map(f64::from))
                                .normalize()
                                .to_array(),
                            ray.medium,
                            MediumTransportBudget {
                                max_rays: 4096,
                                max_triangle_tests: 1 << 20,
                                absolute_error_rgb: [1e-12; 3],
                            },
                        )
                        .unwrap();
                    for i in 0..3 {
                        assert!(
                            (f64::from(gpu.radiance[i]) - cpu.radiance[i]).abs()
                                <= f64::from(gpu.unresolved_upper_bound[i])
                                    + cpu.unresolved_upper_bound[i]
                                    + 3e-5 * (1. + cpu.radiance[i].abs()),
                            "source={source} opaque_z={opaque_z:?} gpu={gpu:?} cpu={cpu:?}"
                        );
                        assert!(gpu.unresolved_upper_bound[i] <= 1e-5);
                        checks += 2;
                    }
                    assert_eq!(
                        gpu.charged_triangle_tests,
                        gpu.traced_rays as usize * scene.gpu_triangle_count()
                    );
                }
                profiles += 1;
            }
        }
        let media = [
            HomogeneousOpticalMedium::new(AIR, 1., [0.; 3], [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(WATER, 1.5, [0.2; 3], [0.45; 3]).unwrap(),
        ];
        let scene = CpuMediumTransportScene::new(
            &media,
            &boundaries,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        let component = 1. / 3_f32.sqrt();
        let trapped =
            [MediumTransportCameraRay::new([0.123, 0.456, 0.03], [component; 3], WATER).unwrap()];
        let input = MediumPathComputeInput::new(&scene, &trapped, budget()).unwrap();
        let estimate = execute(&device, &queue, &program, &input).unwrap()[0];
        for i in 0..3 {
            assert!(
                (estimate.radiance[i] - 2.25).abs() <= estimate.unresolved_upper_bound[i] + 3e-5
            );
            checks += 1;
        }
        let input = MediumPathComputeInput::new(
            &scene,
            &rays[..1],
            GpuMediumTransportBudget {
                max_rays: 1,
                ..budget()
            },
        )
        .unwrap();
        assert!(matches!(
            execute(&device, &queue, &program, &input),
            Err(ComputeError::WorkBudget)
        ));
        let input = MediumPathComputeInput::new(
            &scene,
            &rays[..1],
            GpuMediumTransportBudget {
                max_pending: 1,
                ..budget()
            },
        )
        .unwrap();
        assert!(matches!(
            execute(&device, &queue, &program, &input),
            Err(ComputeError::MemoryBudget)
        ));
        let bad = [MediumTransportCameraRay::new([0.; 3], [0., 0., -1.], AIR).unwrap()];
        let input = MediumPathComputeInput::new(&scene, &bad, budget()).unwrap();
        assert!(matches!(
            execute(&device, &queue, &program, &input),
            Err(ComputeError::Validation(_))
        ));
        println!(
            "MEDIUM PATH PASS profiles={profiles} camera_rays=25 scalar_checks={checks} work_failure_rejected=true frontier_failure_rejected=true occupancy_failure_rejected=true"
        );
    }
}
