//! Immutable linear-radiance transfer along an already identified ray segment.
//! This owns no simulation inventory, boundary geometry or ray traversal.

/// RGB transfer `source + transmission * background`, directed toward the viewer.
/// Source radiance includes whatever illumination was integrated on this segment;
/// these values do not infer a phase function or refractive interface response.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpticalSegment {
    transmission: [f64; 3],
    source: [f64; 3],
}
impl OpticalSegment {
    /// Constant extinction (1/m), source radiance per metre and segment length (m).
    /// # Errors
    /// Rejects negative/nonfinite coefficients or length and unrepresentable radiance.
    pub fn homogeneous(
        extinction: [f64; 3],
        source_per_m: [f64; 3],
        length_m: f64,
    ) -> Result<Self, &'static str> {
        if !length_m.is_finite()
            || length_m < 0.
            || extinction
                .iter()
                .chain(&source_per_m)
                .any(|x| !x.is_finite() || *x < 0.)
        {
            return Err("invalid homogeneous optical segment");
        }
        let mut segment = Self {
            transmission: [1.; 3],
            source: [0.; 3],
        };
        for axis in 0..3 {
            let sigma = extinction[axis];
            let tau = sigma * length_m;
            segment.transmission[axis] = (-tau).exp();
            // Integral exp(-sigma*s) ds. The thin/zero branch avoids catastrophic
            // subtraction and division by a coefficient whose product underflows.
            // The thick branch also handles an overflowing optical depth: T=0,
            // but the source integral remains finite and bounded by length_m.
            let integral = if tau == 0. {
                length_m
            } else if tau < 1. {
                length_m * (-(-tau).exp_m1() / tau)
            } else {
                -(-tau).exp_m1() / sigma
            };
            segment.source[axis] = source_per_m[axis] * integral;
        }
        if segment.source.iter().any(|x| !x.is_finite()) {
            return Err("optical segment radiance overflow");
        }
        Ok(segment)
    }
    /// Coexisting constituents occupying the SAME segment: sum their local
    /// coefficients before integration. This is not a sequence of layered slabs.
    /// Phase-dependent scattering source terms must be supplied by the caller.
    /// # Errors
    /// Invalid coefficients/length or overflow in coefficient sums/radiance.
    pub fn homogeneous_mixture(
        constituents: &[([f64; 3], [f64; 3])],
        length_m: f64,
    ) -> Result<Self, &'static str> {
        let mut extinction = [0.; 3];
        let mut source = [0.; 3];
        for (sigma, q) in constituents {
            if sigma.iter().chain(q).any(|x| !x.is_finite() || *x < 0.) {
                return Err("invalid optical constituent");
            }
            for axis in 0..3 {
                extinction[axis] += sigma[axis];
                source[axis] += q[axis];
            }
        }
        Self::homogeneous(extinction, source, length_m)
    }
    #[must_use]
    pub fn transmission(&self) -> [f64; 3] {
        self.transmission
    }
    #[must_use]
    pub fn source_radiance(&self) -> [f64; 3] {
        self.source
    }
    /// Compose this FRONT segment with a segment farther from the viewer.
    /// The caller must establish ray identity, adjacency and interface ordering.
    /// # Errors
    /// Rejects unrepresentable accumulated radiance; neither segment is mutated.
    pub fn then(self, back: Self) -> Result<Self, &'static str> {
        let source =
            std::array::from_fn(|i| self.source[i] + self.transmission[i] * back.source[i]);
        if source.iter().any(|v| !v.is_finite()) {
            return Err("optical segment radiance overflow");
        }
        Ok(Self {
            transmission: std::array::from_fn(|i| self.transmission[i] * back.transmission[i]),
            source,
        })
    }
    /// Apply the transfer to linear HDR background RGB, preserving coverage alpha.
    /// # Errors
    /// Negative/nonfinite background, alpha outside 0..1 or radiance overflow.
    pub fn apply_rgba(self, background: [f64; 4]) -> Result<[f64; 4], &'static str> {
        if background.iter().any(|v| !v.is_finite() || *v < 0.) || background[3] > 1. {
            return Err("invalid optical background");
        }
        let mut result = background;
        for (axis, out) in result[..3].iter_mut().enumerate() {
            *out = self.source[axis] + self.transmission[axis] * background[axis];
        }
        if result.iter().any(|v| !v.is_finite()) {
            return Err("optical segment radiance overflow");
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: [f64; 3], b: [f64; 3]) {
        for (a, b) in a.into_iter().zip(b) {
            assert!(
                (a - b).abs() <= 2e-13 * (1. + a.abs().max(b.abs())),
                "{a} != {b}"
            );
        }
    }
    #[test]
    fn homogeneous_transfer_partitions_without_changing_radiance() {
        for sigma in [0., 1e-14, 0.1, 1., 100., 1e300] {
            let all = OpticalSegment::homogeneous([sigma; 3], [3., 1., 2.], 1.).unwrap();
            let mut partition = OpticalSegment::homogeneous([0.; 3], [0.; 3], 0.).unwrap();
            for _ in 0..17 {
                partition = partition
                    .then(OpticalSegment::homogeneous([sigma; 3], [3., 1., 2.], 1. / 17.).unwrap())
                    .unwrap();
            }
            near(all.transmission(), partition.transmission());
            near(all.source_radiance(), partition.source_radiance());
            let rgba = all.apply_rgba([2., 4., 8., 0.37]).unwrap();
            assert_eq!(rgba[3], 0.37);
        }
        let vacuum = OpticalSegment::homogeneous([0.; 3], [0.; 3], 99.).unwrap();
        assert_eq!(
            vacuum.apply_rgba([5., 2., 7., 0.2]).unwrap(),
            [5., 2., 7., 0.2]
        );
        let emissive = OpticalSegment::homogeneous([0.; 3], [3., 1., 2.], 2.).unwrap();
        assert_eq!(emissive.source_radiance(), [6., 2., 4.]);
    }
    #[test]
    fn overlap_and_ordered_layers_have_distinct_independent_solutions() {
        let a = ([1.; 3], [3., 0., 0.]);
        let b = ([1.; 3], [0., 0., 2.]);
        let mixture = OpticalSegment::homogeneous_mixture(&[a, b], 1.).unwrap();
        near(
            mixture.source_radiance(),
            [1.296997075145081, 0., 0.8646647167633873],
        );
        let front = OpticalSegment::homogeneous(a.0, a.1, 1.).unwrap();
        let back = OpticalSegment::homogeneous(b.0, b.1, 1.).unwrap();
        near(
            front.then(back).unwrap().source_radiance(),
            [1.896361676485673, 0., 0.46508831586965926],
        );
        near(
            back.then(front).unwrap().source_radiance(),
            [0.6976324738044889, 0., 1.2642411176571153],
        );
        let c = OpticalSegment::homogeneous([0.2, 0.5, 1.], [0.1, 0.3, 0.7], 0.4).unwrap();
        let left = front.then(back).unwrap().then(c).unwrap();
        let right = front.then(back.then(c).unwrap()).unwrap();
        near(left.source_radiance(), right.source_radiance());
        near(left.transmission(), right.transmission());
    }
    #[test]
    fn thin_dense_underflow_and_rejections_are_explicit() {
        let thin = OpticalSegment::homogeneous([1e-300; 3], [1e300; 3], 1e-300).unwrap();
        near(thin.source_radiance(), [1.; 3]);
        let dense = OpticalSegment::homogeneous([f64::MAX; 3], [f64::MAX; 3], 2.).unwrap();
        assert_eq!(dense.transmission(), [0.; 3]);
        near(dense.source_radiance(), [1.; 3]);
        for bad in [-1., f64::NAN, f64::INFINITY] {
            assert!(OpticalSegment::homogeneous([bad; 3], [0.; 3], 1.).is_err());
            assert!(OpticalSegment::homogeneous([0.; 3], [bad; 3], 1.).is_err());
            assert!(OpticalSegment::homogeneous([0.; 3], [0.; 3], bad).is_err());
        }
        assert!(OpticalSegment::homogeneous([0.; 3], [f64::MAX; 3], 2.).is_err());
        let large = OpticalSegment::homogeneous([0.; 3], [f64::MAX; 3], 1.).unwrap();
        assert!(large.then(large).is_err());
        assert_eq!(
            large
                .apply_rgba([f64::MAX, f64::MAX, f64::MAX, 1.])
                .unwrap_err(),
            "optical segment radiance overflow"
        );
        assert_eq!(
            large.apply_rgba([1., 1., 1., 2.]).unwrap_err(),
            "invalid optical background"
        );
        let saved = large;
        assert!(large.apply_rgba([f64::NAN; 4]).is_err());
        assert_eq!(saved, large);
        assert!(OpticalSegment::homogeneous_mixture(&[([f64::MAX; 3], [0.; 3]); 2], 1.).is_err());
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::*;
    use wgpu::util::DeviceExt;
    #[test]
    #[ignore = "requires physical GPU; optical transfer CPU parity"]
    fn medium_gpu_transfer_matches_cpu_reference() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        println!("MEDIUM GPU {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let source = format!(
            "{}\n{}",
            include_str!("medium_transport.wgsl"),
            r"
struct Probe { sigma_length:vec4f, source:vec4f, back_sigma_length:vec4f, back_source:vec4f, background:vec4f }
struct Result { transmission:vec4f, source:vec4f, combined_transmission:vec4f, combined_source:vec4f, rgba:vec4f }
@group(0) @binding(0) var<storage,read> probes:array<Probe>;
@group(0) @binding(1) var<storage,read_write> results:array<Result>;
@compute @workgroup_size(1) fn check(@builtin(global_invocation_id) id:vec3u) {
    let p=probes[id.x];
    let front=medium_homogeneous(p.sigma_length.xyz,p.source.xyz,p.sigma_length.w);
    let back=medium_homogeneous(p.back_sigma_length.xyz,p.back_source.xyz,p.back_sigma_length.w);
    let combined=medium_compose(front,back);
    results[id.x]=Result(vec4f(front.transmission,0.0),vec4f(front.source,0.0),vec4f(combined.transmission,0.0),vec4f(combined.source,0.0),vec4f(medium_apply(combined,p.background.xyz),p.background.w));
}"
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("production medium transfer"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("check"),
            compilation_options: Default::default(),
            cache: None,
        });
        let mut probes = Vec::<[[f32; 4]; 5]>::new();
        for sigma in [0., 1e-8, 0.001, 0.01, 1., 100., 1e6, 1e38] {
            for length in [0., 0.001, 1., 100.] {
                probes.push([
                    [sigma, sigma * 0.5, 0., length],
                    if sigma > 1e30 {
                        [sigma, sigma * 0.5, 2., 0.]
                    } else {
                        [3., 1., 2., 0.]
                    },
                    [0.7, 0.2, 1., 0.4],
                    [0.1, 0.3, 0.7, 0.],
                    [2., 4., 8., 0.37],
                ]);
            }
        }
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&probes),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bytes = (probes.len() * 80) as u64;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(probes.len() as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, bytes);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        let values: &[[[f32; 4]; 5]] = bytemuck::cast_slice(&mapped);
        let mut checks = 0;
        for (p, actual) in probes.iter().zip(values) {
            let rgb = |v: [f32; 4]| [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])];
            let front =
                OpticalSegment::homogeneous(rgb(p[0]), rgb(p[1]), f64::from(p[0][3])).unwrap();
            let back =
                OpticalSegment::homogeneous(rgb(p[2]), rgb(p[3]), f64::from(p[2][3])).unwrap();
            let combined = front.then(back).unwrap();
            let expected = [
                front.transmission(),
                front.source_radiance(),
                combined.transmission(),
                combined.source_radiance(),
            ];
            for (gpu, cpu) in actual[..4].iter().zip(expected) {
                for axis in 0..3 {
                    let a = f64::from(gpu[axis]);
                    let b = cpu[axis];
                    assert!(
                        a.is_finite() && (a - b).abs() <= 2e-5 * b.abs() + 1e-6,
                        "probe={p:?} gpu={a} cpu={b}"
                    );
                    checks += 1;
                }
            }
            let rgba = combined.apply_rgba(p[4].map(f64::from)).unwrap();
            for axis in 0..4 {
                assert!(
                    (f64::from(actual[4][axis]) - rgba[axis]).abs()
                        <= 2e-5 * rgba[axis].abs() + 1e-6
                );
                checks += 1;
            }
            assert_eq!(actual[4][3].to_bits(), p[4][3].to_bits());
        }
        drop(mapped);
        readback.unmap();
        assert!(pollster::block_on(scope.pop()).is_none());
        println!(
            "MEDIUM TRANSFER GPU PASS probes={} scalar_checks={checks} actual_shader=true cpu_reference=true alpha_bits_preserved=true",
            probes.len()
        );
    }
}

#[cfg(test)]
mod portability_tests {
    #[test]
    fn medium_transfer_translates_to_gles_without_storage() {
        use naga::{back::glsl, proc::BoundsCheckPolicies, valid};
        let source = format!(
            "{}\n{}",
            include_str!("medium_transport.wgsl"),
            r"
@fragment fn transport(@builtin(position) position:vec4f)->@location(0) vec4f {
    let a=medium_homogeneous(abs(position.xyz)+vec3f(1.0),vec3f(3.0,1.0,2.0),1.0);
    let b=medium_homogeneous(vec3f(0.2),vec3f(0.1),0.3);
    return vec4f(medium_apply(medium_compose(a,b),vec3f(1.0)),1.0);
}"
        );
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        let info = valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::all())
            .validate(&module)
            .unwrap();
        let options = glsl::Options {
            version: glsl::Version::new_gles(300),
            ..Default::default()
        };
        let pipeline = glsl::PipelineOptions {
            shader_stage: naga::ShaderStage::Fragment,
            entry_point: "transport".into(),
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
        assert!(output.starts_with("#version 300 es"));
        assert!(!output.contains(" buffer "));
    }
}
