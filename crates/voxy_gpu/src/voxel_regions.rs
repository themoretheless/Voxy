//! Integer broadphase classification; continuous swept collision remains external.
use voxy_render::{ComputeError, ComputeProgram, PendingComputeReadback};

#[derive(Clone, Copy, Debug)]
pub enum VoxelClass {
    Empty,
    Solid,
    Unavailable,
    Unknown,
}
#[derive(Clone, Copy, Debug)]
pub struct VoxelRegion {
    pub min: [u32; 3],
    pub max: [u32; 3],
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VoxelRegionResult {
    pub solid_count: u32,
    pub first_solid: Option<u32>,
    pub first_fault: Option<u32>,
}
#[derive(Debug)]
pub struct VoxelRegionProgram {
    device: wgpu::Device,
    program: ComputeProgram,
}
/// An owned classification submission. Input metadata remains immutable until readback.
#[derive(Debug)]
pub struct PendingVoxelRegions {
    readback: PendingComputeReadback,
    words: Vec<u32>,
    dimensions: [u32; 3],
    cell_count: usize,
    submission: wgpu::SubmissionIndex,
}
impl PendingVoxelRegions {
    /// Takes a completed result once; returns `None` while GPU work is pending.
    /// Native callers must poll the owning device; browser callbacks progress naturally.
    /// # Errors
    /// Returns mapping, corrupt output or consumed-result errors.
    pub fn try_result(&mut self) -> Result<Option<Vec<VoxelRegionResult>>, ComputeError> {
        self.readback
            .try_read()?
            .map(|bytes| decode_regions(&bytes, &self.words, self.dimensions, self.cell_count))
            .transpose()
    }
}
impl VoxelRegionProgram {
    /// Creates parallel integer region classification on the supplied device.
    /// # Errors
    /// Returns unsupported compute or pipeline validation errors.
    pub async fn new(device: &wgpu::Device) -> Result<Self, ComputeError> {
        Ok(Self {
            device: device.clone(),
            program: ComputeProgram::new(device, include_str!("voxel_regions.wgsl")).await?,
        })
    }
    /// Classifies inclusive local integer regions in a dense canonical x/y/z grid.
    /// Indices retain integer anchors externally; no absolute float conversion.
    /// Supports 131072 cells, 16384 queries and 4194304 aggregate cell visits.
    /// Native synchronous readback. Queue must belong to the program's device.
    /// # Errors
    /// Rejects invalid dimensions, grids, regions, limits and device failures.
    pub fn classify(
        &self,
        queue: &wgpu::Queue,
        dimensions: [u32; 3],
        cells: &[VoxelClass],
        regions: &[VoxelRegion],
    ) -> Result<Vec<VoxelRegionResult>, ComputeError> {
        let mut pending = self.begin_classify(queue, dimensions, cells, regions)?;
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(pending.submission.clone()),
                timeout: None,
            })
            .map_err(|failure| ComputeError::Mapping(failure.to_string()))?;
        pending.try_result()?.ok_or(ComputeError::InvalidBuffer)
    }

    /// Submits classification and starts readback without waiting for GPU completion.
    /// Uses the same limits and input validation as `classify`.
    /// # Errors
    /// Rejects invalid inputs and compute submission failures.
    pub fn begin_classify(
        &self,
        queue: &wgpu::Queue,
        dimensions: [u32; 3],
        cells: &[VoxelClass],
        regions: &[VoxelRegion],
    ) -> Result<PendingVoxelRegions, ComputeError> {
        let count = dimensions
            .iter()
            .try_fold(1_u32, |n, &dimension| {
                if dimension == 0 || dimension > 512 {
                    None
                } else {
                    n.checked_mul(dimension)
                }
            })
            .ok_or(ComputeError::InvalidBuffer)?;
        if count > 131_072
            || usize::try_from(count).ok() != Some(cells.len())
            || regions.is_empty()
            || regions.len() > 16_384
            || regions.iter().any(|region| {
                (0..3).any(|axis| {
                    region.min[axis] > region.max[axis] || region.max[axis] >= dimensions[axis]
                })
            })
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let visits: u64 = regions
            .iter()
            .map(|region| {
                (0..3)
                    .map(|axis| u64::from(region.max[axis] - region.min[axis] + 1))
                    .product::<u64>()
            })
            .sum();
        if visits > 4_194_304 {
            return Err(ComputeError::InvalidBuffer);
        }
        let queries = u32::try_from(regions.len()).map_err(|_| ComputeError::InvalidBuffer)?;
        let mut words = vec![dimensions[0], dimensions[1], dimensions[2], queries, count];
        words.extend(cells.iter().map(|cell| match cell {
            VoxelClass::Empty => 0,
            VoxelClass::Solid => 1,
            VoxelClass::Unavailable => 2,
            VoxelClass::Unknown => 3,
        }));
        for region in regions {
            words.extend(region.min);
            words.extend(region.max);
            words.extend([0, u32::MAX, u32::MAX]);
        }
        let job = self
            .program
            .create_job(&self.device, bytemuck::cast_slice(&words))?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let dispatch = job.encode(&mut encoder, [queries.div_ceil(64), 1, 1])?;
        let submission = queue.submit([encoder.finish()]);
        Ok(PendingVoxelRegions {
            readback: dispatch.begin_read(),
            words,
            dimensions,
            cell_count: cells.len(),
            submission,
        })
    }

    /// Advances native mapping callbacks without waiting for GPU completion.
    /// # Errors
    /// Returns device polling failures.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn poll_native(&self) -> Result<(), ComputeError> {
        self.device
            .poll(wgpu::PollType::Poll)
            .map(|_| ())
            .map_err(|failure| ComputeError::Mapping(failure.to_string()))
    }
}

fn decode_regions(
    bytes: &[u8],
    words: &[u32],
    dimensions: [u32; 3],
    cell_count: usize,
) -> Result<Vec<VoxelRegionResult>, ComputeError> {
    if bytes.len() != words.len() * 4 {
        return Err(ComputeError::InvalidBuffer);
    }
    let output: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let base = 5 + cell_count;
    if output[..base] != words[..base] {
        return Err(ComputeError::InvalidBuffer);
    }
    output[base..]
        .chunks_exact(9)
        .zip(words[base..].chunks_exact(9))
        .map(|(row, original)| {
            let volume: u32 = (0..3)
                .map(|axis| original[axis + 3] - original[axis] + 1)
                .product();
            let candidate_valid = |index: u32, fault: bool| {
                if index == u32::MAX {
                    return true;
                }
                let Ok(index_usize) = usize::try_from(index) else {
                    return false;
                };
                if index_usize >= cell_count {
                    return false;
                }
                let position = [
                    index / (dimensions[1] * dimensions[2]),
                    index / dimensions[2] % dimensions[1],
                    index % dimensions[2],
                ];
                let class = words[5 + index_usize];
                (0..3).all(|axis| {
                    position[axis] >= original[axis] && position[axis] <= original[axis + 3]
                }) && if fault {
                    class == 2 || class == 3
                } else {
                    class == 1
                }
            };
            if row[..6] != original[..6]
                || row[6] > volume
                || (row[6] == 0) != (row[7] == u32::MAX)
                || !candidate_valid(row[7], false)
                || !candidate_valid(row[8], true)
            {
                return Err(ComputeError::InvalidBuffer);
            }
            Ok(VoxelRegionResult {
                solid_count: row[6],
                first_solid: (row[7] != u32::MAX).then_some(row[7]),
                first_fault: (row[8] != u32::MAX).then_some(row[8]),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_corrupted_region_results() {
        // Query covers only cell 1: cell 0 is solid but outside the region.
        let words = vec![
            1,
            1,
            4,
            1,
            4,
            1,
            1,
            2,
            0,
            0,
            0,
            1,
            0,
            0,
            1,
            0,
            u32::MAX,
            u32::MAX,
        ];
        let mut valid = words.clone();
        valid[15] = 1;
        valid[16] = 1;
        assert_eq!(
            decode_regions(bytemuck::cast_slice(&valid), &words, [1, 1, 4], 4).unwrap()[0]
                .first_solid,
            Some(1)
        );
        for (offset, value) in [
            (15, 2),
            (15, 0),
            (16, u32::MAX),
            (16, 0),
            (16, 2),
            (16, 4),
            (17, 1),
            (17, 2),
            (9, 1),
            (5, 0),
        ] {
            let mut corrupt = valid.clone();
            corrupt[offset] = value;
            assert!(
                decode_regions(bytemuck::cast_slice(&corrupt), &words, [1, 1, 4], 4).is_err(),
                "offset {offset}, value {value}"
            );
        }
    }
}
