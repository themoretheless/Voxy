//! Metric medium/opaque geometry through the existing ComputeProgram owner.
use crate::{ComputeError, CpuMediumTransportScene, OpticalMediumId};

pub const MEDIUM_GEOMETRY_SHADER: &str = concat!(
    include_str!("medium_geometry_common.wgsl"),
    include_str!("medium_geometry_compute.wgsl")
);
#[derive(Clone, Copy, Debug)]
pub struct MediumGeometryRay {
    origin: [f32; 3],
    direction: [f32; 3],
    minimum: f32,
    maximum: f32,
}
impl MediumGeometryRay {
    pub(crate) fn components(self) -> ([f32; 3], [f32; 3]) {
        (self.origin, self.direction)
    }
    /// # Errors
    /// Nonfinite/nonunit direction or invalid explicit metric interval (min,max].
    pub fn new(
        origin: [f32; 3],
        direction: [f32; 3],
        minimum: f32,
        maximum: f32,
    ) -> Result<Self, ComputeError> {
        let d = glam::Vec3::from_array(direction);
        let length = d.length();
        if origin.iter().chain(&direction).any(|x| !x.is_finite())
            || !length.is_finite()
            || (length - 1.).abs() > 1e-5
            || !minimum.is_finite()
            || !maximum.is_finite()
            || minimum < 0.
            || maximum <= minimum
        {
            return Err(ComputeError::InvalidBuffer);
        }
        Ok(Self {
            origin,
            direction: d.normalize().to_array(),
            minimum,
            maximum,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MediumGeometryHitKind {
    Boundary {
        incident: OpticalMediumId,
        transmitted: OpticalMediumId,
    },
    Opaque {
        radiance: [f32; 3],
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediumGeometryGpuHit {
    pub distance_m: f32,
    pub position_m: [f32; 3],
    pub incident_normal: [f32; 3],
    /// Global triangle index and snapshot-local object index within its kind.
    pub primitive_index: u32,
    pub object_index: u32,
    pub kind: MediumGeometryHitKind,
}
#[derive(Debug)]
pub struct MediumGeometryComputeInput {
    words: Vec<u32>,
    medium_ids: Vec<OpticalMediumId>,
    rays: usize,
    hit_offset: usize,
}
impl MediumGeometryComputeInput {
    /// Pack a borrowed immutable scene. Budget covers the storage ABI including
    /// outputs; ComputeProgram separately admits device storage/readback budgets.
    /// Medium IDs remain u64 on CPU and map to dense snapshot-local GPU indices.
    /// # Errors
    /// Invalid/empty rays, ABI/memory/work capacity or geometry unrepresentable in f32.
    pub fn new(
        scene: &CpuMediumTransportScene<'_>,
        rays: &[MediumGeometryRay],
        max_bytes: usize,
        max_triangle_tests: usize,
    ) -> Result<Self, ComputeError> {
        let triangles = scene.gpu_triangle_count();
        let work = triangles
            .checked_mul(rays.len())
            .ok_or(ComputeError::WorkBudget)?;
        if work > max_triangle_tests {
            return Err(ComputeError::WorkBudget);
        }
        let ray_offset = triangles
            .checked_mul(24)
            .and_then(|n| n.checked_add(4))
            .ok_or(ComputeError::InvalidBuffer)?;
        let hit_offset = rays
            .len()
            .checked_mul(8)
            .and_then(|n| n.checked_add(ray_offset))
            .ok_or(ComputeError::InvalidBuffer)?;
        let total = rays
            .len()
            .checked_mul(16)
            .and_then(|n| n.checked_add(hit_offset))
            .ok_or(ComputeError::InvalidBuffer)?;
        if rays.is_empty()
            || rays.len() > u32::MAX as usize
            || total > u32::MAX as usize
            || total.checked_mul(4).is_none_or(|n| n > max_bytes)
        {
            return Err(ComputeError::MemoryBudget);
        }
        let medium_ids: Vec<_> = scene.gpu_medium_ids().collect();
        if medium_ids.len() > u32::MAX as usize {
            return Err(ComputeError::InvalidBuffer);
        }
        let mut words = Vec::with_capacity(total);
        words.extend([
            triangles as u32,
            rays.len() as u32,
            ray_offset as u32,
            hit_offset as u32,
        ]);
        scene.append_gpu_geometry(&mut words)?;
        if words.len() != ray_offset {
            return Err(ComputeError::InvalidBuffer);
        }
        for ray in rays {
            words.extend(ray.origin.map(f32::to_bits));
            words.push(ray.maximum.to_bits());
            words.extend(ray.direction.map(f32::to_bits));
            words.push(ray.minimum.to_bits());
        }
        words.resize(total, 0);
        Ok(Self {
            words,
            medium_ids,
            rays: rays.len(),
            hit_offset,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        [(self.rays as u32).div_ceil(64), 1, 1]
    }
    /// Validate output structure, finite hit geometry and preserved input bytes
    /// before exposing any hit. This does not certify intersection accuracy.
    pub fn decode(&self, bytes: &[u8]) -> Result<Vec<Option<MediumGeometryGpuHit>>, ComputeError> {
        if bytes.len() != self.words.len() * 4
            || bytes[..self.hit_offset * 4] != self.bytes()[..self.hit_offset * 4]
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let words: Vec<_> = bytes
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let mut hits = Vec::with_capacity(self.rays);
        for ray in 0..self.rays {
            let h = &words[self.hit_offset + 16 * ray..self.hit_offset + 16 * (ray + 1)];
            if h[15] != 0 {
                return Err(ComputeError::Validation(
                    "ambiguous or nonfinite optical geometry hit".into(),
                ));
            }
            if h[7] == 2 {
                if h.iter().enumerate().any(|(i, &v)| i != 7 && v != 0) {
                    return Err(ComputeError::InvalidBuffer);
                }
                hits.push(None);
                continue;
            }
            let index = h[8] as usize;
            if h[7] > 1 || index >= self.words[0] as usize {
                return Err(ComputeError::InvalidBuffer);
            }
            let triangle = &self.words[4 + 24 * index..4 + 24 * (index + 1)];
            if h[7] != triangle[20] || h[9] != triangle[23] {
                return Err(ComputeError::InvalidBuffer);
            }
            let rgb = |v: &[u32]| std::array::from_fn::<_, 3, _>(|i| f32::from_bits(v[i]));
            let distance = f32::from_bits(h[0]);
            let position = rgb(&h[1..4]);
            let normal = rgb(&h[4..7]);
            let r = &self.words
                [self.words[2] as usize + 8 * ray..self.words[2] as usize + 8 * (ray + 1)];
            let d = glam::Vec3::from_array(rgb(&r[4..7])).normalize();
            let n = glam::Vec3::from_array(normal);
            let p = glam::Vec3::from_array(position);
            let o = glam::Vec3::from_array(rgb(&r[..3]));
            if !distance.is_finite()
                || distance <= f32::from_bits(r[7])
                || distance > f32::from_bits(r[3])
                || !p.is_finite()
                || !n.is_finite()
                || (n.length() - 1.).abs() > 2e-5
                || n.dot(d) > 1e-5
                || (p - (o + d * distance)).length() > 1e-4 * (1. + p.length().max(o.length()))
            {
                return Err(ComputeError::InvalidBuffer);
            }
            let outward = glam::Vec3::from_array(rgb(&triangle[12..15]));
            let entering = d.dot(outward) < 0.;
            let expected = if entering { outward } else { -outward };
            if (n - expected).length() > 2e-5 {
                return Err(ComputeError::InvalidBuffer);
            }
            let kind = if h[7] == 0 {
                let (incident, transmitted) = if entering {
                    (triangle[22], triangle[21])
                } else {
                    (triangle[21], triangle[22])
                };
                if h[10] != incident || h[11] != transmitted {
                    return Err(ComputeError::InvalidBuffer);
                }
                MediumGeometryHitKind::Boundary {
                    incident: *self
                        .medium_ids
                        .get(incident as usize)
                        .ok_or(ComputeError::InvalidBuffer)?,
                    transmitted: *self
                        .medium_ids
                        .get(transmitted as usize)
                        .ok_or(ComputeError::InvalidBuffer)?,
                }
            } else {
                if h[12..15] != triangle[16..19] || h[10] != u32::MAX || h[11] != u32::MAX {
                    return Err(ComputeError::InvalidBuffer);
                }
                MediumGeometryHitKind::Opaque {
                    radiance: rgb(&h[12..15]),
                }
            };
            hits.push(Some(MediumGeometryGpuHit {
                distance_m: distance,
                position_m: position,
                incident_normal: normal,
                primitive_index: h[8],
                object_index: h[9],
                kind,
            }));
        }
        Ok(hits)
    }
}
pub(crate) fn append_triangle(
    words: &mut Vec<u32>,
    vertices: [[f64; 3]; 3],
    radiance: [f64; 3],
    identity: [u32; 4],
) -> Result<(), ComputeError> {
    let convert = |x: f64| {
        let v = x as f32;
        if !v.is_finite() || (x != 0. && v == 0.) {
            Err(ComputeError::InvalidBuffer)
        } else {
            Ok(v)
        }
    };
    let mut v = [[0.; 3]; 3];
    for i in 0..3 {
        for axis in 0..3 {
            v[i][axis] = convert(vertices[i][axis])?;
        }
    }
    let a = glam::Vec3::from_array(v[0]);
    let b = glam::Vec3::from_array(v[1]);
    let c = glam::Vec3::from_array(v[2]);
    let cross = (b - a).cross(c - a);
    let length = cross.length();
    if !length.is_finite() || length == 0. {
        return Err(ComputeError::InvalidBuffer);
    }
    let source = vertices.map(glam::DVec3::from_array);
    let source_normal = (source[1] - source[0])
        .cross(source[2] - source[0])
        .normalize();
    if !source_normal.is_finite()
        || source_normal.dot((cross / length).to_array().map(f64::from).into()) < 1. - 1e-5
    {
        return Err(ComputeError::InvalidBuffer);
    }
    for vertex in v {
        words.extend(vertex.map(f32::to_bits));
        words.push(0);
    }
    words.extend((cross / length).to_array().map(f32::to_bits));
    words.push(0);
    for x in radiance {
        words.push(convert(x)?.to_bits());
    }
    words.push(0);
    words.extend(identity);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HomogeneousOpticalMedium, MediumBoundaryMesh, OpaqueRadianceMesh};
    fn opaque(z: f32) -> OpaqueRadianceMesh {
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
    fn geometry_input_admits_work_memory_and_numeric_representation() {
        let air =
            HomogeneousOpticalMedium::new(OpticalMediumId(u64::MAX), 1., [0.; 3], [0.; 3]).unwrap();
        let media = [air];
        let scene = CpuMediumTransportScene::new(&media, &[], [-1.; 3], [1.; 3], [1.; 3]).unwrap();
        let ray = MediumGeometryRay::new([0.; 3], [0., 0., -1.], 0., 10.).unwrap();
        assert!(MediumGeometryRay::new([0.; 3], [0.; 3], 0., 10.).is_err());
        assert!(MediumGeometryRay::new([0.; 3], [0., 0., -1.], -1., 10.).is_err());
        assert!(MediumGeometryComputeInput::new(&scene, &[ray], 111, 0).is_err());
        let input = MediumGeometryComputeInput::new(&scene, &[ray], 112, 0).unwrap();
        assert_eq!(input.bytes().len(), 112);
        assert!(input.decode(input.bytes()).is_err()); // unexecuted output is not a miss
        let mut completed = input.words.clone();
        completed[input.hit_offset + 7] = 2;
        assert_eq!(
            input.decode(bytemuck::cast_slice(&completed)).unwrap(),
            vec![None]
        );
        completed[0] = 1;
        assert!(input.decode(bytemuck::cast_slice(&completed)).is_err());
        // f32 can leave a nondegenerate triangle while changing its orientation
        // substantially. Reject the derivative rather than switching media on
        // a different surface normal.
        assert!(
            append_triangle(
                &mut Vec::new(),
                [[1e8, 0., 0.], [1e8 + 1., 1., 0.], [1e8, 0., 1.]],
                [0.; 3],
                [0, 0, 1, 0]
            )
            .is_err()
        );
        let boundary = [MediumBoundaryMesh::from_scene_mesh(
            &crate::medium_geometry::tests::box_mesh(false, false),
            OpticalMediumId(17),
            OpticalMediumId(u64::MAX),
            1.,
            12,
        )
        .unwrap()];
        let water =
            HomogeneousOpticalMedium::new(OpticalMediumId(17), 1.333, [0.; 3], [0.; 3]).unwrap();
        let media = [air, water];
        let scene = CpuMediumTransportScene::new(
            &media,
            &boundary,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        assert_eq!(
            MediumGeometryComputeInput::new(&scene, &[ray], 4096, 11).unwrap_err(),
            ComputeError::WorkBudget
        );
    }
    #[test]
    fn geometry_compute_translates_to_gles_310_with_one_storage_binding() {
        use naga::{back::glsl, proc::BoundsCheckPolicies, valid};
        let module = naga::front::wgsl::parse_str(MEDIUM_GEOMETRY_SHADER).unwrap();
        assert_eq!(module.global_variables.len(), 1);
        let info =
            valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
                .validate(&module)
                .unwrap();
        let options = glsl::Options {
            version: glsl::Version::new_gles(310),
            ..Default::default()
        };
        let pipeline = glsl::PipelineOptions {
            shader_stage: naga::ShaderStage::Compute,
            entry_point: "cs_main".into(),
            multiview: None,
        };
        let mut output = String::new();
        glsl::Writer::new(
            &mut output,
            &module,
            &info,
            &options,
            &pipeline,
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
        input: &MediumGeometryComputeInput,
    ) -> Result<Vec<Option<MediumGeometryGpuHit>>, ComputeError> {
        let job = program.create_job(device, input.bytes())?;
        let mut encoder = device.create_command_encoder(&Default::default());
        let dispatch = job.encode(&mut encoder, input.workgroups())?;
        queue.submit([encoder.finish()]);
        let mut pending = dispatch.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        input.decode(&pending.try_read()?.ok_or(ComputeError::Consumed)?)
    }
    #[test]
    #[ignore = "requires physical GPU; actual metric geometry CPU parity"]
    fn gpu_geometry_queries_match_cpu_and_reject_ambiguous_hits() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        println!("MEDIUM GEOMETRY GPU {:?}", adapter.get_info());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits {
                max_storage_buffers_per_shader_stage: 1,
                ..Default::default()
            },
            ..Default::default()
        }))
        .unwrap();
        assert_eq!(device.limits().max_storage_buffers_per_shader_stage, 1);
        let program =
            pollster::block_on(crate::ComputeProgram::new(&device, MEDIUM_GEOMETRY_SHADER))
                .unwrap();
        let air = OpticalMediumId(u64::MAX);
        let water = OpticalMediumId(17);
        let media = [
            HomogeneousOpticalMedium::new(air, 1., [0.; 3], [0.; 3]).unwrap(),
            HomogeneousOpticalMedium::new(water, 1.333, [0.; 3], [0.; 3]).unwrap(),
        ];
        let boundaries = [MediumBoundaryMesh::from_scene_mesh(
            &crate::medium_geometry::tests::box_mesh(false, false),
            water,
            air,
            1.,
            12,
        )
        .unwrap()];
        let surfaces = [opaque(0.)];
        let scene = CpuMediumTransportScene::with_opaque(
            &media,
            &boundaries,
            &surfaces,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        let mut rays = vec![
            MediumGeometryRay::new([0., 0., 3.], [0., 0., -1.], 0., 100.).unwrap(),
            MediumGeometryRay::new([0., 0., 0.5], [0., 0., -1.], 0., 100.).unwrap(),
            MediumGeometryRay::new([0., 0., 0.5], [0., 0., 1.], 0., 100.).unwrap(),
            MediumGeometryRay::new([0., 0., -3.], [0., 0., 1.], 0., 100.).unwrap(),
            MediumGeometryRay::new([30., 0., 3.], [0., 0., -1.], 0., 100.).unwrap(),
            MediumGeometryRay::new([0., 0., 3.], [0., 0., -1.], 0., 1.).unwrap(),
            MediumGeometryRay::new([0., 0., 3.], [0., 0., -1.], 2.1, 100.).unwrap(),
        ];
        for i in 0..60 {
            let angle = (f64::from(i) - 29.5) * 0.04;
            rays.push(
                MediumGeometryRay::new(
                    [0.125, 0.17, 3.],
                    [angle.sin() as f32, 0., -angle.cos() as f32],
                    0.,
                    100.,
                )
                .unwrap(),
            );
        }
        let input =
            MediumGeometryComputeInput::new(&scene, &rays, 1 << 20, 14 * rays.len()).unwrap();
        assert_eq!(input.workgroups(), [2, 1, 1]);
        let hits = execute(&device, &queue, &program, &input).unwrap();
        let mut checks = 0;
        for (ray, gpu) in rays.iter().zip(&hits) {
            let d = glam::DVec3::from_array(ray.direction.map(f64::from))
                .normalize()
                .to_array();
            let origin = ray.origin.map(f64::from);
            let boundary = boundaries[0]
                .first_hit(origin, d, f64::from(ray.minimum), f64::from(ray.maximum))
                .unwrap();
            // first_distance uses minimum=0; explicit range filter belongs to
            // this reference only for nonzero-minimum queries below.
            let opaque = surfaces[0]
                .first_distance(origin, d, f64::from(ray.maximum))
                .unwrap()
                .filter(|x| *x > f64::from(ray.minimum));
            let nearest = opaque.filter(|x| boundary.is_none_or(|h| *x < h.distance_m));
            if boundary.is_none() && nearest.is_none() {
                assert!(gpu.is_none());
                continue;
            }
            let gpu = gpu.unwrap();
            let expected = nearest.or(boundary.map(|h| h.distance_m)).unwrap();
            assert!((f64::from(gpu.distance_m) - expected).abs() < 1e-4 + 2e-5 * expected.abs());
            checks += 1;
            let p = std::array::from_fn::<_, 3, _>(|i| origin[i] + d[i] * expected);
            for i in 0..3 {
                assert!((f64::from(gpu.position_m[i]) - p[i]).abs() < 2e-4);
                checks += 1;
            }
            if nearest.is_some() {
                assert_eq!(
                    gpu.kind,
                    MediumGeometryHitKind::Opaque {
                        radiance: [2., 3., 4.]
                    }
                );
                assert!(gpu.primitive_index >= 12);
                assert_eq!(gpu.object_index, 0);
            } else {
                let h = boundary.unwrap();
                assert_eq!(
                    gpu.kind,
                    MediumGeometryHitKind::Boundary {
                        incident: h.incident_medium,
                        transmitted: h.transmitted_medium
                    }
                );
                assert_eq!(gpu.primitive_index as usize, h.triangle_index);
                for i in 0..3 {
                    assert!(
                        (f64::from(gpu.incident_normal[i]) - h.incident_normal[i]).abs() < 2e-5
                    );
                    checks += 1;
                }
            }
        }
        let c = -1. / 2_f32.sqrt();
        let crease = [MediumGeometryRay::new([11., 0., 2.], [c, 0., c], 0., 100.).unwrap()];
        let bad = MediumGeometryComputeInput::new(&scene, &crease, 4096, 14).unwrap();
        assert!(matches!(
            execute(&device, &queue, &program, &bad),
            Err(ComputeError::Validation(_))
        ));
        let hidden = [opaque(1.5)];
        let scene = CpuMediumTransportScene::with_opaque(
            &media,
            &boundaries,
            &hidden,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        let input = MediumGeometryComputeInput::new(&scene, &crease, 4096, 14).unwrap();
        assert_eq!(
            execute(&device, &queue, &program, &input).unwrap()[0]
                .unwrap()
                .kind,
            MediumGeometryHitKind::Opaque {
                radiance: [2., 3., 4.]
            }
        );
        let coincident = [opaque(1.)];
        let scene = CpuMediumTransportScene::with_opaque(
            &media,
            &boundaries,
            &coincident,
            [-12., -12., -3.],
            [12., 12., 3.],
            [1.; 3],
        )
        .unwrap();
        let input = MediumGeometryComputeInput::new(&scene, &rays[..1], 4096, 14).unwrap();
        assert!(matches!(
            execute(&device, &queue, &program, &input),
            Err(ComputeError::Validation(_))
        ));
        println!(
            "MEDIUM GEOMETRY PASS rays={} scalar_checks={checks} storage_bindings=1 u64_ids_preserved=true ambiguous_hit_rejected=true hidden_crease_masked=true",
            rays.len()
        );
    }
}
