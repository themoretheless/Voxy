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
    /// Camera-plane medium is explicit, including each orthographic ray origin.
    /// Scene occupancy is checked by transport at encountered boundaries.
    pub fn from_pixel(
        camera: crate::SceneCamera,
        viewport: [u32; 2],
        pixel: [u32; 2],
        medium: OpticalMediumId,
    ) -> Result<Self, ComputeError> {
        let (origin, direction) = camera
            .pixel_ray(viewport, pixel)
            .map_err(|_| ComputeError::InvalidBuffer)?;
        Self::new(origin.to_array(), direction.to_array(), medium)
    }

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
/// Caller-owned scene identity and revision namespace, camera revision and size.
/// Every change to source geometry, media, light sources or lens must change its
/// corresponding revision. A new scene must have a distinct identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MediumFrameKey {
    pub scene_id: u128,
    pub scene_revision: u64,
    pub camera_revision: u64,
    pub viewport: [u32; 2],
}
/// Encoded work, not yet submitted. Caller discards its encoder on failure.
#[derive(Debug)]
pub struct EncodedMediumImage {
    input: MediumPathComputeInput,
    key: MediumFrameKey,
    device: wgpu::Device,
    dispatch: crate::ComputeDispatch,
}
/// Nonblocking readback using the existing bounded staging pool.
#[derive(Debug)]
pub struct PendingMediumImage {
    input: MediumPathComputeInput,
    key: MediumFrameKey,
    device: wgpu::Device,
    pending: crate::PendingComputeReadback,
    ready: Option<AcceptedMediumImage>,
}
impl EncodedMediumImage {
    /// Call only after successful submission of its encoder on the device queue.
    pub fn submitted(self) -> PendingMediumImage {
        PendingMediumImage {
            input: self.input,
            key: self.key,
            device: self.device,
            pending: self.dispatch.begin_read(),
            ready: None,
        }
    }
}
impl PendingMediumImage {
    /// Caller polls the original device with PollType::Poll (or the browser loop).
    /// Stale results are drained, then rejected before allocation or queue writes.
    /// Only replace the current image on Ok(Some(image)). No blocking wait occurs.
    /// MemoryBudget retains the admitted snapshot for retry after retirement.
    /// Changed frame keys discard retained snapshots before another upload.
    pub fn try_upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        current: MediumFrameKey,
    ) -> Result<Option<MediumImage>, ComputeError> {
        if device != &self.device {
            return Err(ComputeError::DeviceMismatch);
        }
        if self.ready.is_none() {
            let Some(bytes) = self.pending.try_read()? else {
                return Ok(None);
            };
            if current != self.key {
                return Err(ComputeError::Validation(
                    "stale optical frame scene/camera/viewport".into(),
                ));
            }
            self.ready = Some(self.input.accept_image(&bytes, self.key.viewport)?);
        }
        if current != self.key {
            self.ready.take();
            return Err(ComputeError::Validation(
                "stale optical frame scene/camera/viewport".into(),
            ));
        }
        match self
            .ready
            .as_ref()
            .expect("admitted medium snapshot")
            .upload(device, queue)
        {
            Ok(image) => {
                self.ready.take();
                Ok(Some(image))
            }
            Err(ComputeError::MemoryBudget) => Err(ComputeError::MemoryBudget),
            Err(error) => {
                self.ready.take();
                Err(error)
            }
        }
    }
}
/// Immutable, fully admitted CPU snapshot for HDR presentation.
/// Half-float conversion is checked before any GPU allocation or queue write.
#[derive(Debug)]
pub struct AcceptedMediumImage {
    size: [u32; 2],
    rgba16: Vec<u16>,
}
/// Shared-budget HDR source usable by the existing TextureBlit pass.
#[derive(Debug)]
pub struct MediumImage {
    texture: std::sync::Arc<crate::compute_memory::ManagedTexture>,
    view: wgpu::TextureView,
}
impl MediumImage {
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    pub fn allocation_bytes(&self) -> u64 {
        self.texture.allocation_bytes()
    }
}
impl AcceptedMediumImage {
    /// Upload a new image without mutating an existing presentation source.
    /// The queue must belong to the supplied device. The caller replaces its
    /// previous source only on success and retains submitted sources until done.
    pub fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<MediumImage, ComputeError> {
        let [width, height] = self.size;
        let texture = crate::ComputeMemoryBudget::for_device(device).allocate_texture(
            &wgpu::TextureDescriptor {
                label: Some("accepted medium HDR image"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            },
        )?;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&self.rgba16),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 8),
                rows_per_image: Some(height),
            },
            texture.size(),
        );
        let view = texture.create_view(&Default::default());
        Ok(MediumImage { texture, view })
    }
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
        // Two-dimensional dispatch keeps full-window ray batches below the
        // portable per-dimension limit without another submission owner.
        let columns = (self.rays as u32).min(65_535);
        [columns, (self.rays as u32).div_ceil(columns), 1]
    }
    /// Record computation and bounded snapshot copy in the caller's encoder.
    /// Reuse a program compiled with MEDIUM_PATH_SHADER; unrelated/reloaded
    /// shaders are rejected before job allocation or command encoding.
    pub fn encode_image(
        self,
        device: &wgpu::Device,
        program: &crate::ComputeProgram,
        encoder: &mut wgpu::CommandEncoder,
        key: MediumFrameKey,
    ) -> Result<EncodedMediumImage, ComputeError> {
        if key.viewport.contains(&0)
            || (key.viewport[0] as usize).checked_mul(key.viewport[1] as usize) != Some(self.rays)
            || key.viewport[0].checked_mul(8).is_none()
        {
            return Err(ComputeError::InvalidBuffer);
        }
        if !program.matches_shader(MEDIUM_PATH_SHADER, "cs_main") {
            return Err(ComputeError::Validation(
                "optical frame requires qualified medium path shader".into(),
            ));
        }
        let job = program.create_job(device, self.bytes())?;
        let dispatch = job.encode(encoder, self.workgroups())?;
        Ok(EncodedMediumImage {
            input: self,
            key,
            device: device.clone(),
            dispatch,
        })
    }
    /// Admit row-major pixels as a complete HDR image; no partial image is returned.
    /// Rejects dimension mismatch, failed rays and radiance outside finite f16.
    pub fn accept_image(
        &self,
        bytes: &[u8],
        size: [u32; 2],
    ) -> Result<AcceptedMediumImage, ComputeError> {
        let [width, height] = size;
        if width == 0
            || height == 0
            || width.checked_mul(8).is_none()
            || (width as usize).checked_mul(height as usize) != Some(self.rays)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let estimates = self.decode(bytes)?;
        let mut rgba16 = Vec::with_capacity(self.rays * 4);
        for estimate in estimates {
            for x in estimate.radiance.into_iter().chain([1.]) {
                let h = half::f16::from_f32(x);
                if !h.is_finite() {
                    return Err(ComputeError::InvalidBuffer);
                }
                rgba16.push(h.to_bits());
            }
        }
        Ok(AcceptedMediumImage { size, rgba16 })
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
    #[test]
    fn accepted_image_rejects_partial_dimensions_and_half_float_overflow() {
        let media = [HomogeneousOpticalMedium::new(AIR, 1., [0.; 3], [0.; 3]).unwrap()];
        let scene = CpuMediumTransportScene::new(&media, &[], [-1.; 3], [1.; 3], [1.; 3]).unwrap();
        let rays = [MediumTransportCameraRay::new([0.; 3], [0., 0., -1.], AIR).unwrap(); 2];
        let input = MediumPathComputeInput::new(&scene, &rays, budget()).unwrap();
        let mut words = input.words.clone();
        for i in 0..2 {
            words[input.color_offset + i * 4..input.color_offset + i * 4 + 4].copy_from_slice(&[
                2_f32.to_bits(),
                3_f32.to_bits(),
                4_f32.to_bits(),
                1_f32.to_bits(),
            ]);
        }
        let bytes = bytemuck::cast_slice(&words);
        assert!(input.accept_image(bytes, [1, 1]).is_err());
        assert!(input.accept_image(input.bytes(), [2, 1]).is_err());
        let accepted = input.accept_image(bytes, [2, 1]).unwrap();
        assert_eq!(accepted.rgba16.len(), 8);
        words[input.color_offset + 4] = 70000_f32.to_bits();
        assert!(
            input
                .accept_image(bytemuck::cast_slice(&words), [2, 1])
                .is_err()
        );
        words[input.color_offset + 4] = 2_f32.to_bits();
        words[input.color_offset + 4 * input.rays + 8 + 4] = 4;
        assert!(matches!(
            input.accept_image(bytemuck::cast_slice(&words), [2, 1]),
            Err(ComputeError::MemoryBudget)
        ));
    }
    fn execute_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        program: &crate::ComputeProgram,
        input: &MediumPathComputeInput,
    ) -> Result<Vec<u8>, ComputeError> {
        let job = program.create_job(device, input.bytes())?;
        let mut encoder = device.create_command_encoder(&Default::default());
        let dispatch = job.encode(&mut encoder, input.workgroups())?;
        queue.submit([encoder.finish()]);
        let mut pending = dispatch.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        pending.try_read()?.ok_or(ComputeError::Consumed)
    }
    fn execute(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        program: &crate::ComputeProgram,
        input: &MediumPathComputeInput,
    ) -> Result<Vec<GpuMediumTransportEstimate>, ComputeError> {
        input.decode(&execute_bytes(device, queue, program, input)?)
    }
    #[test]
    #[ignore = "requires physical GPU; full-window dispatch beyond 65535 rays"]
    fn gpu_image_dispatch_crosses_portable_row_limit() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        println!("MEDIUM LARGE DISPATCH GPU {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let _readback_owner = crate::ComputeReadbackPool::configure(
            &device,
            crate::ComputeReadbackLimits {
                max_bytes: 96 << 20,
                max_buffers: 2,
            },
        )
        .unwrap();
        let program =
            pollster::block_on(crate::ComputeProgram::new(&device, MEDIUM_PATH_SHADER)).unwrap();
        let media = [HomogeneousOpticalMedium::new(AIR, 1., [0.2, 0.4, 0.8], [0.; 3]).unwrap()];
        let scene = CpuMediumTransportScene::new(&media, &[], [-1.; 3], [1.; 3], [1.; 3]).unwrap();
        let camera = crate::SceneCamera {
            eye: glam::Vec3::ZERO,
            target: glam::Vec3::NEG_Z,
            up: glam::Vec3::new(0.2, 1., 0.),
            projection: crate::SceneProjection::Perspective {
                vertical_fov: 0.7,
                aspect: 1280. / 720.,
                near: 0.1,
                far: 3.,
            },
        };
        let rays: Vec<_> = (0..720)
            .flat_map(|y| {
                (0..1280).map(move |x| {
                    MediumTransportCameraRay::from_pixel(camera, [1280, 720], [x, y], AIR).unwrap()
                })
            })
            .collect();
        let input = MediumPathComputeInput::new(
            &scene,
            &rays,
            GpuMediumTransportBudget {
                max_bytes: 96 << 20,
                ..budget()
            },
        )
        .unwrap();
        assert_eq!(input.workgroups(), [65_535, 15, 1]);
        let results = execute(&device, &queue, &program, &input).unwrap();
        assert_eq!(results.len(), rays.len());
        for (i, result) in results.iter().enumerate() {
            let (_, direction) = rays[i].geometry.components();
            // The entire frustum reaches z=-1 before either side face.
            assert!(direction[0].abs() < -direction[2] && direction[1].abs() < -direction[2]);
            let distance = 1. / -f64::from(direction[2]);
            for (k, sigma) in [0.2_f64, 0.4, 0.8].into_iter().enumerate() {
                assert!(
                    (f64::from(result.radiance[k]) - (-sigma * distance).exp()).abs() < 2e-6,
                    "pixel={i} channel={k}"
                );
            }
            assert_eq!(result.traced_rays, 1);
        }
        println!(
            "MEDIUM LARGE DISPATCH PASS viewport=1280x720 pixels=921600 groups=65535x15 rgb_analytic_checks=2764800 partial_last_row_guard=true"
        );
    }
    #[test]
    #[ignore = "requires physical GPU; image upload budget recovery"]
    fn gpu_pending_image_retries_after_confirmed_memory_retirement() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let memory = crate::ComputeMemoryBudget::configure(&device, 1 << 20).unwrap();
        let program =
            pollster::block_on(crate::ComputeProgram::new(&device, MEDIUM_PATH_SHADER)).unwrap();
        let media = [HomogeneousOpticalMedium::new(AIR, 1., [0.; 3], [0.; 3]).unwrap()];
        let scene = CpuMediumTransportScene::new(&media, &[], [-1.; 3], [1.; 3], [2.; 3]).unwrap();
        let rays = [MediumTransportCameraRay::new([0.; 3], [0., 0., -1.], AIR).unwrap(); 2];
        let key = MediumFrameKey {
            scene_id: 1,
            scene_revision: 0,
            camera_revision: 0,
            viewport: [2, 1],
        };
        for stale in [false, true] {
            let input = MediumPathComputeInput::new(&scene, &rays, budget()).unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            let encoded = input
                .encode_image(&device, &program, &mut encoder, key)
                .unwrap();
            queue.submit([encoder.finish()]);
            let mut pending = encoded.submitted();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let pressure = memory
                .allocate_storage(
                    "upload memory pressure",
                    &vec![0; (memory.max_bytes() - memory.stats().allocated_bytes) as usize],
                )
                .unwrap();
            assert!(matches!(
                pending.try_upload(&device, &queue, key),
                Err(ComputeError::MemoryBudget)
            ));
            assert!(pending.ready.is_some());
            drop(pressure);
            // Drop alone is insufficient: retirement has not been confirmed.
            assert!(matches!(
                pending.try_upload(&device, &queue, key),
                Err(ComputeError::MemoryBudget)
            ));
            if stale {
                let before = memory.stats();
                assert!(matches!(
                    pending.try_upload(
                        &device,
                        &queue,
                        MediumFrameKey {
                            camera_revision: 1,
                            ..key
                        }
                    ),
                    Err(ComputeError::Validation(_))
                ));
                assert!(pending.ready.is_none());
                assert_eq!(memory.stats(), before);
            }
            memory.discard_retired().unwrap();
            if !stale {
                let image = pending.try_upload(&device, &queue, key).unwrap().unwrap();
                assert_eq!(image.allocation_bytes(), 16);
                assert!(pending.ready.is_none());
                assert!(matches!(
                    pending.try_upload(&device, &queue, key),
                    Err(ComputeError::Consumed)
                ));
                queue.submit([]);
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                drop(image);
                memory.discard_retired().unwrap();
            }
        }
        println!(
            "MEDIUM MEMORY RETRY PASS no_retrace=true unconfirmed_retirement_rejected=true confirmed_retirement_retry=true retained_stale_rejected=true"
        );
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
        let key = MediumFrameKey {
            scene_id: 17,
            scene_revision: 3,
            camera_revision: 9,
            viewport: [2, 1],
        };
        let memory = crate::ComputeMemoryBudget::for_device(&device);
        for current in [
            key,
            MediumFrameKey {
                scene_id: 18,
                ..key
            },
            MediumFrameKey {
                scene_revision: 4,
                ..key
            },
            MediumFrameKey {
                camera_revision: 10,
                ..key
            },
            MediumFrameKey {
                viewport: [1, 2],
                ..key
            },
        ] {
            let input = MediumPathComputeInput::new(&scene, &rays[..2], budget()).unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            let encoded = input
                .encode_image(&device, &program, &mut encoder, key)
                .unwrap();
            queue.submit([encoder.finish()]);
            let mut pending = encoded.submitted();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let before = memory.stats();
            let result = pending.try_upload(&device, &queue, current);
            if current == key {
                let image = result.unwrap().unwrap();
                assert_eq!(image.texture().width(), 2);
                assert_eq!(memory.stats().allocated_bytes, before.allocated_bytes + 16);
                assert_eq!(
                    memory.stats().allocated_textures,
                    before.allocated_textures + 1
                );
            } else {
                assert!(matches!(result, Err(ComputeError::Validation(_))));
                assert_eq!(memory.stats(), before);
            }
            assert!(matches!(
                pending.try_upload(&device, &queue, current),
                Err(ComputeError::Consumed)
            ));
        }
        let unrelated = pollster::block_on(crate::ComputeProgram::new(
            &device,
            crate::MEDIUM_GEOMETRY_SHADER,
        ))
        .unwrap();
        let input = MediumPathComputeInput::new(&scene, &rays[..2], budget()).unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        let before = memory.stats();
        assert!(matches!(
            input.encode_image(&device, &unrelated, &mut encoder, key),
            Err(ComputeError::Validation(_))
        ));
        assert_eq!(memory.stats(), before);
        println!(
            "MEDIUM ASYNC IMAGE PASS valid_upload=1 stale_scene_identity_revision_camera_viewport_rejected=4 unrelated_shader_rejected=true consumed_rejected=true stale_texture_allocations=0"
        );
        let mut pixel_checks = 0;
        for projection in [
            crate::SceneProjection::Perspective {
                vertical_fov: 0.7,
                aspect: 9. / 7.,
                near: 0.1,
                far: 30.,
            },
            crate::SceneProjection::Orthographic {
                left: -0.6,
                right: 0.8,
                bottom: -0.5,
                top: 0.7,
                near: 0.,
                far: 30.,
            },
        ] {
            let camera = crate::SceneCamera {
                eye: glam::Vec3::new(0.13, 0.17, 2.),
                target: glam::Vec3::new(0., 0., -1.),
                up: glam::Vec3::new(0.2, 1., 0.),
                projection,
            };
            let pixels: Vec<_> = (0..7)
                .flat_map(|y| {
                    (0..9).map(move |x| {
                        MediumTransportCameraRay::from_pixel(camera, [9, 7], [x, y], AIR).unwrap()
                    })
                })
                .collect();
            let input = MediumPathComputeInput::new(&scene, &pixels, budget()).unwrap();
            let bytes = execute_bytes(&device, &queue, &program, &input).unwrap();
            let results = input.decode(&bytes).unwrap();
            let accepted = input.accept_image(&bytes, [9, 7]).unwrap();
            let image = accepted.upload(&device, &queue).unwrap();
            assert_eq!(image.allocation_bytes(), 9 * 7 * 8);
            // A failed pixel cannot yield a replacement image. Previous source
            // is still sampled below after the rejection.
            let mut failed = bytes.clone();
            let status = (input.color_offset + 4 * input.rays + 8 * (input.rays - 1) + 4) * 4;
            failed[status..status + 4].copy_from_slice(&3_u32.to_le_bytes());
            assert!(matches!(
                input.accept_image(&failed, [9, 7]),
                Err(ComputeError::WorkBudget)
            ));
            let target = crate::ProcessedColorTarget::new(&device, 9, 7, true).unwrap();
            let blit = crate::TextureBlit::new(&device, wgpu::TextureFormat::Rgba16Float);
            let mut encoder = device.create_command_encoder(&Default::default());
            blit.encode(&device, &mut encoder, image.view(), target.view());
            let mut probes: Vec<_> = [[0, 0], [4, 3], [8, 6]]
                .into_iter()
                .map(|[x, y]| {
                    let mut probe = crate::HdrPixelProbe::new(&device);
                    probe.encode(&mut encoder, target.texture(), x, y).unwrap();
                    (x, y, probe)
                })
                .collect();
            queue.submit([encoder.finish()]);
            for (_, _, probe) in &mut probes {
                probe.begin_read();
            }
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            for (x, y, probe) in probes {
                let pixel = probe.take_result().unwrap().unwrap();
                for k in 0..4 {
                    assert_eq!(
                        pixel[k],
                        half::f16::from_bits(accepted.rgba16[(y * 9 + x) as usize * 4 + k])
                            .to_f32()
                    );
                }
            }
            for (ray, gpu) in pixels.iter().zip(results) {
                let (origin, direction) = ray.geometry.components();
                let cpu = scene
                    .estimate(
                        origin.map(f64::from),
                        glam::DVec3::from_array(direction.map(f64::from))
                            .normalize()
                            .to_array(),
                        AIR,
                        MediumTransportBudget {
                            max_rays: 4096,
                            max_triangle_tests: 1 << 20,
                            absolute_error_rgb: [1e-12; 3],
                        },
                    )
                    .unwrap();
                for k in 0..3 {
                    assert!(
                        (f64::from(gpu.radiance[k]) - cpu.radiance[k]).abs()
                            <= f64::from(gpu.unresolved_upper_bound[k])
                                + cpu.unresolved_upper_bound[k]
                                + 3e-5 * (1. + cpu.radiance[k].abs()),
                        "pixel GPU={gpu:?} CPU={cpu:?}"
                    );
                    pixel_checks += 1;
                }
            }
        }
        println!("MEDIUM CAMERA GRID PASS lenses=2 pixels=126 rgb_comparisons={pixel_checks}");
        println!(
            "MEDIUM HDR IMAGE PASS images=2 displayed_probe_scalars=24 last_pixel_failure_preserves_source=true"
        );
        println!(
            "MEDIUM PATH PASS profiles={profiles} camera_rays=25 scalar_checks={checks} work_failure_rejected=true frontier_failure_rejected=true occupancy_failure_rejected=true"
        );
    }
}
