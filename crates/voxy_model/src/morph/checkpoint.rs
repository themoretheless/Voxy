//! Accepted local deformation inputs, including exact material and sampler data.
//! Resource aliases and image-index resolution belong to the enclosing model owner.
use super::*;
use crate::ModelTexture;
use voxy_render::{
    GeometryArchiveLimits, SceneMeshCheckpoint, SkinnedMeshCheckpoint, TextureFilter,
    TextureSampling, TextureWrap,
};

#[derive(Clone, Copy, Debug)]
pub struct MorphArchiveLimits {
    pub geometry: GeometryArchiveLimits,
    pub max_bytes: usize,
    pub max_delta_vectors: usize,
}
impl Default for MorphArchiveLimits {
    fn default() -> Self {
        Self {
            geometry: GeometryArchiveLimits::default(),
            max_bytes: 512 * 1024 * 1024,
            max_delta_vectors: 8_000_000,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelPrimitiveCheckpoint(Vec<u8>);
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphGeometryCheckpoint(Vec<u8>);
const HEADER: usize = 48;
const MAGIC: &[u8; 8] = b"VOXYMRP1";
fn bad() -> ModelError {
    fail("invalid morph resource archive")
}
fn capacity() -> ModelError {
    fail("morph resource archive exceeds policy")
}
fn native(e: impl std::fmt::Display) -> ModelError {
    fail(e.to_string())
}
fn wrap(v: TextureWrap) -> u8 {
    match v {
        TextureWrap::Clamp => 0,
        TextureWrap::Repeat => 1,
        TextureWrap::Mirror => 2,
    }
}
fn unwrap_wrap(v: u8) -> Result<TextureWrap, ModelError> {
    match v {
        0 => Ok(TextureWrap::Clamp),
        1 => Ok(TextureWrap::Repeat),
        2 => Ok(TextureWrap::Mirror),
        _ => Err(bad()),
    }
}
fn filter(v: TextureFilter) -> u8 {
    match v {
        TextureFilter::Nearest => 0,
        TextureFilter::Linear => 1,
    }
}
fn unwrap_filter(v: u8) -> Result<TextureFilter, ModelError> {
    match v {
        0 => Ok(TextureFilter::Nearest),
        1 => Ok(TextureFilter::Linear),
        _ => Err(bad()),
    }
}
struct Writer {
    bytes: Vec<u8>,
    limit: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<(), ModelError> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > self.limit)
        {
            return Err(capacity());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn byte(&mut self, v: u8) -> Result<(), ModelError> {
        self.put(&[v])
    }
    fn count(&mut self, v: usize) -> Result<(), ModelError> {
        self.put(&u64::try_from(v).map_err(|_| capacity())?.to_le_bytes())
    }
    fn finish(self) -> Result<Vec<u8>, ModelError> {
        let mut bytes =
            Vec::with_capacity(HEADER.checked_add(self.bytes.len()).ok_or_else(capacity)?);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&(self.bytes.len() as u64).to_le_bytes());
        bytes.extend_from_slice(blake3::hash(&self.bytes).as_bytes());
        bytes.extend_from_slice(&self.bytes);
        Ok(bytes)
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], ModelError> {
        let end = self.offset.checked_add(count).ok_or_else(bad)?;
        let bytes = self.bytes.get(self.offset..end).ok_or_else(bad)?;
        self.offset = end;
        Ok(bytes)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], ModelError> {
        self.take(N)?.try_into().map_err(|_| bad())
    }
    fn byte(&mut self) -> Result<u8, ModelError> {
        Ok(self.array::<1>()?[0])
    }
    fn count(&mut self) -> Result<usize, ModelError> {
        usize::try_from(u64::from_le_bytes(self.array()?)).map_err(|_| capacity())
    }
    fn done(&self) -> Result<(), ModelError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(bad())
        }
    }
}
fn body(bytes: &[u8], limits: MorphArchiveLimits, kind: u8) -> Result<&[u8], ModelError> {
    if bytes.len() > limits.max_bytes {
        return Err(capacity());
    }
    let mut r = Reader { bytes, offset: 0 };
    if r.take(8)? != MAGIC {
        return Err(bad());
    }
    let n = r.count()?;
    let digest = r.take(32)?;
    let body = r.take(n)?;
    r.done()?;
    if body.first() != Some(&kind) || blake3::hash(body).as_bytes() != digest {
        return Err(bad());
    }
    Ok(&body[1..])
}
fn writer(limits: MorphArchiveLimits, kind: u8) -> Result<Writer, ModelError> {
    let mut w = Writer {
        bytes: Vec::new(),
        limit: limits.max_bytes.checked_sub(HEADER).ok_or_else(capacity)?,
    };
    w.byte(kind)?;
    Ok(w)
}
fn geometry_limits(limits: MorphArchiveLimits) -> GeometryArchiveLimits {
    GeometryArchiveLimits {
        max_bytes: limits.geometry.max_bytes.min(limits.max_bytes),
        ..limits.geometry
    }
}
fn write_primitive(
    w: &mut Writer,
    source: &ModelPrimitive,
    limits: MorphArchiveLimits,
) -> Result<(), ModelError> {
    let ModelPrimitive {
        geometry,
        color,
        base_color_texture,
    } = source;
    let (kind, data) = match geometry {
        ModelGeometry::Static(mesh) => (
            0,
            mesh.checkpoint()
                .to_bytes(geometry_limits(limits))
                .map_err(native)?,
        ),
        ModelGeometry::Skinned(mesh) => (
            1,
            mesh.checkpoint()
                .to_bytes(geometry_limits(limits))
                .map_err(native)?,
        ),
    };
    w.byte(kind)?;
    w.count(data.len())?;
    w.put(&data)?;
    for v in color {
        w.put(&v.to_bits().to_le_bytes())?;
    }
    w.byte(u8::from(base_color_texture.is_some()))?;
    if let Some(texture) = base_color_texture {
        let ModelTexture {
            image,
            sampling,
            use_mips,
        } = texture;
        let TextureSampling {
            wrap_u,
            wrap_v,
            min_filter,
            mag_filter,
            mipmap_filter,
            anisotropy,
        } = sampling;
        w.count(*image)?;
        w.put(&[
            wrap(*wrap_u),
            wrap(*wrap_v),
            filter(*min_filter),
            filter(*mag_filter),
            mipmap_filter.map_or(2, filter),
        ])?;
        w.put(&anisotropy.to_le_bytes())?;
        w.byte(u8::from(*use_mips))?;
    }
    Ok(())
}
fn read_primitive(
    r: &mut Reader<'_>,
    limits: MorphArchiveLimits,
) -> Result<ModelPrimitive, ModelError> {
    let kind = r.byte()?;
    let len = r.count()?;
    let data = r.take(len)?;
    let geometry = match kind {
        0 => ModelGeometry::Static(
            SceneMeshCheckpoint::from_bytes(data, geometry_limits(limits))
                .map_err(native)?
                .prepare(geometry_limits(limits))
                .map_err(native)?,
        ),
        1 => ModelGeometry::Skinned(
            SkinnedMeshCheckpoint::from_bytes(data, geometry_limits(limits))
                .map_err(native)?
                .prepare(geometry_limits(limits))
                .map_err(native)?,
        ),
        _ => return Err(bad()),
    };
    let mut color = [0.; 4];
    for v in &mut color {
        *v = f32::from_bits(u32::from_le_bytes(r.array()?));
    }
    let base_color_texture = match r.byte()? {
        0 => None,
        1 => {
            let image = r.count()?;
            let wrap_u = unwrap_wrap(r.byte()?)?;
            let wrap_v = unwrap_wrap(r.byte()?)?;
            let min_filter = unwrap_filter(r.byte()?)?;
            let mag_filter = unwrap_filter(r.byte()?)?;
            let mipmap_filter = match r.byte()? {
                2 => None,
                v => Some(unwrap_filter(v)?),
            };
            let anisotropy = u16::from_le_bytes(r.array()?);
            let use_mips = match r.byte()? {
                0 => false,
                1 => true,
                _ => return Err(bad()),
            };
            Some(ModelTexture {
                image,
                sampling: TextureSampling {
                    wrap_u,
                    wrap_v,
                    min_filter,
                    mag_filter,
                    mipmap_filter,
                    anisotropy,
                },
                use_mips,
            })
        }
        _ => return Err(bad()),
    };
    Ok(ModelPrimitive {
        geometry,
        color,
        base_color_texture,
    })
}
impl ModelPrimitive {
    /// Preserve accepted geometry and exact metadata. Image indices are resolved by the enclosing model.
    pub fn checkpoint(
        &self,
        limits: MorphArchiveLimits,
    ) -> Result<ModelPrimitiveCheckpoint, ModelError> {
        let mut w = writer(limits, 0)?;
        write_primitive(&mut w, self, limits)?;
        Ok(ModelPrimitiveCheckpoint(w.finish()?))
    }
}
impl ModelPrimitiveCheckpoint {
    /// Check envelope integrity and type. Call prepare for native semantic admission.
    pub fn from_bytes(bytes: &[u8], limits: MorphArchiveLimits) -> Result<Self, ModelError> {
        body(bytes, limits, 0)?;
        Ok(Self(bytes.to_vec()))
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn prepare(&self, limits: MorphArchiveLimits) -> Result<ModelPrimitive, ModelError> {
        let mut r = Reader {
            bytes: body(&self.0, limits, 0)?,
            offset: 0,
        };
        let result = read_primitive(&mut r, limits)?;
        r.done()?;
        Ok(result)
    }
}
impl MorphGeometry {
    pub fn checkpoint(
        &self,
        limits: MorphArchiveLimits,
    ) -> Result<MorphGeometryCheckpoint, ModelError> {
        let Self { source, targets } = self;
        let mut count = 0usize;
        for stream in targets
            .iter()
            .flat_map(|t| [&t.positions, &t.normals])
            .flatten()
        {
            count = count.checked_add(stream.len()).ok_or_else(capacity)?;
        }
        if count > limits.max_delta_vectors {
            return Err(capacity());
        }
        let mut w = writer(limits, 1)?;
        write_primitive(&mut w, source, limits)?;
        w.count(targets.len())?;
        for target in targets {
            let MorphTarget { positions, normals } = target;
            for stream in [positions, normals] {
                w.byte(u8::from(stream.is_some()))?;
                if let Some(stream) = stream {
                    w.count(stream.len())?;
                    // Check the entire stream before appending any of it.
                    let size = stream.len().checked_mul(12).ok_or_else(capacity)?;
                    if w.bytes.len().checked_add(size).is_none_or(|n| n > w.limit) {
                        return Err(capacity());
                    }
                    for v in stream.iter().flatten() {
                        w.put(&v.to_bits().to_le_bytes())?;
                    }
                }
            }
        }
        Ok(MorphGeometryCheckpoint(w.finish()?))
    }
}
impl MorphGeometryCheckpoint {
    /// Check envelope integrity and type. Call prepare for native semantic admission.
    pub fn from_bytes(bytes: &[u8], limits: MorphArchiveLimits) -> Result<Self, ModelError> {
        body(bytes, limits, 1)?;
        Ok(Self(bytes.to_vec()))
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn prepare(&self, limits: MorphArchiveLimits) -> Result<MorphGeometry, ModelError> {
        let mut r = Reader {
            bytes: body(&self.0, limits, 1)?,
            offset: 0,
        };
        let source = read_primitive(&mut r, limits)?;
        let vertices = match &source.geometry {
            ModelGeometry::Static(mesh) => mesh.vertices().len(),
            ModelGeometry::Skinned(mesh) => mesh.vertices().len(),
        };
        let n = r.count()?;
        if n == 0 || n > voxy_render::MAX_MORPH_TARGETS {
            return Err(bad());
        }
        let mut targets = Vec::new();
        let mut count = 0usize;
        for _ in 0..n {
            let mut streams = [None, None];
            for stream in &mut streams {
                match r.byte()? {
                    0 => {}
                    1 => {
                        let n = r.count()?;
                        if n != vertices {
                            return Err(bad());
                        }
                        count = count.checked_add(n).ok_or_else(capacity)?;
                        if count > limits.max_delta_vectors {
                            return Err(capacity());
                        }
                        let bytes = r.take(n.checked_mul(12).ok_or_else(capacity)?)?;
                        *stream = Some(
                            bytes
                                .chunks_exact(12)
                                .map(|v| {
                                    std::array::from_fn(|i| {
                                        f32::from_bits(u32::from_le_bytes(
                                            v[i * 4..i * 4 + 4].try_into().expect("fixed stride"),
                                        ))
                                    })
                                })
                                .collect(),
                        );
                    }
                    _ => return Err(bad()),
                }
            }
            let [positions, normals] = streams;
            targets.push(MorphTarget { positions, normals });
        }
        r.done()?;
        MorphGeometry::new(source, targets, limits.max_delta_vectors)
    }
}
#[cfg(test)]
mod tests;
