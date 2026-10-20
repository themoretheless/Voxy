//! Original breed anatomy studies. Neutral, stationary surfaces before grooming.
//! This module intentionally does not depend on the rejected companion prototype.
mod glb;
mod ears;
mod sculpt;
mod surface;
use glam::Vec3;
pub use surface::AnatomySurface;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species {
    Dog,
    Cat,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Breed {
    Labrador,
    GermanShepherd,
    SiberianHusky,
    DomesticShorthair,
    BritishShorthair,
    MaineCoon,
}
impl Breed {
    pub const ALL: [Self; 6] = [
        Self::Labrador,
        Self::GermanShepherd,
        Self::SiberianHusky,
        Self::DomesticShorthair,
        Self::BritishShorthair,
        Self::MaineCoon,
    ];
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Labrador => "labrador",
            Self::GermanShepherd => "german-shepherd",
            Self::SiberianHusky => "husky",
            Self::DomesticShorthair => "domestic-shorthair",
            Self::BritishShorthair => "british-shorthair",
            Self::MaineCoon => "maine-coon",
        }
    }
    pub fn parse(id: &str) -> Result<Self, String> {
        Self::ALL.into_iter().find(|b| b.id() == id).ok_or_else(|| {
            format!(
                "unknown breed {id}; expected {}",
                Self::ALL.map(Self::id).join(", ")
            )
        })
    }
    #[must_use]
    pub const fn species(self) -> Species {
        match self {
            Self::Labrador | Self::GermanShepherd | Self::SiberianHusky => Species::Dog,
            _ => Species::Cat,
        }
    }
    #[must_use]
    pub fn anatomy(self) -> BreedAnatomy {
        // Metres. Dog shoulder targets lie within the cited adult male FCI
        // intervals. Other dimensions are authored study targets, not standards.
        let (
            height,
            length,
            width,
            depth,
            rump,
            head_width,
            head_length,
            muzzle,
            ear,
            tail,
            paw,
            neck,
        ) = match self {
            Self::Labrador => (
                0.565,
                0.66,
                0.125,
                0.285,
                0.55,
                0.185,
                0.225,
                0.095,
                Ear::Folded { length: 0.105 },
                0.34,
                0.043,
                0.145,
            ),
            Self::GermanShepherd => (
                0.625,
                0.765,
                0.132,
                0.31,
                0.585,
                0.125,
                0.265,
                0.135,
                Ear::Upright { height: 0.112 },
                0.43,
                0.034,
                0.155,
            ),
            Self::SiberianHusky => (
                0.56,
                0.66,
                0.122,
                0.275,
                0.548,
                0.116,
                0.224,
                0.100,
                Ear::Upright { height: 0.080 },
                0.34,
                0.031,
                0.130,
            ),
            Self::DomesticShorthair => (
                0.27,
                0.37,
                0.052,
                0.143,
                0.278,
                0.085,
                0.079,
                0.021,
                Ear::Upright { height: 0.041 },
                0.285,
                0.018,
                0.067,
            ),
            Self::BritishShorthair => (
                0.285,
                0.365,
                0.085,
                0.156,
                0.292,
                0.113,
                0.090,
                0.032,
                Ear::Upright { height: 0.034 },
                0.248,
                0.024,
                0.068,
            ),
            Self::MaineCoon => (
                0.345,
                0.495,
                0.082,
                0.18,
                0.35,
                0.105,
                0.112,
                0.036,
                Ear::Upright { height: 0.062 },
                0.40,
                0.026,
                0.084,
            ),
        };
        BreedAnatomy {
            breed: self,
            shoulder_height: height,
            trunk_length: length,
            rib_half_width: width,
            rib_depth: depth,
            rump_height: rump,
            skull_width: head_width,
            head_length,
            muzzle_length: muzzle,
            ears: ear,
            tail_length: tail,
            paw_half_width: paw,
            neck_length: neck,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ear {
    Folded { length: f32 },
    Upright { height: f32 },
}
#[derive(Clone, Copy, Debug)]
pub struct BreedAnatomy {
    pub breed: Breed,
    pub shoulder_height: f32,
    pub trunk_length: f32,
    pub rib_half_width: f32,
    pub rib_depth: f32,
    pub rump_height: f32,
    pub skull_width: f32,
    pub head_length: f32,
    pub muzzle_length: f32,
    pub ears: Ear,
    pub tail_length: f32,
    pub paw_half_width: f32,
    pub neck_length: f32,
}
#[derive(Clone, Debug)]
pub struct Landmark {
    pub name: &'static str,
    pub position: Vec3,
}
impl BreedAnatomy {
    #[must_use]
    pub fn head_origin(&self) -> Vec3 {
        let cat = self.breed.species() == Species::Cat;
        Vec3::new(
            0.0,
            self.shoulder_height + self.neck_length * if cat { 0.15 } else { 0.55 },
            self.trunk_length * 0.36 + self.neck_length * if cat { 0.75 } else { 0.48 },
        )
    }
    #[must_use]
    pub fn eye_center(&self, side: f32) -> Vec3 {
        let skull = self.head_length - self.muzzle_length;
        let cat = self.breed.species() == Species::Cat;
        let provisional=self.head_origin()
            + Vec3::new(
                side * self.skull_width * if cat { 0.29 } else { 0.32 },
                skull * 0.08,
                skull * if cat { 0.87 } else { 0.85 },
            );
        sculpt::eye_anchor(*self,provisional,self.eye_normal(side))
    }
    #[must_use]
    pub fn eye_normal(&self, side: f32) -> Vec3 {
        if self.breed.species() == Species::Cat {
            Vec3::new(side * 0.35, 0.07, 0.94).normalize()
        } else {
            Vec3::new(side * 0.68, 0.07, 0.73).normalize()
        }
    }
    #[must_use]
    pub fn eye_radius(&self) -> f32 {
        self.skull_width * if self.breed.species() == Species::Cat { 0.09 } else { 0.072 }
    }
    #[must_use]
    pub fn digit_tip(&self,side:f32,front:bool,digit:usize) -> Vec3 {
        let paw=self.paw_half_width;
        let length=if digit==1 || digit==2 {1.45}else{1.30};
        Vec3::new(side*self.rib_half_width*0.68+(digit as f32-1.5)*paw*0.42,
            paw*0.48,(if front {1.}else{-1.})*self.trunk_length*0.36+paw*length)
    }
    #[must_use]
    pub fn nose_center(&self) -> Vec3 {
        let skull = self.head_length - self.muzzle_length;
        self.head_origin()
            + Vec3::new(
                0.0,
                -skull
                    * if self.breed.species() == Species::Cat {
                        0.25
                    } else {
                        0.22
                    },
                self.head_length + 0.001,
            )
    }
    pub fn validate(&self) -> Result<(), String> {
        let dimensions = [
            self.shoulder_height,
            self.trunk_length,
            self.rib_half_width,
            self.rib_depth,
            self.rump_height,
            self.skull_width,
            self.head_length,
            self.muzzle_length,
            self.tail_length,
            self.paw_half_width,
            self.neck_length,
        ];
        if dimensions
            .into_iter()
            .any(|v| !v.is_finite() || v <= 0.0 || v > 2.0)
        {
            return Err("invalid anatomy dimension".into());
        }
        if self.muzzle_length >= self.head_length
            || self.rib_depth >= self.shoulder_height
            || self.rib_half_width * 2.0 >= self.trunk_length
        {
            return Err("inconsistent anatomy proportions".into());
        }
        let ear = match self.ears {
            Ear::Folded { length } => length,
            Ear::Upright { height } => height,
        };
        if !ear.is_finite() || ear <= 0.0 || ear >= self.shoulder_height * 0.4 {
            return Err("invalid ear dimension".into());
        }
        Ok(())
    }
    #[must_use]
    pub fn landmarks(&self) -> Vec<Landmark> {
        let front = self.trunk_length * 0.36;
        let back = -self.trunk_length * 0.36;
        let h = self.shoulder_height;
        vec![
            Landmark {
                name: "withers",
                position: Vec3::new(0.0, h, front),
            },
            Landmark {
                name: "croup",
                position: Vec3::new(0.0, self.rump_height, back),
            },
            Landmark {
                name: "sternum",
                position: Vec3::new(0.0, h - self.rib_depth, front * 0.72),
            },
            Landmark {
                name: "nose",
                position: self.nose_center(),
            },
            Landmark {
                name: "left-front-paw",
                position: Vec3::new(-self.rib_half_width * 0.68, 0.0, front),
            },
            Landmark {
                name: "right-front-paw",
                position: Vec3::new(self.rib_half_width * 0.68, 0.0, front),
            },
            Landmark {
                name: "left-hind-paw",
                position: Vec3::new(-self.rib_half_width * 0.68, 0.0, back),
            },
            Landmark {
                name: "right-hind-paw",
                position: Vec3::new(self.rib_half_width * 0.68, 0.0, back),
            },
        ]
    }
}
/// Build an untextured static anatomical study, without fur, pose tricks or animation.
/// # Errors
/// Rejects invalid anatomy and nonfinite generated surfaces.
pub fn build_study(anatomy: BreedAnatomy) -> Result<AnatomySurface, String> {
    anatomy.validate()?;
    surface::build(anatomy)
}
/// Export one original neutral study as a self-contained GLB for the native viewport.
/// # Errors
/// Rejects invalid anatomy/geometry and encoding overflow.
pub fn study_glb(breed: Breed) -> Result<Vec<u8>, String> {
    glb::encode(&[breed], false)
}
/// Six real-scale studies, in two labelled rows in metadata. Body shape is independent
/// of coat colour: all studies use exactly the same neutral material.
/// # Errors
/// Rejects geometry or encoding overflow.
pub fn gallery_glb() -> Result<Vec<u8>, String> {
    glb::encode(&Breed::ALL, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_breed_is_a_distinct_importable_neutral_surface() {
        let mut hashes = std::collections::HashSet::new();
        for breed in Breed::ALL {
            let bytes = study_glb(breed).unwrap();
            let model =
                crate::ModelAsset::parse(&bytes, &[], crate::ModelLimits::default()).unwrap();
            assert_eq!(model.primitives.len(), 1);
            assert!(model.animations.is_empty());
            let surface = build_study(breed.anatomy()).unwrap();
            assert!(surface.positions.len() > 3000);
            assert!(
                surface
                    .indices
                    .iter()
                    .all(|i| (*i as usize) < surface.positions.len())
            );
            assert!(
                surface
                    .normals
                    .iter()
                    .all(|n| (Vec3::from(*n).length() - 1.0).abs() < 0.001)
            );

            assert!(
                surface
                    .positions
                    .iter()
                    .all(|p| p.iter().all(|v| v.is_finite()))
            );
            let signature = surface
                .positions
                .iter()
                .flatten()
                .fold(0_u64, |h, x| h.rotate_left(5) ^ u64::from(x.to_bits()));
            assert!(
                hashes.insert(signature),
                "breed geometry must not be a recolour"
            );
            assert_eq!(bytes, study_glb(breed).unwrap());
        }
        let gallery = gallery_glb().unwrap();
        let model = crate::ModelAsset::parse(&gallery, &[], crate::ModelLimits::default()).unwrap();
        assert_eq!(model.primitives.len(), 6);
    }
    #[test]
    fn face_landmarks_keep_the_feline_muzzle_short() {
        for breed in Breed::ALL {
            let a = breed.anatomy();
            let eye = a.eye_center(1.0);
            let nose = a.nose_center();
            let forward = nose.z - eye.z;
            assert!(forward > 0.0);
            if breed.species() == Species::Cat {
                assert!(forward < a.skull_width * 0.55);
            }
            assert!((eye.x + a.eye_center(-1.0).x).abs() < 1e-7);
        }
    }
    #[test]
    fn invalid_profiles_fail_before_mesh_generation() {
        let mut profile = Breed::Labrador.anatomy();
        profile.shoulder_height = f32::NAN;
        assert!(build_study(profile).is_err());
        let mut profile = Breed::MaineCoon.anatomy();
        profile.muzzle_length = profile.head_length;
        assert!(build_study(profile).is_err());
        assert!(Breed::parse("unknown").is_err());
    }
}

/// Unified, blended anatomical surface for native companion rendering.
/// # Errors
/// Rejects invalid profiles or a surface exceeding its bounded grid budget.
pub fn build_sculpt(anatomy: BreedAnatomy) -> Result<AnatomySurface, String> {
    anatomy.validate()?;
    sculpt::build(anatomy)
}
