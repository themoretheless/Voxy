//! Exact little-endian native records. Physical admission remains `prepare`.
use super::*;

/// Independent limits on encoded bytes, cumulative heap storage and decoded values.
#[derive(Clone, Copy, Debug)]
pub struct LiquidWireLimits {
    pub max_bytes: usize,
    pub max_allocation_bytes: usize,
    pub max_values: usize,
}
const MAGIC: &[u8; 8] = b"VOXYLQ01";
struct Reader<'a> {
    bytes: &'a [u8],
    allocation: usize,
    values: usize,
}
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], Error> {
        if n > self.bytes.len() {
            return Err(Error::InvalidConfig);
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }
    fn charge(&mut self, bytes: usize, values: usize) -> Result<(), Error> {
        self.allocation = self
            .allocation
            .checked_sub(bytes)
            .ok_or(Error::InvalidConfig)?;
        self.values = self
            .values
            .checked_sub(values)
            .ok_or(Error::InvalidConfig)?;
        Ok(())
    }
}
struct Writer {
    bytes: Vec<u8>,
    remaining: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.remaining = self
            .remaining
            .checked_sub(bytes.len())
            .ok_or(Error::InvalidConfig)?;
        self.bytes
            .try_reserve(bytes.len())
            .map_err(|_| Error::InvalidConfig)?;
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
                    r.take($n)?.try_into().map_err(|_| Error::InvalidConfig)?,
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
        u64::try_from(*self)
            .map_err(|_| Error::InvalidConfig)?
            .write(w)
    }
    fn read(r: &mut Reader<'_>) -> Result<Self, Error> {
        Self::try_from(u64::read(r)?).map_err(|_| Error::InvalidConfig)
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
            _ => Err(Error::InvalidConfig),
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
            return Err(Error::InvalidConfig);
        }
        r.charge(
            n.checked_mul(std::mem::size_of::<T>())
                .ok_or(Error::InvalidConfig)?,
            n,
        )?;
        let mut values = Self::new();
        values
            .try_reserve_exact(n)
            .map_err(|_| Error::InvalidConfig)?;
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
        let text = std::str::from_utf8(r.take(n)?).map_err(|_| Error::InvalidConfig)?;
        let mut result = Self::new();
        result
            .try_reserve_exact(n)
            .map_err(|_| Error::InvalidConfig)?;
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
record!(LiquidCheckpoint {
    version: u32,
    particles: LiquidParticleCheckpoint,
    polymer: PolymerCheckpoint,
    rheology: RheologyCheckpoint,
    thermal: Option<ThermalFieldCheckpoint>,
    phase: Option<PhaseCheckpoint>,
    species: Option<SpeciesCheckpoint>,
    field_response: FieldResponseCheckpoint,
    evolution: LiquidEvolutionCheckpoint,
    surface: SurfaceInteractionCheckpoint,
    boundary: LiquidBoundaryCheckpoint,
    population_heat: PopulationHeatCheckpoint,
});

record!(ParticleCheckpoint {
    values: [u64; 7],
    material: usize,
});

record!(LiquidParticleCheckpoint {
    version: u32,
    particles: Vec<ParticleCheckpoint>,
    materials: Vec<[u64; 3]>,
    configuration: [u64; 5],
    budgets: [usize; 4],
});

record!(PolymerCheckpoint {
    version: u32,
    laws: Vec<Option<[u64; 2]>>,
    conformations: Vec<[[u64; 3]; 3]>,
    particle_materials: Vec<usize>,
    heat: u64,
});

record!(RheologyCheckpoint {
    version: u32,
    shear: Vec<Option<[u64; 5]>>,
    yield_stresses: Vec<u64>,
    thixotropy: Vec<Option<[u64; 4]>>,
    structure: Vec<u64>,
    particle_materials: Vec<usize>,
});

record!(ThermalMaterialCheckpoint {
    values: [u64; 3],
    mixing_group: u32,
});

record!(ThermalFieldCheckpoint {
    version: u32,
    fields: Vec<[u64; 2]>,
    materials: Vec<ThermalMaterialCheckpoint>,
    particle_materials: Vec<usize>,
});

record!(SaturationCheckpoint {
    curves: Vec<Option<[u64; 6]>>,
    pressures: Vec<u64>,
    temperatures: Vec<Option<u64>>,
});

record!(PhaseCheckpoint {
    version: u32,
    models: Vec<Option<[u64; 5]>>,
    fractions: Vec<u64>,
    particle_materials: Vec<usize>,
    saturation: Option<SaturationCheckpoint>,
});

record!(ReactionCheckpoint {
    indices: [usize; 3],
    coefficients: [u64; 2],
    chemical_energies: Vec<u64>,
});

record!(ReactionAccuracyCheckpoint {
    tolerances: [u64; 3],
    max_attempts: usize,
});

record!(SpeciesPropertiesCheckpoint {
    components: Vec<[u64; 3]>,
    viscosity_blend: u8,
});

record!(SpeciesCheckpoint {
    version: u32,
    names: Vec<String>,
    fractions: Vec<Vec<u64>>,
    particle_materials: Vec<usize>,
    reaction: Option<ReactionCheckpoint>,
    kinetics: Option<[u64; 2]>,
    accuracy: Option<ReactionAccuracyCheckpoint>,
    properties: Option<SpeciesPropertiesCheckpoint>,
    heat_capacities: Option<Vec<u64>>,
});

record!(PropertyResponseCheckpoint {
    coefficients: [u64; 3],
    solute: Option<[u64; 3]>,
});

record!(FieldResponseCheckpoint {
    version: u32,
    materials: Vec<[u64; 3]>,
    properties: Vec<Option<PropertyResponseCheckpoint>>,
    arrhenius: Vec<Option<[u64; 2]>>,
});

record!(LiquidEvolutionCheckpoint {
    version: u32,
    materials: Vec<[u64; 3]>,
    formulation: u8,
    gas_equations: Vec<Option<[u64; 2]>>,
    pressure_work: bool,
    viscous_heating: bool,
    viscous_integrator: u8,
    tensile_floor_pa: Option<u64>,
});

record!(SurfaceInteractionCheckpoint {
    version: u32,
    materials: Vec<[u64; 3]>,
    strengths: Vec<u64>,
    interfaces: Vec<([usize; 2], u64)>,
    wall_adhesion: Vec<Option<[u64; 2]>>,
});

record!(LiquidBoundaryCheckpoint {
    version: u32,
    smoothing_radius: u64,
    samples: Vec<[u64; 4]>,
    velocities: Vec<[u64; 3]>,
    static_pressure_extrapolation: bool,
    reflecting_box: Option<[[u64; 3]; 2]>,
    no_slip: bool,
});

record!(PopulationHeatCheckpoint {
    version: u32,
    particle_materials: Vec<usize>,
    population: Option<Vec<bool>>,
    heat: Vec<[u64; 2]>,
});

impl LiquidCheckpoint {
    /// Encodes integer words exactly; rejects output exceeding the caller's byte cap.
    pub fn to_bytes(&self, max_bytes: usize) -> Result<Vec<u8>, Error> {
        let mut writer = Writer {
            bytes: Vec::new(),
            remaining: max_bytes,
        };
        writer.put(MAGIC)?;
        self.write(&mut writer)?;
        Ok(writer.bytes)
    }
    /// Decodes bounded structural records. Call `prepare` for physical admission.
    /// Unknown root versions, invalid tags, truncated or trailing bytes are rejected.
    /// Component versions and physical constraints are checked by `prepare`.
    pub fn from_bytes(bytes: &[u8], limits: LiquidWireLimits) -> Result<Self, Error> {
        if bytes.len() > limits.max_bytes {
            return Err(Error::InvalidConfig);
        }
        let mut reader = Reader {
            bytes,
            allocation: limits.max_allocation_bytes,
            values: limits.max_values,
        };
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(Error::InvalidConfig);
        }
        let result = Self::read(&mut reader)?;
        if !reader.bytes.is_empty() || result.version != 1 {
            return Err(Error::InvalidConfig);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> LiquidWireLimits {
        LiquidWireLimits {
            max_bytes: 1_000_000,
            max_allocation_bytes: 1_000_000,
            max_values: 100_000,
        }
    }
    fn sample() -> LiquidCheckpoint {
        Liquid::new(
            vec![Particle {
                position: [-0., 0., 0.],
                velocity: [0.; 3],
                mass: 1.,
                material: 0,
            }],
            vec![Material::WATER],
            Config::default(),
        )
        .unwrap()
        .checkpoint()
        .unwrap()
    }
    #[test]
    fn exact_record_rejects_every_truncation_trailing_data_and_limits() {
        let saved = sample();
        let bytes = saved.to_bytes(1_000_000).unwrap();
        let path = std::env::temp_dir().join(format!(
            "voxy-liquid-wire-{}-{}.bin",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, &bytes).unwrap();
        let disk_bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(disk_bytes, bytes);
        assert_eq!(
            LiquidCheckpoint::from_bytes(&bytes, limits()).unwrap(),
            saved
        );
        assert!(saved.to_bytes(bytes.len() - 1).is_err());
        assert_eq!(saved.to_bytes(bytes.len()).unwrap(), bytes);
        for n in 0..bytes.len() {
            assert!(
                LiquidCheckpoint::from_bytes(&bytes[..n], limits()).is_err(),
                "accepted prefix {n}"
            );
        }
        let mut bad = bytes.clone();
        bad.push(0);
        assert!(LiquidCheckpoint::from_bytes(&bad, limits()).is_err());
        bad = bytes.clone();
        bad[0] ^= 1;
        assert!(LiquidCheckpoint::from_bytes(&bad, limits()).is_err());
        bad = bytes.clone();
        bad[8..12].copy_from_slice(&2_u32.to_le_bytes());
        assert!(LiquidCheckpoint::from_bytes(&bad, limits()).is_err());
        bad = bytes.clone();
        bad[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(LiquidCheckpoint::from_bytes(&bad, limits()).is_err());
        for policy in [
            LiquidWireLimits {
                max_bytes: bytes.len() - 1,
                ..limits()
            },
            LiquidWireLimits {
                max_allocation_bytes: 0,
                ..limits()
            },
            LiquidWireLimits {
                max_values: 0,
                ..limits()
            },
        ] {
            assert!(LiquidCheckpoint::from_bytes(&bytes, policy).is_err());
        }
        assert_eq!(saved.particles.particles[0].values[0], (-0_f64).to_bits());
    }
    #[test]
    fn nested_allocations_and_strings_share_global_budgets() {
        let nested = vec![vec![], vec![], vec![]] as Vec<Vec<u64>>;
        let mut writer = Writer {
            bytes: Vec::new(),
            remaining: 1000,
        };
        nested.write(&mut writer).unwrap();
        let mut reader = Reader {
            bytes: &writer.bytes,
            allocation: 1000,
            values: 2,
        };
        assert!(Vec::<Vec<u64>>::read(&mut reader).is_err());
        let mut reader = Reader {
            bytes: &writer.bytes,
            allocation: 3 * std::mem::size_of::<Vec<u64>>() - 1,
            values: 100,
        };
        assert!(Vec::<Vec<u64>>::read(&mut reader).is_err());
        let mut writer = Writer {
            bytes: Vec::new(),
            remaining: 1000,
        };
        vec!["живое".to_string(), "state".to_string()]
            .write(&mut writer)
            .unwrap();
        let mut reader = Reader {
            bytes: &writer.bytes,
            allocation: 1000,
            values: 1000,
        };
        assert_eq!(
            Vec::<String>::read(&mut reader).unwrap(),
            ["живое", "state"]
        );
        let mut reader = Reader {
            bytes: &[2],
            allocation: 1000,
            values: 1000,
        };
        assert!(bool::read(&mut reader).is_err());
        let mut reader = Reader {
            bytes: &[2],
            allocation: 1000,
            values: 1000,
        };
        assert!(Option::<u64>::read(&mut reader).is_err());
        let mut invalid = 1_u64.to_le_bytes().to_vec();
        invalid.push(255);
        let mut reader = Reader {
            bytes: &invalid,
            allocation: 1000,
            values: 1000,
        };
        assert!(String::read(&mut reader).is_err());
    }
}
