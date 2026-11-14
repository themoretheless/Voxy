use bytemuck::{Pod, Zeroable};
use glam::camera::rh::{
    proj::directx::{orthographic, perspective},
    view::look_at_mat4,
};
use glam::{Mat4, Vec3, Vec4};
use voxy_core::{CHUNK_EDGE, ChunkPos};

use super::pipelines::create_camera_bind_group;
use super::{CameraView, RendererError};
use crate::BodycamRig;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct CameraUniform {
    pub(super) view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct PhotorealCameraUniform {
    pub(super) view_proj: [[f32; 4]; 4],
    pub(super) camera_eye: [f32; 4],
    pub(super) flashlight_pos: [f32; 4],
    pub(super) flashlight_dir: [f32; 4],
    pub(super) subject_spheres: [[f32; 4]; 12],
    pub(super) viewport: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct ChunkMeta {
    pub(super) relative_origin: [i32; 4],
    pub(super) light_info: [u32; 4],
}

#[derive(Debug)]
pub(super) struct SharedCameraBindings {
    ring: crate::SharedUploadRing,
    groups: Vec<wgpu::BindGroup>,
}

impl SharedCameraBindings {
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        camera: &wgpu::Buffer,
        metadata: &wgpu::Buffer,
        light: &wgpu::Buffer,
    ) -> Option<Self> {
        let ring =
            crate::SharedUploadRing::new(device, camera.size(), wgpu::BufferUsages::UNIFORM)?;
        let groups = ring
            .buffers()
            .map(|buffer| create_camera_bind_group(device, layout, buffer, metadata, light))
            .collect();
        Some(Self { ring, groups })
    }

    pub(super) fn refresh(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        metadata: &wgpu::Buffer,
        light: &wgpu::Buffer,
    ) {
        self.groups = self
            .ring
            .buffers()
            .map(|buffer| create_camera_bind_group(device, layout, buffer, metadata, light))
            .collect();
    }

    pub(super) fn select(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        legacy: &wgpu::Buffer,
        legacy_group: &wgpu::BindGroup,
        data: &[u8],
    ) -> wgpu::BindGroup {
        let (slot, _) = self
            .ring
            .write_or_stage(encoder, legacy, |mut bytes| bytes.copy_from_slice(data))
            .expect("live shared renderer camera");
        self.groups.get(slot).unwrap_or(legacy_group).clone()
    }
}

pub(super) fn camera_uniform(width: u32, height: u32, camera: CameraView) -> CameraUniform {
    CameraUniform {
        view_proj: camera_view_projection(width, height, camera).to_cols_array_2d(),
    }
}

pub(super) fn photoreal_camera_uniform(
    width: u32,
    height: u32,
    camera: CameraView,
    rig: &BodycamRig,
    flashlight_pos: Vec3,
    flashlight_dir: Vec3,
    wetness: f32,
) -> PhotorealCameraUniform {
    PhotorealCameraUniform {
        subject_spheres: [[0.0; 4]; 12],
        viewport: [width as f32, height as f32, 0.0, 0.0],
        view_proj: perspective_camera_view_projection(width, height, camera).to_cols_array_2d(),
        camera_eye: [
            camera.eye.x,
            camera.eye.y,
            camera.eye.z,
            rig.elapsed_seconds,
        ],
        flashlight_pos: [
            flashlight_pos.x,
            flashlight_pos.y,
            flashlight_pos.z,
            if rig.flashlight_enabled { 1.0 } else { 0.0 },
        ],
        flashlight_dir: [
            flashlight_dir.x,
            flashlight_dir.y,
            flashlight_dir.z,
            wetness,
        ],
    }
}

pub(super) fn perspective_camera_view_projection(
    width: u32,
    height: u32,
    camera: CameraView,
) -> Mat4 {
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let fov = camera.vertical_fov_radians.clamp(0.15, 2.65);
    let near = camera.near_plane.clamp(0.02, 10.0);
    // Reverse-Z perspective projection: near maps to z=1, far (512.0) maps to z=0.
    let projection = perspective(fov, aspect, 512.0, near);
    let view = look_at_mat4(camera.eye, camera.target, camera.up);
    projection * view
}

pub(super) fn camera_view_projection(width: u32, height: u32, camera: CameraView) -> Mat4 {
    let [half_width, half_height] = camera_half_extents(width, height, camera);
    // Swap near/far to retain the renderer's reverse-Z depth convention.
    let projection = orthographic(
        -half_width,
        half_width,
        -half_height,
        half_height,
        512.0,
        camera.near_plane,
    );
    let view = look_at_mat4(camera.eye, camera.target, camera.up);
    projection * view
}

pub(super) fn camera_half_extents(width: u32, height: u32, camera: CameraView) -> [f32; 2] {
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let distance = camera.eye.distance(camera.target);
    let half_height = (distance * (camera.vertical_fov_radians * 0.5).tan()).max(1.0);
    let half_width = half_height * aspect;
    [half_width, half_height]
}

pub(super) fn chunk_intersects_frustum(view_projection: Mat4, relative_origin: [i32; 4]) -> bool {
    let min = Vec3::new(
        relative_origin[0] as f32,
        relative_origin[1] as f32,
        relative_origin[2] as f32,
    );
    let max = min + Vec3::splat(CHUNK_EDGE as f32);
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ]
    .map(|corner| view_projection * corner.extend(1.0));
    let planes: [fn(Vec4) -> f32; 6] = [
        |point| point.x + point.w,
        |point| point.w - point.x,
        |point| point.y + point.w,
        |point| point.w - point.y,
        |point| point.z,
        |point| point.w - point.z,
    ];
    planes
        .into_iter()
        .all(|plane| corners.iter().any(|&corner| plane(corner) >= 0.0))
}

pub(super) fn relative_chunk_origin(
    pos: ChunkPos,
    anchor: ChunkPos,
) -> Result<[i32; 4], RendererError> {
    fn component(value: i64, anchor: i64) -> Result<i32, RendererError> {
        value
            .checked_sub(anchor)
            .and_then(|delta| delta.checked_mul(CHUNK_EDGE))
            .and_then(|voxels| i32::try_from(voxels).ok())
            .ok_or(RendererError::OriginOutOfRange)
    }
    Ok([
        component(pos.x, anchor.x)?,
        component(pos.y, anchor.y)?,
        component(pos.z, anchor.z)?,
        0,
    ])
}

pub(super) fn pack_light_words(
    light: Option<&[u8]>,
    output: &mut Vec<u32>,
) -> Result<(), RendererError> {
    let bytes = light.unwrap_or(&[]);
    if !bytes.is_empty() && bytes.len() != voxy_core::CHUNK_VOLUME {
        return Err(RendererError::InvalidLightVolume(bytes.len()));
    }
    output.reserve(voxy_core::CHUNK_VOLUME / 4);
    if bytes.is_empty() {
        output.resize(
            output.len() + voxy_core::CHUNK_VOLUME / 4,
            u32::from_le_bytes([0xf0; 4]),
        );
    } else {
        output.extend(
            bytes
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])),
        );
    }
    Ok(())
}
