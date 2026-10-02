//! Optional skeletal index residency; vertex, palette and object streams are shared.
use crate::skinned::{GpuSkinnedMesh, ObjectUniform, upload_skinned};
use crate::{PreparedSkinnedLod, SkinnedLodError, SkinnedMesh, SkinnedUploadError};
use std::{fmt, sync::Arc};
use wgpu::util::DeviceExt;

#[derive(Debug)]
pub enum SkinnedLodGpuError {
    Policy(crate::LodError),
    InvalidLevel,
    ForeignDevice,
    BudgetExceeded {
        live: u64,
        additional: u64,
        budget: u64,
    },
    Skin(SkinnedUploadError),
    Pose(SkinnedLodError),
}
impl fmt::Display for SkinnedLodGpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "skinned LOD GPU error: {self:?}")
    }
}
impl std::error::Error for SkinnedLodGpuError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkinnedLodResidency {
    pub selected_level: usize,
    /// Shared vertices/palette/object buffers plus resident indices; driver and
    /// in-flight allocations, attachments and other renderer resources excluded.
    pub resident_bytes: u64,
    pub resident_levels: usize,
}

#[derive(Debug)]
pub(crate) struct Level {
    pub index: wgpu::Buffer,
    pub mesh: SkinnedMesh,
}
#[derive(Debug)]
pub(crate) struct GpuSkinnedLod {
    pub pose: PreparedSkinnedLod,
    pub levels: Vec<Option<Level>>,
    pub selected: usize,
    pub bytes: u64,
    pub policy: Option<crate::LodPolicy>,
    device: wgpu::Device,
}

impl GpuSkinnedLod {
    pub fn upload(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        pose: PreparedSkinnedLod,
        material: u32,
        live: u64,
        budget: u64,
    ) -> Result<(GpuSkinnedMesh, Self), SkinnedLodGpuError> {
        let mesh = pose.source().mesh();
        let bytes = [
            std::mem::size_of_val(mesh.vertices()),
            std::mem::size_of_val(mesh.indices()),
            std::mem::size_of_val(pose.joints()),
            std::mem::size_of::<ObjectUniform>(),
        ]
        .into_iter()
        .try_fold(0_u64, |total, bytes| total.checked_add(bytes as u64))
        .ok_or(SkinnedLodGpuError::BudgetExceeded {
            live,
            additional: u64::MAX,
            budget,
        })?;
        admission(live, bytes, budget)?;
        let gpu = upload_skinned(device, layout, mesh, pose.joints(), pose.model(), material)
            .map_err(SkinnedLodGpuError::Skin)?;
        let mut levels: Vec<_> = (0..pose.levels().len()).map(|_| None).collect();
        levels[0] = Some(Level {
            index: gpu.index.clone(),
            mesh: mesh.clone(),
        });
        Ok((
            gpu,
            Self {
                pose,
                levels,
                selected: 0,
                bytes,
                policy: None,
                device: device.clone(),
            },
        ))
    }
    pub fn ensure(
        &mut self,
        device: &wgpu::Device,
        level: usize,
        budget: u64,
    ) -> Result<bool, SkinnedLodGpuError> {
        if device != &self.device {
            return Err(SkinnedLodGpuError::ForeignDevice);
        }
        let current = self
            .levels
            .get(level)
            .ok_or(SkinnedLodGpuError::InvalidLevel)?;
        if current.is_some() {
            return Ok(false);
        }
        let indices = self
            .pose
            .source()
            .indices(level)
            .ok_or(SkinnedLodGpuError::InvalidLevel)?;
        let bytes = std::mem::size_of_val(indices) as u64;
        admission(self.bytes, bytes, budget)?;
        let mesh = self
            .pose
            .source()
            .mesh()
            .with_shared_indices(Arc::from(indices))
            .map_err(|_| SkinnedLodGpuError::InvalidLevel)?;
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Voxy skeletal LOD indices"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.levels[level] = Some(Level { index, mesh });
        self.bytes += bytes;
        Ok(true)
    }
    pub fn fallback(&self, desired: usize) -> Result<usize, SkinnedLodGpuError> {
        if desired >= self.levels.len() {
            return Err(SkinnedLodGpuError::InvalidLevel);
        }
        (0..=desired)
            .rev()
            .find(|&level| self.levels[level].is_some())
            .ok_or(SkinnedLodGpuError::InvalidLevel)
    }
    pub fn evict(&mut self, level: usize) -> Result<bool, SkinnedLodGpuError> {
        if level == 0 || level >= self.levels.len() || self.selected == level {
            return Err(SkinnedLodGpuError::InvalidLevel);
        }
        let Some(resident) = self.levels[level].take() else {
            return Ok(false);
        };
        self.bytes -= resident.index.size();
        Ok(true)
    }
    pub fn residency(&self) -> SkinnedLodResidency {
        SkinnedLodResidency {
            selected_level: self.selected,
            resident_bytes: self.bytes,
            resident_levels: self.levels.iter().filter(|level| level.is_some()).count(),
        }
    }
    pub fn bind(
        &mut self,
        level: usize,
        gpu: &mut GpuSkinnedMesh,
        motion: &mut Option<crate::skinned_motion::ResidentSkinnedMotion>,
    ) -> Result<bool, SkinnedLodGpuError> {
        let resident = self
            .levels
            .get(level)
            .and_then(Option::as_ref)
            .ok_or(SkinnedLodGpuError::InvalidLevel)?;
        if self.selected == level {
            return Ok(false);
        }
        gpu.index = resident.index.clone();
        gpu.index_count = self.pose.levels()[level].index_count;
        self.selected = level;
        *motion = Some(crate::skinned_motion::ResidentSkinnedMotion {
            history: crate::SkinnedMotionHistory::new(resident.mesh.clone()),
            joints: self.pose.joints().to_vec(),
            model: self.pose.model(),
        });
        Ok(true)
    }
}
fn admission(live: u64, additional: u64, budget: u64) -> Result<(), SkinnedLodGpuError> {
    if live
        .checked_add(additional)
        .is_none_or(|total| total > budget)
    {
        return Err(SkinnedLodGpuError::BudgetExceeded {
            live,
            additional,
            budget,
        });
    }
    Ok(())
}
