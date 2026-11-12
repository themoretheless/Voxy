//! GPU projection of rendered cube faces into order-1 SH irradiance probes.

/// Probe capacity of the archive scene's preallocated SH buffer.
pub(crate) const MAX_PROBES: usize = 4096;
/// Bytes per probe: four RGB(A) SH coefficients.
pub(crate) const PROBE_BYTES: u64 = 4 * 16;

/// Versioned, CRC-32C-checked binary cache (`.vprobes`) of baked order-1 SH irradiance probes
/// (`ProbeGrid` + 64-byte per-probe `[f32; 16]` coefficients). Eliminates multi-minute cold
/// startup baking on scenes such as `street_walk` when the grid and probe count match.
#[derive(Clone, Debug, PartialEq)]
pub struct BakedProbeCache {
    pub origin: [f32; 4],
    pub spacing: [f32; 4],
    pub dims: [u32; 4],
    /// Each probe holds 4 `vec4<f32>` order-1 SH coefficients (`[f32; 16]`, 64 bytes).
    pub probes: Vec<[f32; 16]>,
}

impl BakedProbeCache {
    pub const ENVELOPE: voxy_core::CheckpointEnvelope =
        voxy_core::CheckpointEnvelope::new(*b"VPRB", 1);
    pub const MAX_PAYLOAD_BYTES: usize = 64 + MAX_PROBES * 64;

    /// Constructs a validated baked probe cache.
    /// # Errors
    /// Rejects non-finite origin/spacing/coefficients, zero dimensions, or probe count mismatch.
    pub fn new(
        origin: [f32; 4],
        spacing: [f32; 4],
        dims: [u32; 4],
        probes: Vec<[f32; 16]>,
    ) -> Result<Self, voxy_core::CodecError> {
        let count = (dims[0] as usize)
            .checked_mul(dims[1] as usize)
            .and_then(|xy| xy.checked_mul(dims[2] as usize))
            .ok_or(voxy_core::CodecError::InvalidValue("probe grid overflow"))?;
        if count == 0 || count > MAX_PROBES {
            return Err(voxy_core::CodecError::CapacityExceeded {
                count,
                limit: MAX_PROBES,
            });
        }
        if probes.len() != count {
            return Err(voxy_core::CodecError::InvalidValue(
                "probe count does not match grid dims",
            ));
        }
        if origin.iter().chain(spacing.iter()).any(|v| !v.is_finite())
            || spacing[..3].iter().any(|s| *s <= 0.0)
            || probes.iter().flatten().any(|c| !c.is_finite())
        {
            return Err(voxy_core::CodecError::InvalidValue(
                "non-finite or non-positive probe cache values",
            ));
        }
        Ok(Self {
            origin,
            spacing,
            dims,
            probes,
        })
    }

    /// Encodes the probe cache into a `VPRB` v1 `CheckpointEnvelope` byte vector.
    /// # Errors
    /// Propagates any [`voxy_core::CodecError`].
    pub fn to_bytes(&self) -> Result<Vec<u8>, voxy_core::CodecError> {
        Self::ENVELOPE.seal(self)
    }

    /// Decodes and validates a `VPRB` v1 `CheckpointEnvelope` byte slice.
    /// # Errors
    /// Rejects corrupt magic/version/checksum, non-finite values, or mismatched dimensions.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, voxy_core::CodecError> {
        Self::ENVELOPE.open(bytes, Self::MAX_PAYLOAD_BYTES)
    }

    /// Writes the encoded cache to `path` through a sibling temporary file and a rename, so a
    /// crash never leaves a truncated `.vprobes` in place.
    /// # Errors
    /// Returns encoding failures as [`std::io::ErrorKind::InvalidData`] and any I/O error.
    pub fn save_to_path(&self, path: &std::path::Path) -> std::io::Result<()> {
        let bytes = self
            .to_bytes()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e:?}")))?;
        let mut temporary = path.as_os_str().to_owned();
        temporary.push(".tmp");
        let temporary = std::path::PathBuf::from(temporary);
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(&temporary, path).inspect_err(|_| {
            let _ = std::fs::remove_file(&temporary);
        })
    }

    /// Reads and validates a cache written by [`Self::save_to_path`].
    /// # Errors
    /// Returns any I/O error; a file larger than the envelope limit or failing validation is
    /// reported as [`std::io::ErrorKind::InvalidData`].
    pub fn load_from_path(path: &std::path::Path) -> std::io::Result<Self> {
        // Header and checksum overhead of the envelope is far below this slack.
        let limit = Self::MAX_PAYLOAD_BYTES as u64 + 64;
        if std::fs::metadata(path)?.len() > limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "probe cache file exceeds the envelope limit",
            ));
        }
        Self::from_bytes(&std::fs::read(path)?)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e:?}")))
    }

    /// Uploads the cached SH coefficients directly into the GPU probe storage buffer.
    pub fn upload_to_buffer(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.probes));
    }
}

impl voxy_core::BinaryCheckpoint for BakedProbeCache {
    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), voxy_core::CodecError> {
        let mut writer = voxy_core::ByteWriter::new(out);
        for v in self.origin {
            if !v.is_finite() {
                return Err(voxy_core::CodecError::InvalidValue("non-finite origin"));
            }
            writer.write_f32_le(v);
        }
        for v in self.spacing {
            if !v.is_finite() {
                return Err(voxy_core::CodecError::InvalidValue("non-finite spacing"));
            }
            writer.write_f32_le(v);
        }
        for d in self.dims {
            writer.write_u32_le(d);
        }
        writer.write_u32_le(self.probes.len() as u32);
        for probe in &self.probes {
            for coeff in *probe {
                if !coeff.is_finite() {
                    return Err(voxy_core::CodecError::InvalidValue("non-finite SH coeff"));
                }
                writer.write_f32_le(coeff);
            }
        }
        Ok(())
    }

    fn decode_from(bytes: &[u8]) -> Result<Self, voxy_core::CodecError> {
        let mut reader = voxy_core::ByteReader::new(bytes);
        let origin = [
            reader.read_finite_f32_le()?,
            reader.read_finite_f32_le()?,
            reader.read_finite_f32_le()?,
            reader.read_finite_f32_le()?,
        ];
        let spacing = [
            reader.read_finite_f32_le()?,
            reader.read_finite_f32_le()?,
            reader.read_finite_f32_le()?,
            reader.read_finite_f32_le()?,
        ];
        let dims = [
            reader.read_u32_le()?,
            reader.read_u32_le()?,
            reader.read_u32_le()?,
            reader.read_u32_le()?,
        ];
        let count = reader.read_u32_le()? as usize;
        if count > MAX_PROBES {
            return Err(voxy_core::CodecError::CapacityExceeded {
                count,
                limit: MAX_PROBES,
            });
        }
        let mut probes = Vec::with_capacity(count);
        for _ in 0..count {
            let mut probe = [0.0_f32; 16];
            for coeff in &mut probe {
                *coeff = reader.read_finite_f32_le()?;
            }
            probes.push(probe);
        }
        reader.finish()?;
        Self::new(origin, spacing, dims, probes)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Bake {
    forward: [f32; 4],
    right: [f32; 4],
    up: [f32; 4],
    square: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ProbeGrid {
    pub origin: [f32; 4],
    pub spacing: [f32; 4],
    pub dims: [u32; 4],
}

#[derive(Debug)]
pub(crate) struct ProbeProjector {
    pipeline: wgpu::ComputePipeline,
    uniform: wgpu::Buffer,
    trace: Option<ProjectionTrace>,
}

#[path = "probe_projection_trace.rs"]
mod projection_trace;
use projection_trace::ProjectionTrace;

impl ProbeProjector {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("irradiance probe projection"),
            source: wgpu::ShaderSource::Wgsl(include_str!("probe_gi.wgsl").into()),
        });
        Self {
            trace: None,
            pipeline: device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("irradiance probe projection"),
                layout: None,
                module: &module,
                entry_point: Some("cs_project"),
                compilation_options: Default::default(),
                cache: None,
            }),
            uniform: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("irradiance probe face"),
                size: std::mem::size_of::<Bake>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    pub(crate) fn enable_trace(
        &mut self,
        device: &wgpu::Device,
        count: usize,
        target: usize,
    ) -> Result<(), crate::ComputeError> {
        let trace = ProjectionTrace::new(device, count, target)?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("probe projection operand trace"),
            source: wgpu::ShaderSource::Wgsl(trace.shader.clone().into()),
        });
        self.pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("probe projection operand trace"),
            layout: None,
            module: &module,
            entry_point: Some("cs_project"),
            compilation_options: Default::default(),
            cache: None,
        });
        self.trace = Some(trace);
        Ok(())
    }

    pub(crate) fn save_trace(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), crate::ComputeError> {
        if let Some(trace) = &self.trace {
            trace.save(device, queue)?;
        }
        Ok(())
    }

    pub(crate) fn diagnostic_uniform(&self) -> &wgpu::Buffer {
        &self.uniform
    }

    /// Accumulate one rendered face (90-degree vertical FOV, centred square of
    /// side `height`) into probe `index`. Face 0 resets the slot.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn encode_projection(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        face_view: &wgpu::TextureView,
        (width, height): (u32, u32),
        probes: &wgpu::Buffer,
        index: usize,
        face: usize,
        basis: [glam::Vec3; 3],
        inverse_exposure: f32,
    ) {
        let [forward, right, up] = basis;
        let size = height.min(width) as f32;
        let bake = Bake {
            forward: [forward.x, forward.y, forward.z, index as f32],
            right: [right.x, right.y, right.z, face as f32],
            up: [up.x, up.y, up.z, inverse_exposure],
            square: [
                ((width as f32 - size) * 0.5).floor(),
                size,
                (size / 128.).max(1.).floor(),
                0.,
            ],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&bake));
        if let Some(trace) = &self.trace {
            trace.validate_face(index, face, size, bake.square[2]);
        }
        let entries = [
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(face_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: probes.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: self.trace.as_ref().map_or_else(
                    || probes.as_entire_binding(),
                    |trace| trace.buffer.as_entire_binding(),
                ),
            },
        ];
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("irradiance probe face"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries[..if self.trace.is_some() { 4 } else { 3 }],
        });
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}

/// Grid of probe positions covering `min..=max` at roughly `spacing`.
/// Every axis has at least two probes so trilinear lookup is always defined.
pub(crate) fn grid(
    min: glam::Vec3,
    max: glam::Vec3,
    spacing: f32,
) -> Option<(ProbeGrid, Vec<glam::Vec3>)> {
    if !(min.is_finite() && max.is_finite() && spacing.is_finite() && spacing > 0.)
        || max.cmple(min).any()
    {
        return None;
    }
    let extent = max - min;
    let dims = (extent / spacing).ceil().as_uvec3().max(glam::UVec3::ONE) + 1;
    if (dims.x * dims.y * dims.z) as usize > MAX_PROBES {
        return None;
    }
    let step = extent / (dims - 1).as_vec3();
    let mut points = Vec::with_capacity((dims.x * dims.y * dims.z) as usize);
    for z in 0..dims.z {
        for y in 0..dims.y {
            for x in 0..dims.x {
                points.push(min + step * glam::UVec3::new(x, y, z).as_vec3());
            }
        }
    }
    Some((
        ProbeGrid {
            origin: [min.x, min.y, min.z, 1.],
            spacing: [step.x, step.y, step.z, 0.],
            dims: [dims.x, dims.y, dims.z, 0],
        },
        points,
    ))
}

/// Evaluates the one-tailed Chebyshev upper-bound visibility weight in `[0.0, 1.0]` for an
/// irradiance probe at distance `distance` from a shaded point, given the probe's radial depth
/// moments `(mean_depth, mean_squared_depth)`. Prevents light leaking through thin walls.
#[must_use]
pub fn chebyshev_probe_visibility(
    distance: f32,
    mean_depth: f32,
    mean_squared_depth: f32,
    min_variance: f32,
) -> f32 {
    if !distance.is_finite()
        || !mean_depth.is_finite()
        || !mean_squared_depth.is_finite()
        || !min_variance.is_finite()
        || distance < 0.0
    {
        return 0.0;
    }
    if distance <= mean_depth {
        return 1.0;
    }
    let variance = (mean_squared_depth - mean_depth * mean_depth).max(min_variance.max(1e-5));
    let diff = distance - mean_depth;
    let p_max = variance / (variance + diff * diff);
    // Cube-sharpen the tail to crush low-probability light leaks:
    (p_max * p_max * p_max).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn bake_pod_layout_matches_wgsl_struct_definition() {
        let module =
            naga::front::wgsl::parse_str(include_str!("probe_gi.wgsl")).expect("valid WGSL");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        );
        validator.validate(&module).expect("naga validation passes");
        let (_, bake_ty) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some("Bake"))
            .expect("Bake struct exists in probe_gi.wgsl");
        let naga::TypeInner::Struct { members, span } = &bake_ty.inner else {
            panic!("expected WGSL struct");
        };
        assert_eq!(*span as usize, std::mem::size_of::<Bake>());
        assert_eq!(
            members[0].offset as usize,
            std::mem::offset_of!(Bake, forward)
        );
        assert_eq!(
            members[1].offset as usize,
            std::mem::offset_of!(Bake, right)
        );
        assert_eq!(members[2].offset as usize, std::mem::offset_of!(Bake, up));
        assert_eq!(
            members[3].offset as usize,
            std::mem::offset_of!(Bake, square)
        );
    }

    #[test]
    fn grid_covers_bounds_with_at_least_two_probes_per_axis() {
        let (g, points) = grid(Vec3::ZERO, Vec3::new(10., 0.5, 30.), 4.).unwrap();
        assert_eq!(g.dims, [4, 2, 9, 0]);
        assert_eq!(points.len(), 4 * 2 * 9);
        assert_eq!(points[0], Vec3::ZERO);
        assert!((*points.last().unwrap() - Vec3::new(10., 0.5, 30.)).length() < 1e-4);
    }

    #[test]
    fn grid_rejects_degenerate_or_oversized_requests() {
        assert!(grid(Vec3::ZERO, Vec3::ZERO, 1.).is_none());
        assert!(grid(Vec3::ZERO, Vec3::ONE, 0.).is_none());
        assert!(grid(Vec3::ZERO, Vec3::splat(1000.), 1.).is_none());
        assert_eq!(chebyshev_probe_visibility(1.5, 2.0, 4.1, 1e-3), 1.0);
        assert!(chebyshev_probe_visibility(4.0, 1.5, 2.3, 1e-3) < 0.02);
    }

    #[test]
    fn baked_probe_cache_roundtrips_and_rejects_corrupted_or_mismatched_grids() {
        let probes = vec![[0.25_f32; 16]; 8];
        let cache = BakedProbeCache::new(
            [0.0, 1.0, 2.0, 1.0],
            [4.0, 4.0, 4.0, 0.0],
            [2, 2, 2, 0],
            probes,
        )
        .unwrap();
        let mut encoded = cache.to_bytes().unwrap();
        assert_eq!(&encoded[0..4], b"VPRB");
        let decoded = BakedProbeCache::from_bytes(&encoded).unwrap();
        assert_eq!(decoded, cache);

        // Flipping a payload byte triggers CRC-32C rejection:
        encoded[24] ^= 0x01;
        assert!(matches!(
            BakedProbeCache::from_bytes(&encoded),
            Err(voxy_core::CodecError::ChecksumMismatch { .. })
        ));

        // File round trip leaves no temporary behind and rejects a corrupted file:
        let dir = std::env::temp_dir().join(format!("voxy-vprobes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("scene.vprobes");
        cache.save_to_path(&path).unwrap();
        assert_eq!(BakedProbeCache::load_from_path(&path).unwrap(), cache);
        assert!(!dir.join("scene.vprobes.tmp").exists());
        std::fs::write(&path, &encoded).unwrap();
        assert_eq!(
            BakedProbeCache::load_from_path(&path).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
        std::fs::remove_dir_all(&dir).unwrap();

        // Mismatched probe count vs dims is rejected:
        assert!(
            BakedProbeCache::new(
                [0.0, 0.0, 0.0, 1.0],
                [1.0, 1.0, 1.0, 0.0],
                [2, 2, 2, 0],
                vec![[0.0; 16]; 4]
            )
            .is_err()
        );
    }
}

#[cfg(test)]
#[path = "probe_projection_replay_tests.rs"]
mod projection_replay_tests;
