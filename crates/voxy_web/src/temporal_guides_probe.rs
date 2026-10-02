//! Numeric readback of production raster correspondence against CPU barycentrics.
use super::browser::error;
use glam::{Mat4, Vec2, Vec3};
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub(crate) struct GuideProbe {
    dispatch: Option<voxy_render::ComputeDispatch>,
    pending: Option<voxy_render::PendingComputeReadback>,
    expected: Vec<[f32; 3]>,
}
impl GuideProbe {
    pub(crate) fn encode(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        motion: &wgpu::Texture,
        depth: &wgpu::Texture,
        cameras: [Mat4; 2],
        vertices: &[voxy_render::PreviousPositionVertex],
        reset: bool,
    ) -> Result<Self, JsValue> {
        let size = [motion.width(), motion.height()];
        let pixels = sample_pixels(size, cameras[0], vertices, 2.0);
        let byte_count = (pixels.len() * 512) as u64;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("production temporal guide probe"),
            size: byte_count,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut expected = Vec::new();
        for pixel in pixels {
            let uv = Vec2::new(
                (pixel[0] as f32 + 0.5) / size[0] as f32,
                (pixel[1] as f32 + 0.5) / size[1] as f32,
            );
            expected.push(reference(cameras, vertices, uv, reset));
            let index = expected.len() - 1;
            for (slot, texture) in [motion, depth].into_iter().enumerate() {
                encoder.copy_texture_to_buffer(
                    wgpu::TexelCopyTextureInfo {
                        texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: pixel[0],
                            y: pixel[1],
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: (index * 512 + slot * 256) as u64,
                            bytes_per_row: Some(256),
                            rows_per_image: Some(1),
                        },
                    },
                    wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        let dispatch =
            voxy_render::ComputeDispatch::copy_buffer(device, encoder, &buffer, 0, byte_count)
                .map_err(error)?;
        Ok(Self {
            dispatch: Some(dispatch),
            pending: None,
            expected,
        })
    }
    pub(crate) fn begin_read(&mut self) {
        if let Some(dispatch) = self.dispatch.take() {
            self.pending = Some(dispatch.begin_read());
        }
    }
    pub(crate) fn poll(&mut self) -> Result<Option<(u32, u32)>, JsValue> {
        let Some(pending) = &mut self.pending else {
            return Ok(None);
        };
        let Some(bytes) = pending.try_read().map_err(error)? else {
            return Ok(None);
        };
        for (pixel, expected) in self.expected.iter().enumerate() {
            let offset = pixel * 512;
            let actual = [
                decode_half(u16::from_le_bytes(
                    bytes[offset..offset + 2].try_into().map_err(error)?,
                )),
                decode_half(u16::from_le_bytes(
                    bytes[offset + 2..offset + 4].try_into().map_err(error)?,
                )),
                f32::from_le_bytes(
                    bytes[offset + 256..offset + 260]
                        .try_into()
                        .map_err(error)?,
                ),
            ];
            for channel in 0..3 {
                if !actual[channel].is_finite()
                    || (actual[channel] - expected[channel]).abs() > 0.00005
                {
                    return Err(error(format!(
                        "production temporal guide pixel {pixel} channel {channel}: {} != {}",
                        actual[channel], expected[channel]
                    )));
                }
            }
        }
        self.pending = None;
        let moving = self
            .expected
            .iter()
            .filter(|value| value[0].abs().max(value[1].abs()) > 0.00005)
            .count() as u32;
        Ok(Some(((self.expected.len() * 3) as u32, moving)))
    }
}
fn decode_half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = (bits >> 10) & 31;
    let fraction = f32::from(bits & 1023);
    match exponent {
        0 => sign * fraction * 2.0_f32.powi(-24),
        31 => {
            if fraction == 0.0 {
                sign * f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => sign * (1.0 + fraction / 1024.0) * 2.0_f32.powi(i32::from(exponent) - 15),
    }
}
fn reference(
    cameras: [Mat4; 2],
    vertices: &[voxy_render::PreviousPositionVertex],
    uv: Vec2,
    reset: bool,
) -> [f32; 3] {
    let point = Vec2::new(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let mut nearest = 1.0;
    let mut result = [0.0; 3];
    for triangle in vertices.chunks_exact(3) {
        let clip = std::array::from_fn::<_, 3, _>(|i| {
            cameras[0] * Vec3::from_array(triangle[i].current).extend(1.0)
        });
        if clip.iter().any(|v| v.w <= 0.0) {
            continue;
        }
        let screen = clip.map(|v| v.truncate().truncate() / v.w);
        let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
        let area = cross(screen[1] - screen[0], screen[2] - screen[0]);
        if area.abs() < 0.000001 {
            continue;
        }
        let weights = [
            cross(screen[1] - point, screen[2] - point) / area,
            cross(screen[2] - point, screen[0] - point) / area,
            cross(screen[0] - point, screen[1] - point) / area,
        ];
        if weights.iter().any(|v| *v < -0.000001) {
            continue;
        }
        let current_depth: f32 = (0..3).map(|i| weights[i] * clip[i].z / clip[i].w).sum();
        if current_depth <= 0.0 || current_depth >= nearest {
            continue;
        }
        nearest = current_depth;
        let denominator: f32 = (0..3).map(|i| weights[i] / clip[i].w).sum();
        let previous: Vec3 = (0..3)
            .map(|i| {
                Vec3::from_array(triangle[i].previous) * (weights[i] / clip[i].w / denominator)
            })
            .sum();
        let projected = cameras[1] * previous.extend(1.0);
        let depth = projected.z / projected.w;
        if projected.w <= 0.0 || !(0.0..1.0).contains(&depth) {
            result = [0.0; 3];
            continue;
        }
        let backward = Vec2::new(
            (projected.x / projected.w + 1.0) * 0.5,
            (1.0 - projected.y / projected.w) * 0.5,
        ) - uv;
        result = [
            if reset { 0.0 } else { backward.x },
            if reset { 0.0 } else { backward.y },
            depth,
        ];
    }
    result
}

pub(crate) fn sample_pixels(
    size: [u32; 2],
    camera: Mat4,
    vertices: &[voxy_render::PreviousPositionVertex],
    edge_offset: f32,
) -> Vec<[u32; 2]> {
    let mut pixels = Vec::new();
    for row in 1..=3 {
        for column in 1..=3 {
            pixels.push([size[0] * column / 4, size[1] * row / 4]);
        }
    }
    // The first six vertices are the animated quad; remaining triangles are
    // the static receiver. Probe two pixels to either side of each mesh edge.
    for triangle in vertices
        .iter()
        .take(6)
        .copied()
        .collect::<Vec<_>>()
        .chunks_exact(3)
    {
        let projected = std::array::from_fn::<_, 3, _>(|i| {
            let clip = camera * Vec3::from_array(triangle[i].current).extend(1.0);
            Vec2::new(
                (clip.x / clip.w + 1.0) * size[0] as f32 * 0.5,
                (1.0 - clip.y / clip.w) * size[1] as f32 * 0.5,
            )
        });
        for edge in 0..3 {
            let a = projected[edge];
            let b = projected[(edge + 1) % 3];
            let delta = b - a;
            if !delta.is_finite() || delta.length_squared() < 1.0 {
                continue;
            }
            let normal = Vec2::new(-delta.y, delta.x).normalize() * edge_offset;
            for point in [(a + b) * 0.5 + normal, (a + b) * 0.5 - normal] {
                if point.x >= 0.0
                    && point.y >= 0.0
                    && point.x < size[0] as f32
                    && point.y < size[1] as f32
                {
                    pixels.push([point.x.floor() as u32, point.y.floor() as u32]);
                }
            }
        }
    }
    pixels
}
