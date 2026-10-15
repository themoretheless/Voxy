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
    [(c[0] | c[1] << 8 | c[2] << 16) as f32, -10000. - surface.code()]
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
    }
}
