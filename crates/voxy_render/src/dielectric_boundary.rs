//! Geometric optics at a smooth, lossless, unpolarized dielectric interface.
//! Distinguishes power fractions from radiance arriving along camera branches.
//! This samples an interface, not a traced scene hit.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DielectricBoundarySample {
    reflected: [f64; 3],
    transmitted: Option<[f64; 3]>,
    reflectance: f64,
    transmittance: f64,
    incident_over_transmitted_ior: f64,
}
impl DielectricBoundarySample {
    #[must_use]
    pub fn reflected_direction(self) -> [f64; 3] {
        self.reflected
    }
    #[must_use]
    pub fn transmitted_direction(self) -> Option<[f64; 3]> {
        self.transmitted
    }
    #[must_use]
    pub fn reflected_power_fraction(self) -> f64 {
        self.reflectance
    }
    #[must_use]
    pub fn transmitted_power_fraction(self) -> f64 {
        self.transmittance
    }
    /// Weights for radiance returning from the reflected/transmitted branches
    /// toward a viewer in the incident medium. Camera rays travel away from that
    /// viewer: transmitted branch radiance scales by (n_incident/n_transmitted)^2.
    /// These are deterministic branch weights, not BSDF values or PDF weights.
    /// # Errors
    /// An unrepresentable transmission radiance multiplier.
    pub fn camera_radiance_weights(self) -> Result<[f64; 2], &'static str> {
        // Ordering avoids squaring eta first, which could overflow although the
        // final value is finite when a tiny Fresnel transmission cancels it.
        let transmitted = (self.transmittance * self.incident_over_transmitted_ior)
            * self.incident_over_transmitted_ior;
        if !transmitted.is_finite() {
            return Err("dielectric radiance multiplier overflow");
        }
        Ok([self.reflectance, transmitted])
    }
    /// Sum nonnegative linear RGB radiance from explicitly identified branches.
    /// Supply None exactly when total internal reflection has no transmitted ray.
    /// # Errors
    /// Missing/unexpected branch, nonfinite/negative radiance or arithmetic overflow.
    pub fn camera_radiance(
        self,
        reflected: [f64; 3],
        transmitted: Option<[f64; 3]>,
    ) -> Result<[f64; 3], &'static str> {
        if transmitted.is_some() != self.transmitted.is_some() {
            return Err("dielectric radiance branch mismatch");
        }
        let transmitted = transmitted.unwrap_or([0.; 3]);
        if reflected
            .iter()
            .chain(&transmitted)
            .any(|x| !x.is_finite() || *x < 0.)
        {
            return Err("invalid dielectric branch radiance");
        }
        let [r, t] = self.camera_radiance_weights()?;
        let out = std::array::from_fn(|i| r * reflected[i] + t * transmitted[i]);
        if out.iter().any(|x| !x.is_finite()) {
            return Err("dielectric branch radiance overflow");
        }
        Ok(out)
    }
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn unit(v: [f64; 3]) -> Result<[f64; 3], &'static str> {
    if v.iter().any(|x| !x.is_finite()) {
        return Err("invalid dielectric direction");
    }
    let length = v.iter().fold(0_f64, |a, v| a.hypot(*v));
    if !length.is_finite() || (length - 1.).abs() > 1e-10 {
        return Err("dielectric direction must be unit length");
    }
    Ok(v.map(|x| x / length))
}
/// Incident direction travels TOWARD the boundary. Normal points into the
/// incident medium; its dot product with the incident direction must be <=0.
/// Refractive indices are explicit for the incident and transmitted media.
/// # Errors
/// Nonunit/nonfinite directions, reversed normal, nonpositive/nonfinite indices
/// or an index ratio that cannot be represented in f64.
pub fn dielectric_boundary_sample(
    incident: [f64; 3],
    normal: [f64; 3],
    incident_ior: f64,
    transmitted_ior: f64,
) -> Result<DielectricBoundarySample, &'static str> {
    let d = unit(incident)?;
    let n = unit(normal)?;
    if !incident_ior.is_finite()
        || !transmitted_ior.is_finite()
        || incident_ior <= 0.
        || transmitted_ior <= 0.
    {
        return Err("invalid dielectric refractive index");
    }
    let cosine = -dot(d, n);
    if cosine < 0. {
        return Err("dielectric normal points into transmitted medium");
    }
    let cosine = cosine.min(1.);
    let reflected = std::array::from_fn(|i| d[i] + 2. * cosine * n[i]);
    if incident_ior == transmitted_ior {
        return Ok(DielectricBoundarySample {
            reflected,
            transmitted: Some(d),
            reflectance: 0.,
            transmittance: 1.,
            incident_over_transmitted_ior: 1.,
        });
    }
    let eta = incident_ior / transmitted_ior;
    if !eta.is_finite() || eta == 0. {
        return Err("unrepresentable dielectric index ratio");
    }
    // Tangential norm avoids 1-cos^2 cancellation at almost normal incidence.
    let tangent: [f64; 3] = if d == n.map(|v| -v) {
        [0.; 3]
    } else {
        std::array::from_fn(|i| d[i] + cosine * n[i])
    };
    let sine = tangent.iter().fold(0_f64, |a, v| a.hypot(*v));
    let transmitted_sine = eta * sine;
    if transmitted_sine >= 1. {
        return Ok(DielectricBoundarySample {
            reflected,
            transmitted: None,
            reflectance: 1.,
            transmittance: 0.,
            incident_over_transmitted_ior: eta,
        });
    }
    let transmitted_cosine = (1. - transmitted_sine * transmitted_sine).max(0.).sqrt();
    let transmitted = std::array::from_fn(|i| eta * tangent[i] - transmitted_cosine * n[i]);
    // Normalize indices before Fresnel sums/products to avoid overflowing them.
    let scale = incident_ior.max(transmitted_ior);
    let ni = incident_ior / scale;
    let nt = transmitted_ior / scale;
    let polarization = |a: f64, b: f64| {
        let denominator = a + b;
        let r = (a - b) / denominator;
        // Compute transmitted power directly: subtracting R from one would
        // destroy a small but representable transmission for high contrast.
        (r * r, (2. * a / denominator) * (2. * b / denominator))
    };
    let s = polarization(ni * cosine, nt * transmitted_cosine);
    let p = polarization(nt * cosine, ni * transmitted_cosine);
    let result = DielectricBoundarySample {
        reflected,
        transmitted: Some(transmitted),
        reflectance: 0.5 * (s.0 + p.0),
        transmittance: 0.5 * (s.1 + p.1),
        incident_over_transmitted_ior: eta,
    };
    if !result.reflectance.is_finite()
        || !result.transmittance.is_finite()
        || transmitted.iter().any(|x| !x.is_finite())
    {
        return Err("dielectric boundary arithmetic overflow");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn direction(angle: f64) -> [f64; 3] {
        [angle.sin(), 0., -angle.cos()]
    }
    #[test]
    fn camera_radiance_preserves_refractive_equilibrium_and_slab_flux() {
        // Radiance/n^2 is invariant in refractive equilibrium. Testing distinct
        // branch intensities is essential: unit white on both sides is not an
        // equilibrium state for unequal refractive indices.
        for (ni, nt) in [(1., 1.333), (1.333, 1.), (1., 2.42), (2.42, 1.)] {
            for degrees in [0_f64, 10., 30., 45., 60., 80.] {
                let b = dielectric_boundary_sample(
                    direction(degrees.to_radians()),
                    [0., 0., 1.],
                    ni,
                    nt,
                )
                .unwrap();
                let reflected = [0.3, 2., 7.].map(|x| x * ni * ni);
                let transmitted = b.transmitted.map(|_| [0.3, 2., 7.].map(|x| x * nt * nt));
                let result = b.camera_radiance(reflected, transmitted).unwrap();
                for axis in 0..3 {
                    assert!((result[axis] - reflected[axis]).abs() < 1e-12);
                }
                if let Some(t) = b.transmitted {
                    let exit = dielectric_boundary_sample(t, [0., 0., 1.], nt, ni).unwrap();
                    let entry_weight = b.camera_radiance_weights().unwrap()[1];
                    let exit_weight = exit.camera_radiance_weights().unwrap()[1];
                    let r = b.reflectance;
                    // Infinite plane-parallel transparent slab: all internal
                    // bounce orders, with exact angular scale cancellation.
                    let transmission = entry_weight * exit_weight / (1. - r * r);
                    let reflection = r + entry_weight * exit_weight * r / (1. - r * r);
                    assert!((transmission + reflection - 1.).abs() < 1e-12);
                    assert!((transmission - (1. - r) / (1. + r)).abs() < 1e-12);
                }
            }
        }
        let b = dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1., 1.5).unwrap();
        let out = b.camera_radiance([3., 2., 1.], Some([1., 4., 9.])).unwrap();
        for (actual, expected) in
            out.into_iter()
                .zip([0.12 + 0.96 / 2.25, 0.08 + 3.84 / 2.25, 0.04 + 8.64 / 2.25])
        {
            assert!((actual - expected).abs() < 1e-14);
        }
    }
    #[test]
    fn camera_radiance_rejects_branch_mismatch_and_overflow() {
        let b = dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1., 1.5).unwrap();
        assert!(b.camera_radiance([0.; 3], None).is_err());
        for invalid in [f64::NAN, f64::INFINITY, -1.] {
            assert!(b.camera_radiance([invalid; 3], Some([0.; 3])).is_err());
            assert!(b.camera_radiance([0.; 3], Some([invalid; 3])).is_err());
        }
        let tir = dielectric_boundary_sample(direction(80_f64.to_radians()), [0., 0., 1.], 1.5, 1.)
            .unwrap();
        assert_eq!(
            tir.camera_radiance([1., 2., 3.], None).unwrap(),
            [1., 2., 3.]
        );
        assert!(tir.camera_radiance([1.; 3], Some([0.; 3])).is_err());
        let large = dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1e200, 1.).unwrap();
        let weight = large.camera_radiance_weights().unwrap()[1];
        assert!((weight / 4e200 - 1.).abs() < 1e-14);
        assert!(large.camera_radiance([0.; 3], Some([f64::MAX; 3])).is_err());
        assert!(
            dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1e308, 1.)
                .unwrap()
                .camera_radiance_weights()
                .is_err()
        );
    }
    #[test]
    fn normal_incidence_identity_and_brewster_angle() {
        let sample = dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1., 1.5).unwrap();
        assert_eq!(sample.reflected_direction(), [0., 0., 1.]);
        assert_eq!(sample.transmitted_direction(), Some([0., 0., -1.]));
        assert!((sample.reflectance - 0.04).abs() < 1e-15);
        assert!((sample.transmittance - 0.96).abs() < 1e-15);
        let grazing = dielectric_boundary_sample([1., 0., 0.], [0., 0., 1.], 1.333, 1.333).unwrap();
        assert_eq!(grazing.transmitted_direction(), Some([1., 0., 0.]));
        assert_eq!(grazing.reflectance, 0.);
        let brewster =
            dielectric_boundary_sample(direction(1.5_f64.atan()), [0., 0., 1.], 1., 1.5).unwrap();
        // At Brewster angle p-polarized reflectance vanishes; independent s value.
        let rs = ((1. - 1.5_f64.powi(2)) / (1. + 1.5_f64.powi(2))).powi(2);
        assert!((brewster.reflectance - 0.5 * rs).abs() < 1e-14);
    }
    #[test]
    fn snell_flux_and_reciprocity_for_entering_and_exiting_media() {
        for (ni, nt) in [(1., 1.333), (1.333, 1.), (1., 2.42), (2.42, 1.)] {
            for degrees in [0_f64, 10., 30., 45., 60., 80., 89.] {
                let d = direction(degrees.to_radians());
                let s = dielectric_boundary_sample(d, [0., 0., 1.], ni, nt).unwrap();
                assert!((s.reflectance + s.transmittance - 1.).abs() < 1e-14);
                assert!((dot(s.reflected, s.reflected) - 1.).abs() < 1e-14);
                if let Some(t) = s.transmitted {
                    assert!((dot(t, t) - 1.).abs() < 1e-14);
                    assert!((ni * d[0] - nt * t[0]).abs() < 1e-14);
                    let reverse =
                        dielectric_boundary_sample(t.map(|x| -x), [0., 0., -1.], nt, ni).unwrap();
                    let returned = reverse.transmitted.unwrap();
                    for axis in 0..3 {
                        assert!((returned[axis] + d[axis]).abs() < 1e-13);
                    }
                    assert!((reverse.reflectance - s.reflectance).abs() < 1e-13);
                } else {
                    assert!(ni * d[0] >= nt);
                    assert_eq!(s.reflectance, 1.);
                }
            }
        }
        let critical = (1_f64 / 1.5).asin();
        assert!(
            dielectric_boundary_sample(direction(critical - 1e-8), [0., 0., 1.], 1.5, 1.)
                .unwrap()
                .transmitted
                .is_some()
        );
        assert!(
            dielectric_boundary_sample(direction(critical + 1e-8), [0., 0., 1.], 1.5, 1.)
                .unwrap()
                .transmitted
                .is_none()
        );
    }
    #[test]
    fn invalid_inputs_and_high_contrast_do_not_silently_default() {
        for bad in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], bad, 1.).is_err());
            assert!(dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1., bad).is_err());
        }
        assert!(dielectric_boundary_sample([0., 0., -2.], [0., 0., 1.], 1., 1.5).is_err());
        assert!(dielectric_boundary_sample([0., 0., -1.], [0., 0., -1.], 1., 1.5).is_err());
        assert!(
            dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], f64::MAX, f64::MIN_POSITIVE)
                .is_err()
        );
        let high = dielectric_boundary_sample([0., 0., -1.], [0., 0., 1.], 1., 1e300).unwrap();
        assert!(high.transmittance > 0.);
        assert!((high.transmittance / 4e-300 - 1.).abs() < 1e-14);
        let scaled =
            dielectric_boundary_sample(direction(0.5), [0., 0., 1.], 1e300, 1.5e300).unwrap();
        let ordinary = dielectric_boundary_sample(direction(0.5), [0., 0., 1.], 1., 1.5).unwrap();
        assert!((scaled.reflectance - ordinary.reflectance).abs() < 1e-14);
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::*;
    use wgpu::util::DeviceExt;
    #[test]
    #[ignore = "requires physical GPU; dielectric CPU parity"]
    fn dielectric_gpu_matches_cpu_reference() {
        let instance = crate::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        println!("DIELECTRIC GPU {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let source = format!(
            "{}\n{}",
            include_str!("dielectric_boundary.wgsl"),
            r"
struct Probe { incident_ni:vec4f, normal_nt:vec4f }
struct Result { reflected:vec4f, transmitted:vec4f, fractions:vec4f, weights:vec4f, radiance:vec4f }
@group(0) @binding(0) var<storage,read> probes:array<Probe>;
@group(0) @binding(1) var<storage,read_write> results:array<Result>;
@compute @workgroup_size(1) fn check(@builtin(global_invocation_id) id:vec3u) {
    let p=probes[id.x];
    let b=dielectric_boundary(normalize(p.incident_ni.xyz),normalize(p.normal_nt.xyz),p.incident_ni.w,p.normal_nt.w);
    results[id.x]=Result(vec4f(b.reflected,0.0),vec4f(b.transmitted,0.0),vec4f(b.reflectance,b.transmittance,select(0.0,1.0,b.has_transmission),0.0),vec4f(dielectric_camera_radiance_weights(b),0.0,0.0),vec4f(dielectric_camera_radiance(b,vec3f(3.0,2.0,1.0),vec3f(1.0,4.0,9.0)),0.0));
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

        let mut probes = Vec::<[[f32; 4]; 2]>::new();
        // Rotated normals exercise the actual world-space decomposition, not
        // only the trivial coordinate-axis case. Keep away from f32 ambiguity
        // at the exact critical angle; test each side explicitly below.
        for (ni, nt) in [
            (1., 1.333),
            (1.333, 1.),
            (1., 2.42),
            (2.42, 1.),
            (1., 1.),
            (1., 1e6),
            (1e6, 1.),
        ] {
            for angle in [0_f64, 0.001, 10., 30., 45., 60., 80., 89., 90.] {
                for roll in [0_f64, 0.37, 1.2] {
                    let a = angle.to_radians();
                    let n = [roll.sin(), 0., roll.cos()];
                    let tangent = [roll.cos(), 0., -roll.sin()];
                    let d: [f32; 3] =
                        std::array::from_fn(|i| (a.sin() * tangent[i] - a.cos() * n[i]) as f32);
                    // Exact grazing on rounded rotated normals has an ambiguous
                    // sign: use axis normal for this endpoint, not a clamped CPU oracle.
                    if angle == 90. && roll != 0. {
                        continue;
                    }
                    probes.push([
                        [d[0], d[1], d[2], ni],
                        [n[0] as f32, n[1] as f32, n[2] as f32, nt],
                    ]);
                }
            }
        }
        for delta in [-1e-3, 1e-3] {
            let a = (1_f64 / 1.5).asin() + delta;
            probes.push([[a.sin() as f32, 0., -a.cos() as f32, 1.5], [0., 0., 1., 1.]]);
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
            let unit = |v: [f32; 4]| {
                let length = v[..3]
                    .iter()
                    .map(|x| f64::from(*x).powi(2))
                    .sum::<f64>()
                    .sqrt();
                [
                    f64::from(v[0]) / length,
                    f64::from(v[1]) / length,
                    f64::from(v[2]) / length,
                ]
            };
            let b = dielectric_boundary_sample(
                unit(p[0]),
                unit(p[1]),
                f64::from(p[0][3]),
                f64::from(p[1][3]),
            )
            .unwrap();
            assert_eq!(
                actual[2][2],
                if b.transmitted.is_some() { 1. } else { 0. },
                "probe={p:?}"
            );
            let weights = b.camera_radiance_weights().unwrap();
            let expected = [
                b.reflected,
                b.transmitted.unwrap_or([0.; 3]),
                [b.reflectance, b.transmittance, actual[2][2] as f64],
                [weights[0], weights[1], 0.],
                b.camera_radiance([3., 2., 1.], b.transmitted.map(|_| [1., 4., 9.]))
                    .unwrap(),
            ];
            for (gpu, cpu) in actual.iter().zip(expected) {
                for axis in 0..3 {
                    let a = f64::from(gpu[axis]);
                    let e = cpu[axis];
                    assert!(
                        a.is_finite() && (a - e).abs() <= 2e-5 + 2e-5 * e.abs(),
                        "probe={p:?} gpu={a} cpu={e}"
                    );
                    checks += 1;
                }
            }
            assert!((actual[2][0] + actual[2][1] - 1.).abs() < 2e-6);
        }
        drop(mapped);
        readback.unmap();
        assert!(pollster::block_on(scope.pop()).is_none());
        println!(
            "DIELECTRIC GPU PASS probes={} scalar_checks={checks}",
            probes.len()
        );
    }
}
