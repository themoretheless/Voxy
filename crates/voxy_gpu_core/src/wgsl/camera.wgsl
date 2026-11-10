// Shared camera uniform layout (std140-aligned 160 bytes).
struct SharedCameraUniform {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    viewport_size_and_inv: vec4<f32>,
}
