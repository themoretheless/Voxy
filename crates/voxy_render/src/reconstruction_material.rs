//! Linear metallic/roughness material values for Ray Reconstruction guides.
use glam::Vec3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReconstructionMaterialError;
impl std::fmt::Display for ReconstructionMaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid reconstruction material or surface sample")
    }
}
impl std::error::Error for ReconstructionMaterialError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReconstructionMaterial {
    base_color: Vec3,
    metallic: f32,
    roughness: f32,
}
/// Three aligned vec4 values for normal/roughness and linear reflectance guides.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReconstructionMaterialSample {
    pub normal_roughness: [f32; 4],
    pub diffuse_albedo: [f32; 4],
    pub specular_albedo: [f32; 4],
}
/// One GGX NDF-sampled reflection; PDF is per steradian (not conditional on
/// acceptance). Null samples retain their probability mass in the estimator.
#[derive(Clone, Copy, Debug)]
pub struct GgxReflectionSample {
    pub direction: [f32; 3],
    pub pdf: f64,
    pub throughput: [f64; 3],
}
impl ReconstructionMaterial {
    /// Base color must already be linear; dielectric F0 is 0.04.
    /// # Errors
    /// Rejects nonfinite or out-of-range reflectance, metallic and roughness.
    pub fn new(
        base_color: [f32; 3],
        metallic: f32,
        roughness: f32,
    ) -> Result<Self, ReconstructionMaterialError> {
        if base_color
            .into_iter()
            .chain([metallic, roughness])
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
        {
            return Err(ReconstructionMaterialError);
        }
        Ok(Self {
            base_color: Vec3::from_array(base_color),
            metallic,
            roughness,
        })
    }
    /// Delta-mirror throughput using Schlick Fresnel and the material's linear F0.
    /// The normal may face either side; directions are normalized before evaluation.
    /// This is a reflection-only approximation, not transmission or rough GGX.
    /// # Errors
    /// Rejects nonzero roughness and nonfinite/degenerate directions.
    pub fn mirror_throughput(
        &self,
        normal: [f32; 3],
        to_camera: [f32; 3],
    ) -> Result<[f32; 4], ReconstructionMaterialError> {
        if self.roughness != 0.0 {
            return Err(ReconstructionMaterialError);
        }
        let normal = Vec3::from_array(normal)
            .try_normalize()
            .ok_or(ReconstructionMaterialError)?;
        let view = Vec3::from_array(to_camera)
            .try_normalize()
            .ok_or(ReconstructionMaterialError)?;
        let cosine = normal.dot(view).abs().clamp(0.0, 1.0);
        let f0 = Vec3::splat(0.04).lerp(self.base_color, self.metallic);
        let fresnel = f0 + (Vec3::ONE - f0) * (1.0 - cosine).powi(5);
        Ok(fresnel.clamp(Vec3::ZERO, Vec3::ONE).extend(1.0).to_array())
    }
    /// Evaluate isotropic GGX reflection BRDF (per steradian), with correlated
    /// Smith masking and Schlick Fresnel. Perceptual roughness maps to alpha=r².
    /// This is a BSDF value, not a sampling weight or RR reflectance guide.
    /// Both directions point away from the surface; below-surface values are zero.
    /// # Errors
    /// Rejects delta roughness, invalid directions and unrepresentable output.
    pub fn ggx_reflection(
        &self,
        normal: [f32; 3],
        to_camera: [f32; 3],
        to_light: [f32; 3],
    ) -> Result<[f32; 3], ReconstructionMaterialError> {
        if self.roughness == 0.0 {
            return Err(ReconstructionMaterialError);
        }
        let normalize = |v: [f32; 3]| {
            glam::DVec3::from_array(v.map(f64::from))
                .try_normalize()
                .ok_or(ReconstructionMaterialError)
        };
        let n = normalize(normal)?;
        let v = normalize(to_camera)?;
        let l = normalize(to_light)?;
        let nv = n.dot(v).clamp(-1.0, 1.0);
        let nl = n.dot(l).clamp(-1.0, 1.0);
        if nv <= 0.0 || nl <= 0.0 {
            return Ok([0.0; 3]);
        }
        let h = (v + l).try_normalize().ok_or(ReconstructionMaterialError)?;
        let nh = n.dot(h).clamp(0.0, 1.0);
        let vh = v.dot(h).clamp(0.0, 1.0);
        let alpha2 = f64::from(self.roughness).powi(4);
        // Stable near nh=1: avoid subtracting nearly equal values for small alpha.
        let denominator = (1.0 - nh * nh) + alpha2 * nh * nh;
        let distribution = alpha2 / (std::f64::consts::PI * denominator * denominator);
        let lambda = |cosine: f64| {
            ((1.0 + alpha2 * (1.0 - cosine * cosine) / (cosine * cosine)).sqrt() - 1.0) * 0.5
        };
        let masking = 1.0 / (1.0 + lambda(nv) + lambda(nl));
        let scale = distribution * masking / (4.0 * nv * nl);
        let f0 = Vec3::splat(0.04).lerp(self.base_color, self.metallic);
        let values = f0
            .to_array()
            .map(|f0| (f64::from(f0) + (1.0 - f64::from(f0)) * (1.0 - vh).powi(5)) * scale);
        if values
            .iter()
            .any(|x| !x.is_finite() || *x > f64::from(f32::MAX))
        {
            return Err(ReconstructionMaterialError);
        }
        #[allow(clippy::cast_possible_truncation)]
        Ok(values.map(|x| x as f32))
    }

    /// Sample an isotropic GGX microfacet normal from uniform values in [0,1).
    /// Returns None for back-facing view/facet or below-surface reflection; count
    /// it as a zero contribution, without retrying. This uses full NDF sampling,
    /// not visible-normal sampling. Weights may exceed one at grazing angles.
    /// # Errors
    /// Rejects delta roughness, invalid geometry/random values or numeric overflow.
    pub fn sample_ggx_reflection(
        &self,
        normal: [f32; 3],
        to_camera: [f32; 3],
        uniform: [f32; 2],
    ) -> Result<Option<GgxReflectionSample>, ReconstructionMaterialError> {
        if self.roughness == 0.0
            || uniform
                .iter()
                .any(|x| !x.is_finite() || !(0.0..1.0).contains(x))
        {
            return Err(ReconstructionMaterialError);
        }
        let normalize = |v: [f32; 3]| {
            glam::DVec3::from_array(v.map(f64::from))
                .try_normalize()
                .ok_or(ReconstructionMaterialError)
        };
        let n = normalize(normal)?;
        let v = normalize(to_camera)?;
        if n.dot(v) <= 0.0 {
            return Ok(None);
        }
        let axis = if n.z.abs() < 0.9 {
            glam::DVec3::Z
        } else {
            glam::DVec3::X
        };
        let tangent = axis.cross(n).normalize();
        let bitangent = n.cross(tangent);
        let alpha2 = f64::from(self.roughness).powi(4);
        let u = f64::from(uniform[0]);
        let tan2 = alpha2 * u / (1.0 - u);
        let cosine = (1.0 + tan2).sqrt().recip();
        let sine = (tan2 / (1.0 + tan2)).sqrt();
        let phi = std::f64::consts::TAU * f64::from(uniform[1]);
        let h = tangent * (sine * phi.cos()) + bitangent * (sine * phi.sin()) + n * cosine;
        let vh = v.dot(h);
        if vh <= 0.0 {
            return Ok(None);
        }
        let light = (2.0 * vh * h - v).normalize();
        let nl = n.dot(light);
        if nl <= 0.0 {
            return Ok(None);
        }
        let denominator = sine * sine + alpha2 * cosine * cosine;
        let distribution = alpha2 / (std::f64::consts::PI * denominator * denominator);
        let pdf = distribution * cosine / (4.0 * vh);
        #[allow(clippy::cast_possible_truncation)]
        let direction = light.to_array().map(|x| x as f32);
        // Cancel the shared NDF analytically before evaluation. Dividing an f32
        // BRDF by a huge narrow-lobe PDF loses precision or overflows needlessly.
        let nv = n.dot(v).clamp(0.0, 1.0);
        let lambda =
            |c: f64| ((1.0 + alpha2 * (1.0 - c * c).max(0.0) / (c * c)).sqrt() - 1.0) * 0.5;
        let masking = 1.0 / (1.0 + lambda(nv) + lambda(nl));
        let scale = masking * vh / (nv * cosine);
        let f0 = Vec3::splat(0.04).lerp(self.base_color, self.metallic);
        let throughput = f0.to_array().map(|f0| {
            (f64::from(f0) + (1.0 - f64::from(f0)) * (1.0 - vh.clamp(0.0, 1.0)).powi(5)) * scale
        });
        if !pdf.is_finite() || pdf <= 0.0 || throughput.iter().any(|x| !x.is_finite()) {
            return Err(ReconstructionMaterialError);
        }
        Ok(Some(GgxReflectionSample {
            direction,
            pdf,
            throughput,
        }))
    }

    /// Compute guides using normalized world normal and surface-to-camera direction.
    /// Specular reflectance uses the view-dependent `EnvBRDFApprox2` approximation
    /// documented in Streamline 2.14.1's RR guide section 4.2.1.
    /// Reflection hit distance must come separately from actual ray intersections.
    /// # Errors
    /// Rejects nonfinite/degenerate directions or nonfinite BRDF output.
    pub fn sample(
        &self,
        normal: [f32; 3],
        to_camera: [f32; 3],
    ) -> Result<ReconstructionMaterialSample, ReconstructionMaterialError> {
        let normal = Vec3::from_array(normal)
            .try_normalize()
            .ok_or(ReconstructionMaterialError)?;
        let view = Vec3::from_array(to_camera)
            .try_normalize()
            .ok_or(ReconstructionMaterialError)?;
        let f0 = Vec3::splat(0.04).lerp(self.base_color, self.metallic);
        let specular =
            environment_reflectance(f0, self.roughness * self.roughness, normal.dot(view).abs());
        if !specular.is_finite() {
            return Err(ReconstructionMaterialError);
        }
        Ok(ReconstructionMaterialSample {
            normal_roughness: normal.extend(self.roughness).to_array(),
            diffuse_albedo: (self.base_color * (1.0 - self.metallic))
                .extend(1.0)
                .to_array(),
            specular_albedo: specular.clamp(Vec3::ZERO, Vec3::ONE).extend(1.0).to_array(),
        })
    }
}
fn bilinear(coefficients: [[f32; 2]; 2], x: [f32; 2], y: [f32; 2]) -> f32 {
    coefficients
        .into_iter()
        .zip(y)
        .map(|(row, weight)| (row[0] * x[0] + row[1] * x[1]) * weight)
        .sum()
}
fn quadratic(coefficients: [[f32; 3]; 3], x: [f32; 3], y: [f32; 3]) -> f32 {
    coefficients
        .into_iter()
        .zip(y)
        .map(|(row, weight)| (row[0] * x[0] + row[1] * x[1] + row[2] * x[2]) * weight)
        .sum()
}
fn environment_reflectance(f0: Vec3, alpha: f32, cosine: f32) -> Vec3 {
    let c2 = cosine * cosine;
    let a3 = alpha * alpha * alpha;
    let bias = bilinear(
        [[0.99044, -1.28514], [1.29678, -0.755_907]],
        [1.0, cosine],
        [1.0, alpha],
    ) / quadratic(
        [
            [1.0, 2.92338, 59.4188],
            [20.3225, -27.0302, 222.592],
            [121.563, 626.13, 316.627],
        ],
        [1.0, cosine, cosine * c2],
        [1.0, alpha, a3],
    );
    let scale = bilinear(
        [[0.036_546_3, 3.32707], [9.0632, -9.04756]],
        [1.0, cosine],
        [1.0, alpha],
    ) / quadratic(
        [
            [1.0, 3.59685, -1.36772],
            [9.04401, -16.3174, 9.22949],
            [5.56589, 19.7886, -20.2123],
        ],
        [1.0, c2, cosine * c2],
        [1.0, alpha, a3],
    );
    f0 * scale.max(0.0) + Vec3::splat(bias.max(0.0) * (f0.y * 50.0).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metallic_extremes_and_view_dependence() {
        let dielectric = ReconstructionMaterial::new([0.8, 0.4, 0.2], 0.0, 0.0).unwrap();
        let facing = dielectric.sample([0.0, 0.0, 2.0], [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(facing.normal_roughness, [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(facing.diffuse_albedo, [0.8, 0.4, 0.2, 1.0]);
        let grazing = dielectric
            .sample([0.0, 0.0, 1.0], [1.0, 0.0, 0.01])
            .unwrap();
        assert!(grazing.specular_albedo[0] > facing.specular_albedo[0]);
        let metal = ReconstructionMaterial::new([0.8, 0.4, 0.2], 1.0, 0.5).unwrap();
        assert_eq!(
            metal
                .sample([0.0, 1.0, 0.0], [0.0, 1.0, 0.0])
                .unwrap()
                .diffuse_albedo,
            [0.0, 0.0, 0.0, 1.0]
        );
    }
    #[test]
    fn mirror_fresnel_endpoints_and_oblique_angle() {
        let metal = ReconstructionMaterial::new([0.8, 0.4, 0.2], 1.0, 0.0).unwrap();
        assert_eq!(
            metal
                .mirror_throughput([0.0, 0.0, 2.0], [0.0, 0.0, 3.0])
                .unwrap(),
            [0.8, 0.4, 0.2, 1.0]
        );
        assert_eq!(
            metal
                .mirror_throughput([0.0, 0.0, 1.0], [1.0, 0.0, 0.0])
                .unwrap(),
            [1.0; 4]
        );
        let dielectric = ReconstructionMaterial::new([0.8; 3], 0.0, 0.0).unwrap();
        let oblique = dielectric
            .mirror_throughput([0.0, 0.0, 1.0], [3.0_f32.sqrt(), 0.0, 1.0])
            .unwrap();
        for channel in &oblique[..3] {
            assert!((*channel - 0.07).abs() < 0.000_001);
        }
        assert!(metal.mirror_throughput([0.0; 3], [1.0; 3]).is_err());
        assert!(metal.mirror_throughput([1.0; 3], [f32::NAN; 3]).is_err());
        assert!(
            ReconstructionMaterial::new([0.8; 3], 1.0, 0.1)
                .unwrap()
                .mirror_throughput([0.0, 0.0, 1.0], [0.0, 0.0, 1.0])
                .is_err()
        );
    }
    #[test]
    fn invalid_values_and_degenerate_directions_fail() {
        for bad in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert!(ReconstructionMaterial::new([bad, 0.5, 0.5], 0.0, 0.5).is_err());
            assert!(ReconstructionMaterial::new([0.5; 3], bad, 0.5).is_err());
            assert!(ReconstructionMaterial::new([0.5; 3], 0.0, bad).is_err());
        }
        let material = ReconstructionMaterial::new([0.5; 3], 0.5, 0.5).unwrap();
        assert!(material.sample([0.0; 3], [0.0, 0.0, 1.0]).is_err());
        assert!(material.sample([0.0, 0.0, 1.0], [f32::NAN; 3]).is_err());
    }
}

#[cfg(test)]
mod ggx_tests {
    use super::*;
    #[test]
    fn sampled_furnace_matches_independent_hemisphere_integral() {
        let material = ReconstructionMaterial::new([1.0; 3], 1.0, 1.0).unwrap();
        let mut energy = 0.0;
        let cells = 128_u16;
        let mut nulls = 0_u32;
        for x in 0..cells {
            for y in 0..cells {
                let u = [
                    (f32::from(x) + 0.5) / f32::from(cells),
                    (f32::from(y) + 0.5) / f32::from(cells),
                ];
                match material
                    .sample_ggx_reflection([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], u)
                    .unwrap()
                {
                    Some(sample) => {
                        energy += sample.throughput[0];
                        assert!(sample.pdf > 0.0);
                    }
                    None => nulls += 1,
                }
            }
        }
        energy /= f64::from(u32::from(cells).pow(2));
        assert_eq!(nulls, u32::from(cells).pow(2) / 2);
        assert!((energy - (1.0 - 2.0_f64.ln())).abs() < 0.000_1);
        assert!(
            material
                .sample_ggx_reflection([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.5])
                .is_err()
        );
    }
    #[test]
    fn narrow_lobes_keep_finite_pdf_and_mirror_limit_weight() {
        for roughness in [0.001, 1e-10, f32::MIN_POSITIVE] {
            let material = ReconstructionMaterial::new([0.8, 0.4, 0.2], 1.0, roughness).unwrap();
            for u in [0.0, 0.25, 0.75] {
                let sample = material
                    .sample_ggx_reflection([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [u, 0.3])
                    .unwrap()
                    .unwrap();
                assert!(sample.pdf.is_finite() && sample.pdf > 0.0);
                for (weight, expected) in sample.throughput.into_iter().zip([0.8, 0.4, 0.2]) {
                    assert!((weight - expected).abs() < 0.000_001);
                }
            }
        }
    }
    // Uniform solid-angle midpoint quadrature; no GGX sampling/PDF is reused.
    fn furnace(material: ReconstructionMaterial, cosine: f32) -> [f64; 3] {
        let view = [(1.0 - cosine * cosine).sqrt(), 0.0, cosine];
        let mut energy = [0.0; 3];
        let cells = 128_u16;
        for elevation in 0..cells {
            let z = (f32::from(elevation) + 0.5) / f32::from(cells);
            let radius = (1.0 - z * z).sqrt();
            for azimuth in 0..cells {
                let phi = std::f32::consts::TAU * (f32::from(azimuth) + 0.5) / f32::from(cells);
                let light = [radius * phi.cos(), radius * phi.sin(), z];
                let brdf = material
                    .ggx_reflection([0.0, 0.0, 1.0], view, light)
                    .unwrap();
                for (sum, value) in energy.iter_mut().zip(brdf) {
                    *sum += f64::from(value) * f64::from(z) * std::f64::consts::TAU
                        / f64::from(u32::from(cells).pow(2));
                }
            }
        }
        energy
    }
    #[test]
    fn white_furnace_energy_and_analytic_maximum_roughness() {
        let white = ReconstructionMaterial::new([1.0; 3], 1.0, 1.0).unwrap();
        // For alpha=1, F=1, normal view: integral is 1 - ln(2).
        for energy in furnace(white, 1.0) {
            assert!((energy - (1.0 - 2.0_f64.ln())).abs() < 0.000_01);
        }
        for roughness in [0.4, 0.7, 1.0] {
            for cosine in [0.05, 0.25, 0.7, 1.0] {
                for metallic in [0.0, 1.0] {
                    let material =
                        ReconstructionMaterial::new([1.0; 3], metallic, roughness).unwrap();
                    for energy in furnace(material, cosine) {
                        assert!(
                            energy.is_finite() && (0.0..=1.01).contains(&energy),
                            "nonconserving GGX: roughness={roughness} cosine={cosine} metallic={metallic}: {energy}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn analytic_normal_peak_and_reciprocity() {
        let material = ReconstructionMaterial::new([0.8, 0.4, 0.2], 1.0, 0.5).unwrap();
        let peak = material
            .ggx_reflection([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0])
            .unwrap();
        for (actual, base) in peak.into_iter().zip([0.8, 0.4, 0.2]) {
            let expected = base / (4.0 * std::f32::consts::PI * 0.0625);
            assert!((actual - expected).abs() < 0.000_001);
        }
        let v = [0.2, 0.1, 1.0];
        let l = [-0.5, 0.3, 1.0];
        let a = material.ggx_reflection([0.0, 0.0, 2.0], v, l).unwrap();
        let b = material.ggx_reflection([0.0, 0.0, 1.0], l, v).unwrap();
        for (a, b) in a.into_iter().zip(b) {
            assert!((a - b).abs() < 0.000_001);
        }
        assert!(
            material
                .ggx_reflection([0.0, 0.0, 1.0], v, [0.0, 0.0, -1.0])
                .unwrap()
                .iter()
                .all(|value| value.abs() < f32::EPSILON)
        );
        assert!(material.ggx_reflection([f32::NAN; 3], v, l).is_err());
        assert!(
            ReconstructionMaterial::new([1.0; 3], 1.0, 0.0)
                .unwrap()
                .ggx_reflection([0.0, 0.0, 1.0], v, l)
                .is_err()
        );
    }
}
