//! Bounded exact accepted CPU geometry/pixels. GPU handles, LOD proofs and model
//! animation ownership belong to their enclosing resource participants.
mod skinned_set;
pub use skinned_set::{SkinnedMeshSetCheckpoint, SkinnedMeshSetEntry, SkinnedMeshSetLimits};
#[derive(Clone, Copy, Debug)]
pub struct GeometryArchiveLimits {
    pub max_bytes: usize,
    pub max_vertices: usize,
    pub max_indices: usize,
    pub max_joints: usize,
    pub max_image_dimension: u32,
    pub max_pixel_bytes: u64,
}
impl Default for GeometryArchiveLimits {
    fn default() -> Self {
        Self {
            max_bytes: 512 * 1024 * 1024,
            max_vertices: 1_000_000,
            max_indices: 3_000_000,
            max_joints: crate::MAX_SKIN_JOINTS,
            max_image_dimension: 8192,
            max_pixel_bytes: 256 * 1024 * 1024,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum GeometryCheckpointError {
    #[error("invalid geometry archive")]
    Invalid,
    #[error("geometry archive policy exceeded")]
    Capacity,
    #[error(transparent)]
    Scene(#[from] crate::SceneError),
    #[error("invalid archived skin: {0:?}")]
    Skin(crate::SkinnedMeshError),
    #[error("invalid archived image: {0}")]
    Image(crate::ImageAssetError),
}
type Error = GeometryCheckpointError;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SceneMeshCheckpoint {
    pub version: u32,
    pub vertices: Vec<[u32; 9]>,
    pub indices: Vec<u32>,
    pub material_parameters: [u32; 4],
    pub normals: Option<Vec<[u32; 3]>>,
    pub material_coordinates: Option<Vec<[u32; 3]>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkinnedVertexCheckpoint {
    pub attributes: [u32; 8],
    pub joints: [u16; 4],
    pub weights: [u16; 4],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkinnedMeshCheckpoint {
    pub version: u32,
    pub vertices: Vec<SkinnedVertexCheckpoint>,
    pub indices: Vec<u32>,
    pub joint_count: u16,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageCheckpoint {
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

const HEADER: usize = 48;
fn encoded_size(
    prefix: usize,
    parts: &[(usize, usize)],
    limits: GeometryArchiveLimits,
) -> Result<usize, Error> {
    let size = parts
        .iter()
        .try_fold(prefix, |size, (count, stride)| {
            count.checked_mul(*stride).and_then(|n| size.checked_add(n))
        })
        .ok_or(Error::Capacity)?;
    if size > limits.max_bytes {
        return Err(Error::Capacity);
    }
    Ok(size)
}
fn image_size(width: u32, height: u32, limits: GeometryArchiveLimits) -> Result<usize, Error> {
    let size = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|n| n.checked_mul(4))
        .ok_or(Error::Capacity)?;
    if width == 0
        || height == 0
        || width > limits.max_image_dimension
        || height > limits.max_image_dimension
        || size > limits.max_pixel_bytes
    {
        return Err(Error::Capacity);
    }
    usize::try_from(size).map_err(|_| Error::Capacity)
}
struct Writer {
    bytes: Vec<u8>,
    limit: usize,
}
impl Writer {
    fn new(kind: u8, size: usize) -> Result<Self, Error> {
        if size <= HEADER {
            return Err(Error::Capacity);
        }
        let mut bytes = Vec::with_capacity(size);
        bytes.resize(HEADER, 0);
        let mut writer = Self { bytes, limit: size };
        writer.put(&[kind])?;
        Ok(writer)
    }
    fn put(&mut self, data: &[u8]) -> Result<(), Error> {
        if data.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(Error::Capacity);
        }
        self.bytes.extend_from_slice(data);
        Ok(())
    }
    fn count(&mut self, count: usize) -> Result<(), Error> {
        self.put(
            &u64::try_from(count)
                .map_err(|_| Error::Capacity)?
                .to_le_bytes(),
        )
    }
    fn words(&mut self, words: &[u32]) -> Result<(), Error> {
        for word in words {
            self.put(&word.to_le_bytes())?;
        }
        Ok(())
    }
    fn vectors<const N: usize>(&mut self, vectors: &[[u32; N]]) -> Result<(), Error> {
        self.count(vectors.len())?;
        for words in vectors {
            self.words(words)?;
        }
        Ok(())
    }
    fn indices(&mut self, indices: &[u32]) -> Result<(), Error> {
        self.count(indices.len())?;
        self.words(indices)
    }
    fn finish(mut self) -> Result<Vec<u8>, Error> {
        self.bytes[..8].copy_from_slice(b"VOXYGEO1");
        let length = u64::try_from(self.bytes.len() - HEADER).map_err(|_| Error::Capacity)?;
        self.bytes[8..16].copy_from_slice(&length.to_le_bytes());
        let digest = blake3::hash(&self.bytes[HEADER..]);
        self.bytes[16..48].copy_from_slice(digest.as_bytes());
        Ok(self.bytes)
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], kind: u8, limits: GeometryArchiveLimits) -> Result<Self, Error> {
        if bytes.len() > limits.max_bytes {
            return Err(Error::Capacity);
        }
        if bytes.len() <= HEADER || &bytes[..8] != b"VOXYGEO1" {
            return Err(Error::Invalid);
        }
        let length = u64::from_le_bytes(bytes[8..16].try_into().map_err(|_| Error::Invalid)?);
        if usize::try_from(length).ok() != Some(bytes.len() - HEADER)
            || blake3::hash(&bytes[HEADER..]).as_bytes() != &bytes[16..48]
        {
            return Err(Error::Invalid);
        }
        let mut reader = Self {
            bytes,
            offset: HEADER,
        };
        if reader.take(1)?[0] != kind {
            return Err(Error::Invalid);
        }
        Ok(reader)
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self.offset.checked_add(count).ok_or(Error::Invalid)?;
        let data = self.bytes.get(self.offset..end).ok_or(Error::Invalid)?;
        self.offset = end;
        Ok(data)
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().map_err(|_| Error::Invalid)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| Error::Invalid)?,
        ))
    }
    fn words<const N: usize>(&mut self) -> Result<[u32; N], Error> {
        let mut values = [0; N];
        for value in &mut values {
            *value = self.u32()?;
        }
        Ok(values)
    }
    fn count(&mut self, maximum: usize, stride: usize) -> Result<usize, Error> {
        let count = usize::try_from(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| Error::Invalid)?,
        ))
        .map_err(|_| Error::Capacity)?;
        if count > maximum {
            return Err(Error::Capacity);
        }
        if count
            .checked_mul(stride)
            .is_none_or(|bytes| bytes > self.bytes.len() - self.offset)
        {
            return Err(Error::Invalid);
        }
        Ok(count)
    }
    fn vectors<const N: usize>(&mut self, maximum: usize) -> Result<Vec<[u32; N]>, Error> {
        let count = self.count(maximum, N * 4)?;
        (0..count).map(|_| self.words()).collect()
    }
    fn indices(&mut self, maximum: usize) -> Result<Vec<u32>, Error> {
        let count = self.count(maximum, 4)?;
        (0..count).map(|_| self.u32()).collect()
    }
    fn end(self) -> Result<(), Error> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
}
impl SceneMeshCheckpoint {
    pub(crate) fn encoded_size(&self, limits: GeometryArchiveLimits) -> Result<usize, Error> {
        self.admit(limits)?;
        let streams =
            self.normals.is_some() as usize + self.material_coordinates.is_some() as usize;
        encoded_size(
            86 + streams * 8,
            &[
                (self.vertices.len(), 36),
                (self.indices.len(), 4),
                (self.normals.as_ref().map_or(0, Vec::len), 12),
                (self.material_coordinates.as_ref().map_or(0, Vec::len), 12),
            ],
            limits,
        )
    }
    pub(crate) fn admit(&self, limits: GeometryArchiveLimits) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::Invalid);
        }
        if self.vertices.len() > limits.max_vertices || self.indices.len() > limits.max_indices {
            return Err(Error::Capacity);
        }
        if self
            .normals
            .as_ref()
            .is_some_and(|v| v.len() != self.vertices.len())
            || self
                .material_coordinates
                .as_ref()
                .is_some_and(|v| v.len() != self.vertices.len())
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    pub fn to_bytes(&self, limits: GeometryArchiveLimits) -> Result<Vec<u8>, Error> {
        let size = self.encoded_size(limits)?;
        let mut writer = Writer::new(1, size)?;
        writer.words(&[self.version])?;
        writer.words(&self.material_parameters)?;
        writer
            .put(&[u8::from(self.normals.is_some())
                | (u8::from(self.material_coordinates.is_some()) << 1)])?;
        writer.vectors(&self.vertices)?;
        writer.indices(&self.indices)?;
        for values in [&self.normals, &self.material_coordinates]
            .into_iter()
            .flatten()
        {
            writer.vectors(values)?;
        }
        writer.finish()
    }
    pub fn from_bytes(bytes: &[u8], limits: GeometryArchiveLimits) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes, 1, limits)?;
        let version = reader.u32()?;
        if version != 1 {
            return Err(Error::Invalid);
        }
        let material_parameters = reader.words()?;
        let flags = reader.take(1)?[0];
        if flags > 3 {
            return Err(Error::Invalid);
        }
        let vertices = reader.vectors(limits.max_vertices)?;
        let indices = reader.indices(limits.max_indices)?;
        let normals = if flags & 1 != 0 {
            Some(reader.vectors(limits.max_vertices)?)
        } else {
            None
        };
        let material_coordinates = if flags & 2 != 0 {
            Some(reader.vectors(limits.max_vertices)?)
        } else {
            None
        };
        reader.end()?;
        let saved = Self {
            version,
            vertices,
            indices,
            material_parameters,
            normals,
            material_coordinates,
        };
        saved.admit(limits)?;
        Ok(saved)
    }
}
impl SkinnedMeshCheckpoint {
    pub(crate) fn encoded_size(&self, limits: GeometryArchiveLimits) -> Result<usize, Error> {
        self.admit(limits)?;
        encoded_size(
            71,
            &[(self.vertices.len(), 48), (self.indices.len(), 4)],
            limits,
        )
    }
    pub(crate) fn admit(&self, limits: GeometryArchiveLimits) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::Invalid);
        }
        if self.joint_count == 0 || usize::from(self.joint_count) > crate::MAX_SKIN_JOINTS {
            return Err(Error::Invalid);
        }
        if self.vertices.len() > limits.max_vertices
            || self.indices.len() > limits.max_indices
            || usize::from(self.joint_count) > limits.max_joints
        {
            return Err(Error::Capacity);
        }
        Ok(())
    }
    pub fn to_bytes(&self, limits: GeometryArchiveLimits) -> Result<Vec<u8>, Error> {
        let size = self.encoded_size(limits)?;
        let mut writer = Writer::new(2, size)?;
        writer.words(&[self.version])?;
        writer.put(&self.joint_count.to_le_bytes())?;
        writer.count(self.vertices.len())?;
        for vertex in &self.vertices {
            writer.words(&vertex.attributes)?;
            for value in vertex.joints.into_iter().chain(vertex.weights) {
                writer.put(&value.to_le_bytes())?;
            }
        }
        writer.indices(&self.indices)?;
        writer.finish()
    }
    pub fn from_bytes(bytes: &[u8], limits: GeometryArchiveLimits) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes, 2, limits)?;
        let version = reader.u32()?;
        if version != 1 {
            return Err(Error::Invalid);
        }
        let joint_count = reader.u16()?;
        if joint_count == 0 || usize::from(joint_count) > crate::MAX_SKIN_JOINTS {
            return Err(Error::Invalid);
        }
        if usize::from(joint_count) > limits.max_joints {
            return Err(Error::Capacity);
        }
        let count = reader.count(limits.max_vertices, 48)?;
        let mut vertices = Vec::with_capacity(count);
        for _ in 0..count {
            let attributes = reader.words()?;
            let mut joints = [0; 4];
            let mut weights = [0; 4];
            for value in &mut joints {
                *value = reader.u16()?;
            }
            for value in &mut weights {
                *value = reader.u16()?;
            }
            vertices.push(SkinnedVertexCheckpoint {
                attributes,
                joints,
                weights,
            });
        }
        let indices = reader.indices(limits.max_indices)?;
        reader.end()?;
        let saved = Self {
            version,
            vertices,
            indices,
            joint_count,
        };
        saved.admit(limits)?;
        Ok(saved)
    }
}
impl ImageCheckpoint {
    pub(crate) fn encoded_size(&self, limits: GeometryArchiveLimits) -> Result<usize, Error> {
        self.admit(limits)?;
        encoded_size(69, &[(self.rgba.len(), 1)], limits)
    }
    pub(crate) fn admit(&self, limits: GeometryArchiveLimits) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::Invalid);
        }
        if image_size(self.width, self.height, limits)? != self.rgba.len() {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    pub fn to_bytes(&self, limits: GeometryArchiveLimits) -> Result<Vec<u8>, Error> {
        let size = self.encoded_size(limits)?;
        let mut writer = Writer::new(3, size)?;
        writer.words(&[self.version, self.width, self.height])?;
        writer.count(self.rgba.len())?;
        writer.put(&self.rgba)?;
        writer.finish()
    }
    pub fn from_bytes(bytes: &[u8], limits: GeometryArchiveLimits) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes, 3, limits)?;
        let version = reader.u32()?;
        if version != 1 {
            return Err(Error::Invalid);
        }
        let width = reader.u32()?;
        let height = reader.u32()?;
        let expected = image_size(width, height, limits)?;
        let count = reader.count(expected, 1)?;
        if count != expected {
            return Err(Error::Invalid);
        }
        let rgba = reader.take(count)?.to_vec();
        reader.end()?;
        let saved = Self {
            version,
            width,
            height,
            rgba,
        };
        saved.admit(limits)?;
        Ok(saved)
    }
}

#[cfg(test)]
mod tests;
