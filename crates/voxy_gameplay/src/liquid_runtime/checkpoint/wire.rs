//! Bounded exact scene-liquid file payload and BLAKE3 corruption detection.
use super::*;
use crate::convex::AffineBox;
use crate::{
    BoxCollider, LiquidBody, LiquidMassDistribution, LiquidMassPart, LiquidPulse, RigidPointForce,
    RigidPointLoads,
};
use physics::{
    astrophysics_spin::Spin, contact::ContactBody, gravity::Body, liquid::PulsedEmitterCheckpoint,
    mass_properties::MassProperties,
};
use voxy_scene::Transform;
type Error = GameplayError;
fn invalid() -> Error {
    "invalid or over-budget scene liquid archive".into()
}
const MAGIC: &[u8; 8] = b"VOXYSL01";
const HEADER: usize = 48;
/// Outer record budgets plus a separate bounded native-liquid decoder budget.
/// Allocator overhead and physical preparation are outside these payload budgets.
#[derive(Clone, Copy, Debug)]
pub struct SceneLiquidWireLimits {
    pub max_bytes: usize,
    pub max_allocation_bytes: usize,
    pub max_values: usize,
    pub liquid: physics::liquid::LiquidWireLimits,
}
struct Reader<'a> {
    bytes: &'a [u8],
    allocation: usize,
    values: usize,
    liquid_limits: physics::liquid::LiquidWireLimits,
}
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], Error> {
        if n > self.bytes.len() {
            return Err(invalid());
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }
    fn charge(&mut self, bytes: usize, values: usize) -> Result<(), Error> {
        self.allocation = self.allocation.checked_sub(bytes).ok_or(invalid())?;
        self.values = self.values.checked_sub(values).ok_or(invalid())?;
        Ok(())
    }
}
struct Writer {
    bytes: Vec<u8>,
    remaining: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.remaining = self.remaining.checked_sub(bytes.len()).ok_or(invalid())?;
        self.bytes.try_reserve(bytes.len()).map_err(|_| invalid())?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}
trait Wire: Sized {
    const MIN: usize;
    fn write(&self, w: &mut Writer) -> Result<(), Error>;
    fn read(r: &mut Reader<'_>) -> Result<Self, Error>;
}
macro_rules! integer {
    ($t:ty, $n:expr) => {
        impl Wire for $t {
            const MIN: usize = $n;
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                w.put(&self.to_le_bytes())
            }
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                Ok(Self::from_le_bytes(
                    r.take($n)?.try_into().map_err(|_| invalid())?,
                ))
            }
        }
    };
}
integer!(u64, 8);
integer!(u32, 4);
integer!(u8, 1);
impl Wire for usize {
    const MIN: usize = 8;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        u64::try_from(*self).map_err(|_| invalid())?.write(w)
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Self::try_from(u64::read(r)?).map_err(|_| invalid())
    }
}
impl Wire for bool {
    const MIN: usize = 1;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        u8::from(*self).write(w)
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        match u8::read(r)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid()),
        }
    }
}
impl<T: Wire> Wire for Option<T> {
    const MIN: usize = 1;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.is_some().write(w)?;
        if let Some(v) = self {
            v.write(w)?;
        }
        Ok(())
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        if bool::read(r)? {
            Ok(Some(T::read(r)?))
        } else {
            Ok(None)
        }
    }
}
impl<T: Wire> Wire for Vec<T> {
    const MIN: usize = 8;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.len().write(w)?;
        for v in self {
            v.write(w)?;
        }
        Ok(())
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = usize::read(r)?;
        if n > r.bytes.len() / T::MIN.max(1) {
            return Err(invalid());
        }
        r.charge(n.checked_mul(std::mem::size_of::<T>()).ok_or(invalid())?, n)?;
        let mut values = Self::new();
        values.try_reserve_exact(n).map_err(|_| invalid())?;
        for _ in 0..n {
            values.push(T::read(r)?);
        }
        Ok(values)
    }
}
impl<T: Wire, const N: usize> Wire for [T; N] {
    const MIN: usize = T::MIN * N;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        for v in self {
            v.write(w)?;
        }
        Ok(())
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        // Fixed arrays use bounded stack storage; no attacker-controlled array length.
        let mut values = [const { None }; N];
        for v in &mut values {
            *v = Some(T::read(r)?);
        }
        Ok(values.map(|v| v.expect("all array entries decoded")))
    }
}
impl<A: Wire, B: Wire> Wire for (A, B) {
    const MIN: usize = A::MIN + B::MIN;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.0.write(w)?;
        self.1.write(w)
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok((A::read(r)?, B::read(r)?))
    }
}
impl Wire for String {
    const MIN: usize = 8;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.len().write(w)?;
        w.put(self.as_bytes())
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let n = usize::read(r)?;
        r.charge(n, n)?;
        let text = std::str::from_utf8(r.take(n)?).map_err(|_| invalid())?;
        let mut result = Self::new();
        result.try_reserve_exact(n).map_err(|_| invalid())?;
        result.push_str(text);
        Ok(result)
    }
}
macro_rules! record {
    ($t:ident { $($f:ident : $ty:ty),* $(,)? }) => { impl Wire for $t {
        const MIN: usize = 0 $(+ <$ty as Wire>::MIN)*;
        fn write(&self, w: &mut Writer) -> Result<(), Error> {
            let Self { $($f),* } = self; $($f.write(w)?;)* Ok(())
        }
        fn read(r: &mut Reader<'_>) -> Result<Self, Error> { Ok(Self { $($f: <$ty as Wire>::read(r)?),* }) }
    }};
}

impl<A: Wire, B: Wire, C: Wire, D: Wire> Wire for (A, B, C, D) {
    const MIN: usize = A::MIN + B::MIN + C::MIN + D::MIN;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.0.write(w)?;
        self.1.write(w)?;
        self.2.write(w)?;
        self.3.write(w)
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok((A::read(r)?, B::read(r)?, C::read(r)?, D::read(r)?))
    }
}
macro_rules! float {
    ($t:ty,$bits:ty) => {
        impl Wire for $t {
            const MIN: usize = <$bits as Wire>::MIN;
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                self.to_bits().write(w)
            }
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                let value = Self::from_bits(<$bits>::read(r)?);
                if !value.is_finite() {
                    return Err(invalid());
                }
                Ok(value)
            }
        }
    };
}
float!(f64, u64);
float!(f32, u32);
macro_rules! vector {
    ($t:ty,$element:ty,$n:expr) => {
        impl Wire for $t {
            const MIN: usize = <$element as Wire>::MIN * $n;
            fn write(&self, w: &mut Writer) -> Result<(), Error> {
                self.to_array().write(w)
            }
            fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
                Ok(Self::from_array(<[$element; $n]>::read(r)?))
            }
        }
    };
}
vector!(glam::DVec3, f64, 3);
vector!(glam::Vec3, f32, 3);
vector!(glam::Quat, f32, 4);
impl Wire for ObjectId {
    const MIN: usize = 8;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.0.write(w)
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let id = String::read(r)?;
        if id.is_empty() {
            return Err(invalid());
        }
        Ok(Self(id))
    }
}
impl Wire for crate::RigidBodyFrame {
    const MIN: usize = 27 * 8 + 3 * 4;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        let (values, scale) = self.checkpoint_words();
        values.write(w)?;
        scale.write(w)
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Self::from_checkpoint_words(<[u64; 27]>::read(r)?, <[u32; 3]>::read(r)?)
            .map_err(|e| format!("rigid frame checkpoint: {e:?}").into())
    }
}
impl Wire for physics::liquid::LiquidCheckpoint {
    const MIN: usize = 8;
    fn write(&self, w: &mut Writer) -> Result<(), Error> {
        self.to_bytes(w.remaining).map_err(|_| invalid())?.write(w)
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        let bytes = Vec::<u8>::read(r)?;
        Self::from_bytes(&bytes, r.liquid_limits).map_err(|_| invalid())
    }
}

record!(SceneLiquidCheckpoint {version:u32,liquid:physics::liquid::LiquidCheckpoint,sources:Vec<(ObjectId,LiquidSource,physics::liquid::PulsedEmitterCheckpoint,[u64;3])>,bodies:Vec<BodyCheckpoint>});
record!(ColliderCheckpoint {node:ObjectId,collider:crate::BoxCollider,path:Vec<(ObjectId,voxy_scene::Transform)>});
record!(BodyCheckpoint {node:ObjectId,descriptor:crate::LiquidBody,state:physics::contact::ContactBody,principal_templates:Option<Vec<crate::convex::AffineBox>>,published:voxy_scene::Transform,colliders:Vec<ColliderCheckpoint>,mass_descriptor:Option<crate::LiquidMassDistribution>,mass_properties:Option<physics::mass_properties::MassProperties>,rigid_frame:Option<crate::RigidBodyFrame>,point_loads:Option<crate::RigidPointLoads>,point_elapsed_s:u64});
record!(LiquidSource {pulses:Vec<crate::LiquidPulse>,density_kg_m3:f64,particle_volume_m3:f64,nozzle_radius_m:f64,direction:[f64;3],material_asset:String});
record!(LiquidPulse {
    start_s: f64,
    duration_s: f64,
    volume_m3: f64,
    speed_m_s: f64
});
record!(LiquidBody {
    mass_kg: f64,
    initial_velocity_m_s: [f64; 3]
});
record!(LiquidMassDistribution {parts:Vec<crate::LiquidMassPart>});
record!(LiquidMassPart {
    mass_kg: f64,
    center_m: [f64; 3],
    half_edges_m: [[f64; 3]; 3]
});
record!(BoxCollider {
    half_extents: [f32; 3]
});
record!(RigidPointLoads {points:Vec<crate::RigidPointForce>});
record!(RigidPointForce {
    root_point_m: [f64; 3],
    world_force_n: [f64; 3],
    world_force_rate_n_s: [f64; 3]
});
record!(Transform {
    translation: glam::Vec3,
    rotation: glam::Quat,
    scale: glam::Vec3
});
record!(AffineBox {
    center: glam::DVec3,
    edges: [glam::DVec3; 3]
});
record!(ContactBody {motion:physics::gravity::Body,spin:Option<physics::astrophysics_spin::Spin>});
record!(Body {
    mass: f64,
    position: [f64; 3],
    velocity: [f64; 3]
});
record!(Spin {
    orientation: [f64; 4],
    angular_momentum: [f64; 3],
    inertia: [f64; 3]
});
record!(MassProperties {
    mass: f64,
    center: [f64; 3],
    inertia: [[f64; 3]; 3],
    principal_moments: [f64; 3],
    principal_axes: [[f64; 3]; 3]
});
record!(PulsedEmitterCheckpoint {version:u32,pulses:Vec<[u64;4]>,elapsed:u64,emitted_samples:u32,particle:[u64;7],material:usize,field:Option<[u64;2]>,phase_fraction:Option<u64>,direction:[u64;3],source_velocity:[u64;3],density:u64,particle_volume:u64,nozzle_radius:u64});

impl SceneLiquidCheckpoint {
    /// Writes exact words and a digest. This is corruption detection, not authentication.
    pub fn to_bytes(&self, max_bytes: usize) -> Result<Vec<u8>, GameplayError> {
        let mut w = Writer {
            bytes: Vec::new(),
            remaining: max_bytes,
        };
        w.put(MAGIC)?;
        w.put(&[0; 40])?;
        self.write(&mut w)?;
        let length = u64::try_from(w.bytes.len() - HEADER).map_err(|_| invalid())?;
        w.bytes[8..16].copy_from_slice(&length.to_le_bytes());
        let digest = blake3::hash(&w.bytes[HEADER..]);
        w.bytes[16..48].copy_from_slice(digest.as_bytes());
        Ok(w.bytes)
    }
    /// Checks the entire envelope before allocating decoded records. Call `prepare`
    /// to resolve scene bindings and validate physical state before publication.
    pub fn from_bytes(bytes: &[u8], limits: SceneLiquidWireLimits) -> Result<Self, GameplayError> {
        if bytes.len() < HEADER || bytes.len() > limits.max_bytes || &bytes[..8] != MAGIC {
            return Err(invalid());
        }
        let length = u64::from_le_bytes(bytes[8..16].try_into().map_err(|_| invalid())?);
        if usize::try_from(length).map_err(|_| invalid())? != bytes.len() - HEADER
            || blake3::hash(&bytes[HEADER..]).as_bytes() != &bytes[16..48]
        {
            return Err(invalid());
        }
        let mut r = Reader {
            bytes: &bytes[HEADER..],
            allocation: limits.max_allocation_bytes,
            values: limits.max_values,
            liquid_limits: limits.liquid,
        };
        let saved = Self::read(&mut r)?;
        if !r.bytes.is_empty() || saved.version != 1 {
            return Err(invalid());
        }
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> SceneLiquidWireLimits {
        SceneLiquidWireLimits {
            max_bytes: 1_000_000,
            max_allocation_bytes: 1_000_000,
            max_values: 100_000,
            liquid: physics::liquid::LiquidWireLimits {
                max_bytes: 1_000_000,
                max_allocation_bytes: 1_000_000,
                max_values: 100_000,
            },
        }
    }
    fn sample() -> SceneLiquidCheckpoint {
        let scene = SceneGraph::new(4);
        SceneLiquidRuntime::new(
            &scene,
            vec![("water".into(), Material::WATER)],
            Config::default(),
            4,
        )
        .unwrap()
        .checkpoint(&scene, |_| None)
        .unwrap()
    }
    fn digest(bytes: &mut [u8]) {
        let hash = blake3::hash(&bytes[HEADER..]);
        bytes[16..48].copy_from_slice(hash.as_bytes());
    }
    #[test]
    fn file_rejects_every_truncation_single_byte_corruption_and_resource_limits() {
        let saved = sample();
        let bytes = saved.to_bytes(1_000_000).unwrap();
        assert_eq!(
            SceneLiquidCheckpoint::from_bytes(&bytes, limits()).unwrap(),
            saved
        );
        assert_eq!(saved.to_bytes(bytes.len()).unwrap(), bytes);
        assert!(saved.to_bytes(bytes.len() - 1).is_err());
        for i in 0..bytes.len() {
            assert!(SceneLiquidCheckpoint::from_bytes(&bytes[..i], limits()).is_err());
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert!(
                SceneLiquidCheckpoint::from_bytes(&bad, limits()).is_err(),
                "corruption accepted at {i}"
            );
        }
        let mut bad = bytes.clone();
        bad.push(0);
        assert!(SceneLiquidCheckpoint::from_bytes(&bad, limits()).is_err());
        for policy in [
            SceneLiquidWireLimits {
                max_bytes: bytes.len() - 1,
                ..limits()
            },
            SceneLiquidWireLimits {
                max_allocation_bytes: 0,
                ..limits()
            },
            SceneLiquidWireLimits {
                max_values: 0,
                ..limits()
            },
            SceneLiquidWireLimits {
                liquid: physics::liquid::LiquidWireLimits {
                    max_bytes: 1,
                    ..limits().liquid
                },
                ..limits()
            },
            SceneLiquidWireLimits {
                liquid: physics::liquid::LiquidWireLimits {
                    max_allocation_bytes: 0,
                    ..limits().liquid
                },
                ..limits()
            },
        ] {
            assert!(SceneLiquidCheckpoint::from_bytes(&bytes, policy).is_err());
        }
        // Recomputed digests force structural validation rather than only checksum failure.
        let mut bad = bytes.clone();
        bad[48..52].copy_from_slice(&2_u32.to_le_bytes());
        digest(&mut bad);
        assert!(SceneLiquidCheckpoint::from_bytes(&bad, limits()).is_err());
        let mut bad = bytes.clone();
        bad[52..60].copy_from_slice(&u64::MAX.to_le_bytes());
        digest(&mut bad);
        assert!(SceneLiquidCheckpoint::from_bytes(&bad, limits()).is_err());
    }
    #[test]
    fn exact_scalar_codec_preserves_signed_zero_and_rejects_invalid_tags() {
        let mut w = Writer {
            bytes: Vec::new(),
            remaining: 1000,
        };
        (-0_f64).write(&mut w).unwrap();
        (-0_f32).write(&mut w).unwrap();
        let mut r = Reader {
            bytes: &w.bytes,
            allocation: 1000,
            values: 1000,
            liquid_limits: limits().liquid,
        };
        assert_eq!(f64::read(&mut r).unwrap().to_bits(), (-0_f64).to_bits());
        assert_eq!(f32::read(&mut r).unwrap().to_bits(), (-0_f32).to_bits());
        let mut r = Reader {
            bytes: &[2],
            allocation: 1000,
            values: 1000,
            liquid_limits: limits().liquid,
        };
        assert!(Option::<u64>::read(&mut r).is_err());
        let mut w = Writer {
            bytes: Vec::new(),
            remaining: 1000,
        };
        let nested = vec![Vec::<u64>::new(); 3];
        nested.write(&mut w).unwrap();
        let mut r = Reader {
            bytes: &w.bytes,
            allocation: 1000,
            values: 2,
            liquid_limits: limits().liquid,
        };
        assert!(Vec::<Vec<u64>>::read(&mut r).is_err());
    }
}
