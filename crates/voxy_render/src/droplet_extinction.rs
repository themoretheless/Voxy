//! Physical droplet extinction upload through the existing compute job/readback owner.
use crate::ComputeError;
pub const DROPLET_EXTINCTION_REFERENCE_SHADER: &str =
    include_str!("droplet_extinction_reference.wgsl");
pub const DROPLET_EXTINCTION_SHADER: &str = concat!(
    include_str!("droplet_extinction_common.wgsl"),
    include_str!("droplet_extinction.wgsl")
);
pub const DROPLET_EXTINCTION_COMPOSITE_SHADER: &str = concat!(
    include_str!("droplet_extinction_common.wgsl"),
    include_str!("droplet_extinction_composite.wgsl")
);
/// Read-only physical field transport; the renderer does not own the simulation.
#[derive(Clone, Copy, Debug)]
pub struct ExtinctionGridView<'a> {
    pub origin: [f64; 3],
    pub spacing: [f64; 3],
    pub shape: [usize; 3],
    pub extinction_m_inverse: &'a [f64],
}
#[derive(Debug)]
pub struct DropletExtinctionComputeInput {
    words: Vec<u32>,
    cells: usize,
    rays: usize,
}
impl DropletExtinctionComputeInput {
    /// Pack one immutable extinction snapshot and finite line segments; budget includes ABI storage.
    /// Device compute/readback budgets are still admitted by ComputeProgram::create_job.
    pub fn new(
        grid: ExtinctionGridView<'_>,
        rays: &[([f64; 3], [f64; 3])],
        max_bytes: usize,
    ) -> Result<Self, ComputeError> {
        let cells = grid.extinction_m_inverse.len();
        let count = grid.shape.iter().try_fold(1usize, |n, &x| n.checked_mul(x));
        if grid.shape.iter().any(|&n| n > i32::MAX as usize)
            || grid
                .shape
                .iter()
                .try_fold(3usize, |sum, &n| sum.checked_add(n))
                .is_none_or(|sum| sum > u32::MAX as usize)
            || count != Some(cells)
            || cells == 0
            || grid.shape.contains(&0)
            || grid.spacing.iter().any(|&x| !x.is_finite() || x <= 0.)
            || grid
                .extinction_m_inverse
                .iter()
                .any(|&x| !x.is_finite() || x < 0.)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let words = rays
            .len()
            .checked_mul(8)
            .and_then(|r| r.checked_add(cells))
            .and_then(|n| n.checked_add(12))
            .ok_or(ComputeError::InvalidBuffer)?;
        if rays.is_empty()
            || words > u32::MAX as usize
            || words.checked_mul(4).is_none_or(|n| n > max_bytes)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let convert = |x: f64| {
            let v = x as f32;
            if !v.is_finite() || (x != 0. && v == 0.) {
                Err(ComputeError::InvalidBuffer)
            } else {
                Ok(v.to_bits())
            }
        };
        let shape = grid.shape;
        let mut data = Vec::with_capacity(words);
        data.extend(shape.map(|n| n as u32));
        data.extend([cells as u32, rays.len() as u32, 0]);
        for x in grid.origin.into_iter().chain(grid.spacing) {
            data.push(convert(x)?);
        }
        for k in 0..3 {
            let origin = grid.origin[k] as f32;
            let spacing = grid.spacing[k] as f32;
            let end = origin + grid.shape[k] as f32 * spacing;
            let last = origin + (grid.shape[k] - 1) as f32 * spacing;
            if !end.is_finite() || origin + spacing == origin || last + spacing == last {
                return Err(ComputeError::InvalidBuffer);
            }
        }
        for &x in grid.extinction_m_inverse {
            data.push(convert(x)?);
        }
        for &(start, end) in rays {
            let delta: [f32; 3] = std::array::from_fn(|k| (end[k] as f32) - (start[k] as f32));
            if delta.iter().any(|x| !x.is_finite())
                || !delta.iter().map(|x| x * x).sum::<f32>().is_finite()
            {
                return Err(ComputeError::InvalidBuffer);
            }
            for x in start.into_iter().chain(end) {
                data.push(convert(x)?);
            }
            data.extend([0, 0]);
        }
        Ok(Self {
            words: data,
            cells,
            rays: rays.len(),
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        [(self.rays as u32).div_ceil(64), 1, 1]
    }
    /// Reject malformed/nonfinite GPU output before publishing transmission.
    pub fn decode(&self, bytes: &[u8]) -> Result<Vec<[f32; 2]>, ComputeError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(ComputeError::InvalidBuffer);
        }
        let mut result = Vec::with_capacity(self.rays);
        for ray in 0..self.rays {
            let base = (12 + self.cells + 8 * ray + 6) * 4;
            let read = |offset| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let tau = read(base);
            let transmission = read(base + 4);
            if !tau.is_finite()
                || tau < 0.
                || !transmission.is_finite()
                || !(0. ..=1.).contains(&transmission)
                || (transmission - (-tau).exp()).abs() > 1e-5
            {
                return Err(ComputeError::InvalidBuffer);
            }
            result.push([tau, transmission]);
        }
        Ok(result)
    }
}

/// Linear-light direct-transmission composition. Alpha is preserved; no in-scattering is inferred.
#[derive(Debug)]
pub struct DropletExtinctionCompositeInput {
    rays: DropletExtinctionComputeInput,
    words: Vec<u32>,
}
impl DropletExtinctionCompositeInput {
    pub fn new(
        grid: ExtinctionGridView<'_>,
        rays: &[([f64; 3], [f64; 3])],
        linear_rgba: &[[f32; 4]],
        max_bytes: usize,
    ) -> Result<Self, ComputeError> {
        if linear_rgba.len() != rays.len()
            || linear_rgba
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || *v < 0.)
            || linear_rgba.iter().any(|v| v[3] > 1.)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        let color_bytes = rays
            .len()
            .checked_mul(16)
            .ok_or(ComputeError::InvalidBuffer)?;
        let remaining = max_bytes
            .checked_sub(color_bytes)
            .ok_or(ComputeError::InvalidBuffer)?;
        let packed = DropletExtinctionComputeInput::new(grid, rays, remaining)?;
        let total = packed
            .words
            .len()
            .checked_add(color_bytes / 4)
            .ok_or(ComputeError::InvalidBuffer)?;
        if total > u32::MAX as usize {
            return Err(ComputeError::InvalidBuffer);
        }
        let mut words = packed.words.clone();
        words.extend(linear_rgba.iter().flatten().map(|v| v.to_bits()));
        Ok(Self {
            rays: packed,
            words,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        self.rays.workgroups()
    }
    pub fn decode(&self, bytes: &[u8]) -> Result<Vec<[f32; 4]>, ComputeError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(ComputeError::InvalidBuffer);
        }
        let offset = self.rays.words.len() * 4;
        let attenuation = self.rays.decode(&bytes[..offset])?;
        let colors = bytes[offset..]
            .chunks_exact(16)
            .map(|pixel| {
                std::array::from_fn(|k| {
                    f32::from_le_bytes(pixel[k * 4..k * 4 + 4].try_into().unwrap())
                })
            })
            .collect::<Vec<[f32; 4]>>();
        for (i, color) in colors.iter().enumerate() {
            let initial = std::array::from_fn::<_, 4, _>(|k| {
                f32::from_bits(self.words[self.rays.words.len() + 4 * i + k])
            });
            if color.iter().any(|v| !v.is_finite() || *v < 0.)
                || color[3] != initial[3]
                || (0..3).any(|k| color[k] > initial[k])
                || (0..3).any(|k| {
                    (color[k] - initial[k] * attenuation[i][1]).abs() > 2e-6 * initial[k].max(1.)
                })
            {
                return Err(ComputeError::InvalidBuffer);
            }
        }
        Ok(colors)
    }
}

/// Construct world-space segments ending at supplied WebGPU 0..1 opaque depth.
/// Pixel centers use top-left image coordinates; orthographic rays begin at their near plane.
pub fn extinction_segments_from_depth(
    camera: crate::SceneCamera,
    width: u32,
    height: u32,
    depths: &[f32],
) -> Result<Vec<([f64; 3], [f64; 3])>, ComputeError> {
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or(ComputeError::InvalidBuffer)?;
    if width == 0
        || height == 0
        || depths.len() != count
        || depths
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
    {
        return Err(ComputeError::InvalidBuffer);
    }
    let inverse = camera
        .view_projection()
        .map_err(|_| ComputeError::InvalidBuffer)?
        .inverse();
    if !inverse.is_finite() {
        return Err(ComputeError::InvalidBuffer);
    }
    let mut rays = Vec::with_capacity(count);
    for (i, &depth) in depths.iter().enumerate() {
        let x = 2. * ((i % width as usize) as f32 + 0.5) / width as f32 - 1.;
        let y = 1. - 2. * ((i / width as usize) as f32 + 0.5) / height as f32;
        let endpoint = inverse.project_point3(glam::Vec3::new(x, y, depth));
        let start = match camera.projection {
            crate::SceneProjection::Perspective { .. } => camera.eye,
            crate::SceneProjection::Orthographic { .. } => {
                inverse.project_point3(glam::Vec3::new(x, y, 0.))
            }
        };
        if !start.is_finite() || !endpoint.is_finite() {
            return Err(ComputeError::InvalidBuffer);
        }
        rays.push((
            start.to_array().map(f64::from),
            endpoint.to_array().map(f64::from),
        ));
    }
    Ok(rays)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_depth_unprojection_preserves_pixel_center_and_projection_conventions() {
        for projection in [
            crate::SceneProjection::Perspective {
                vertical_fov: 1.,
                aspect: 1.,
                near: 0.1,
                far: 10.,
            },
            crate::SceneProjection::Orthographic {
                left: -1.,
                right: 1.,
                bottom: -1.,
                top: 1.,
                near: 0.1,
                far: 10.,
            },
        ] {
            let camera = crate::SceneCamera {
                eye: glam::Vec3::new(0., 0., 3.),
                target: glam::Vec3::ZERO,
                up: glam::Vec3::Y,
                projection,
            };
            let depths = [0., 0.2, 0.7, 1.];
            let rays = extinction_segments_from_depth(camera, 2, 2, &depths).unwrap();
            let vp = camera.view_projection().unwrap();
            for (i, &(start, end)) in rays.iter().enumerate() {
                let projected = vp.project_point3(glam::Vec3::from_array(end.map(|v| v as f32)));
                assert!((projected.x - [-0.5, 0.5][i % 2]).abs() < 1e-5);
                assert!((projected.y - [0.5, -0.5][i / 2]).abs() < 1e-5);
                assert!((projected.z - depths[i]).abs() < 1e-5);
                match projection {
                    crate::SceneProjection::Perspective { .. } => {
                        assert_eq!(start, camera.eye.to_array().map(f64::from))
                    }
                    crate::SceneProjection::Orthographic { .. } => {
                        let projected =
                            vp.project_point3(glam::Vec3::from_array(start.map(|v| v as f32)));
                        assert!(projected.z.abs() < 1e-5);
                    }
                }
            }
            assert!(extinction_segments_from_depth(camera, 0, 2, &[]).is_err());
            assert!(extinction_segments_from_depth(camera, 2, 2, &[0.; 3]).is_err());
            for depth in [f32::NAN, -0.1, 1.1] {
                assert!(extinction_segments_from_depth(camera, 1, 1, &[depth]).is_err());
            }
        }
    }
}

/// Scene-texture composition storage. Color/depth are sampled on GPU, never reconstructed on CPU.
#[derive(Debug)]
pub struct DropletExtinctionSceneInput {
    words: Vec<u32>,
    pixels: usize,
    color_offset: usize,
}
impl DropletExtinctionSceneInput {
    pub fn new(
        grid: ExtinctionGridView<'_>,
        camera: crate::SceneCamera,
        width: u32,
        height: u32,
        max_bytes: usize,
    ) -> Result<Self, ComputeError> {
        let count = (width as usize)
            .checked_mul(height as usize)
            .ok_or(ComputeError::InvalidBuffer)?;
        let size = count
            .checked_mul(48)
            .and_then(|n| {
                grid.extinction_m_inverse
                    .len()
                    .checked_mul(4)
                    .and_then(|g| n.checked_add(g))
            })
            .and_then(|n| n.checked_add(136))
            .ok_or(ComputeError::InvalidBuffer)?;
        if count == 0 || size > max_bytes || size / 4 > u32::MAX as usize {
            return Err(ComputeError::InvalidBuffer);
        }
        let inverse = camera
            .view_projection()
            .map_err(|_| ComputeError::InvalidBuffer)?
            .inverse();
        if !inverse.is_finite() {
            return Err(ComputeError::InvalidBuffer);
        }
        let packed = DropletExtinctionCompositeInput::new(
            grid,
            &vec![([0.; 3], [0.; 3]); count],
            &vec![[0.; 4]; count],
            max_bytes - 88,
        )?;
        let mut words = packed.words;
        let color_offset = packed.rays.words.len();
        words.extend(inverse.to_cols_array().map(f32::to_bits));
        words.extend(camera.eye.to_array().map(f32::to_bits));
        words.push(match camera.projection {
            crate::SceneProjection::Perspective { .. } => 0,
            crate::SceneProjection::Orthographic { .. } => 1,
        });
        words.extend([width, height]);
        Ok(Self {
            words,
            pixels: count,
            color_offset,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    pub fn workgroups(&self) -> [u32; 3] {
        [
            self.words[self.words.len() - 2].div_ceil(8),
            self.words[self.words.len() - 1].div_ceil(8),
            1,
        ]
    }
    pub fn decode(&self, bytes: &[u8]) -> Result<Vec<[f32; 4]>, ComputeError> {
        if bytes.len() != self.words.len() * 4 {
            return Err(ComputeError::InvalidBuffer);
        }
        for ray in 0..self.pixels {
            let base = (12 + self.words[3] as usize + 8 * ray + 6) * 4;
            let read = |offset| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let tau = read(base);
            let transmission = read(base + 4);
            if !tau.is_finite()
                || tau < 0.
                || !transmission.is_finite()
                || !(0. ..=1.).contains(&transmission)
                || (transmission - (-tau).exp()).abs() > 1e-5
            {
                return Err(ComputeError::InvalidBuffer);
            }
        }
        let end = self.color_offset * 4 + self.pixels * 16;
        let colors = bytes[self.color_offset * 4..end]
            .chunks_exact(16)
            .map(|pixel| {
                std::array::from_fn(|k| {
                    f32::from_le_bytes(pixel[k * 4..k * 4 + 4].try_into().unwrap())
                })
            })
            .collect::<Vec<[f32; 4]>>();
        if colors
            .iter()
            .any(|c| c.iter().any(|v| !v.is_finite() || *v < 0.) || c[3] > 1.)
        {
            return Err(ComputeError::InvalidBuffer);
        }
        Ok(colors)
    }
}
pub const DROPLET_EXTINCTION_SCENE_SHADER: &str = concat!(
    include_str!("droplet_extinction_common.wgsl"),
    include_str!("droplet_extinction_scene.wgsl")
);
