//! Packed per-vertex character materials for the native skinned pipelines.
//! UV.y <= -10000 selects packed linear RGB in UV.x; the remainder below
//! -10000 selects the shading model (see `is_skin`/`is_eye` in
//! voxel_photoreal.wgsl).

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CharacterSurface {
    /// Fur, cloth or other plain coats. Roughness 0.04..=1.
    Coat { roughness: f32 },
    /// Pre-integrated subsurface skin. Roughness 0.04..0.48.
    Skin { roughness: f32 },
    /// Diffuse sclera/iris under a wet corneal coat.
    Eye,
    /// Fibre scattering along the encoded strand tangent.
    Hair,
}

impl CharacterSurface {
    fn code(self) -> f32 {
        match self {
            Self::Coat { roughness } => roughness.clamp(0.04, 1.),
            Self::Skin { roughness } => 1. + roughness.clamp(0.04, 0.48),
            Self::Eye => 1.75,
            Self::Hair => 2.,
        }
    }
}

/// Returns the UV pair that encodes `colour` (linear, 0..=1) and `surface`.
pub fn encode_character_uv(colour: [f32; 3], surface: CharacterSurface) -> [f32; 2] {
    let c = colour.map(|x| (x.clamp(0., 1.) * 255.).round() as u32);
    [
        (c[0] | c[1] << 8 | c[2] << 16) as f32,
        -10000. - surface.code(),
    ]
}

/// Evaluates Kaplanyan & Toksvig Specular Anti-Aliasing roughness in `[0.04, 1.0]` from base
/// `roughness` and normal-map screen-space derivatives `dndu` / `dndv` (or averaged normal length),
/// widening the GGX lobe on subpixel high-frequency normal variations to prevent specular aliasing.
#[must_use]
pub fn specular_aa_roughness(
    base_roughness: f32,
    dndu: [f32; 3],
    dndv: [f32; 3],
    max_variance: f32,
) -> f32 {
    let r = if base_roughness.is_finite() {
        base_roughness.clamp(0.04, 1.0)
    } else {
        0.5
    };
    if dndu.iter().chain(&dndv).any(|v| !v.is_finite())
        || !max_variance.is_finite()
        || max_variance <= 0.0
    {
        return r;
    }
    let du2 = dndu[0] * dndu[0] + dndu[1] * dndu[1] + dndu[2] * dndu[2];
    let dv2 = dndv[0] * dndv[0] + dndv[1] * dndv[1] + dndv[2] * dndv[2];
    let kernel_roughness2 = (0.5 * (du2 + dv2)).min(max_variance);
    (r * r + kernel_roughness2).sqrt().clamp(0.04, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded(surface: CharacterSurface) -> f32 {
        -encode_character_uv([0.5; 3], surface)[1] - 10000.
    }

    #[test]
    fn surfaces_land_in_disjoint_shader_ranges() {
        let coat = decoded(CharacterSurface::Coat { roughness: 2. });
        let skin_low = decoded(CharacterSurface::Skin { roughness: 0. });
        let skin_high = decoded(CharacterSurface::Skin { roughness: 1. });
        let eye = decoded(CharacterSurface::Eye);
        let hair = decoded(CharacterSurface::Hair);
        assert!(coat <= 1.);
        assert!((1.02..1.5).contains(&skin_low) && (1.02..1.5).contains(&skin_high));
        assert!((1.5..2.).contains(&eye));
        assert!(hair >= 2.);
    }

    #[test]
    fn colour_packs_into_exact_float_integer() {
        let [packed, _] = encode_character_uv([1., 0., 0.5], CharacterSurface::Eye);
        assert_eq!(packed as u32, 255 | 128 << 16);
        let smooth = specular_aa_roughness(0.1, [0.0; 3], [0.0; 3], 0.18);
        let filtered = specular_aa_roughness(0.1, [0.3, 0.0, 0.0], [0.0, 0.3, 0.0], 0.18);
        assert!((smooth - 0.1).abs() < 1e-6);
        assert!(filtered > smooth);
    }
}
