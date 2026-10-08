// GPU Compute Vertex Morph / Blendshape Deformer
// Computes ellipsoidal C1-compact support blendshapes, directional shifts,
// and head-space coordinate transformations in parallel on GPU.

struct MorphConfig {
    vertex_count: u32,
    control_count: u32,
    _pad0: u32,
    _pad1: u32,
    head_inverse: mat4x4<f32>,
    head_transform: mat4x4<f32>,
};

struct GpuMorphControl {
    center: vec3<f32>,
    amount: f32,
    radius: vec3<f32>,
    mode: u32,
    axis: u32,
    flags: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<uniform> config: MorphConfig;
@group(0) @binding(1) var<storage, read> in_positions: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> controls: array<GpuMorphControl>;
@group(0) @binding(3) var<storage, read_write> out_positions: array<vec4<f32>>;

fn smooth_step_cubic(t: f32) -> f32 {
    let clamped = clamp(t, 0.0, 1.0);
    return clamped * clamped * (3.0 - 2.0 * clamped);
}

@compute @workgroup_size(64)
fn cs_morph(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= config.vertex_count) {
        return;
    }

    let input_pos = in_positions[index];
    let point = (config.head_inverse * vec4<f32>(input_pos.xyz, 1.0)).xyz;

    // Face boundary filter: vertices below neck/jaw (y < 0.575) do not deform
    if (point.y < 0.575) {
        out_positions[index] = input_pos;
        return;
    }

    var delta = vec3<f32>(0.0, 0.0, 0.0);

    for (var i = 0u; i < config.control_count; i = i + 1u) {
        let ctrl = controls[i];
        let flags = ctrl.flags;

        // Side filtering:
        // bit 1: left only (point.x <= 0 skip)
        // bit 2: right only (point.x >= 0 skip)
        if ((flags & 2u) != 0u && point.x <= 0.0) {
            continue;
        }
        if ((flags & 4u) != 0u && point.x >= 0.0) {
            continue;
        }

        var center = ctrl.center;
        if ((flags & 1u) != 0u && center.x != 0.0) {
            center.x = select(center.x, -center.x, point.x < 0.0);
        }

        let local = point - center;
        let norm_local = local / ctrl.radius;
        let dist_sq = dot(norm_local, norm_local);

        if (dist_sq >= 1.0) {
            continue;
        }

        // Compact C1 support: (1 - d^2)^2
        let weight = (1.0 - dist_sq) * (1.0 - dist_sq);

        var change: f32 = 0.0;
        if (ctrl.mode == 0u) {
            // Scale mode along axis
            change = local[ctrl.axis] * ctrl.amount;
        } else if (ctrl.mode == 4u) {
            // Negative translate
            let sign = select(1.0, -1.0, ctrl.axis == 0u && point.x < 0.0);
            change = ctrl.amount * -0.001 * sign;
        } else {
            // Standard translate (mode 1 or other)
            let sign = select(1.0, -1.0, ctrl.axis == 0u && point.x < 0.0);
            change = ctrl.amount * 0.001 * sign;
        }

        var side_weight: f32 = 1.0;
        if ((flags & 6u) != 0u) {
            side_weight = smooth_step_cubic(abs(point.x) / 0.005);
        }

        var feature_weight: f32 = 1.0;
        if ((flags & 8u) != 0u) {
            // Nose taper
            feature_weight = smooth_step_cubic((point.y - 0.653) / 0.010);
        }

        let total_weight = weight * side_weight * feature_weight;
        delta[ctrl.axis] = delta[ctrl.axis] + change * total_weight;
    }

    let displaced_local = point + delta;
    let final_world = (config.head_transform * vec4<f32>(displaced_local, 1.0)).xyz;
    out_positions[index] = vec4<f32>(final_world, input_pos.w);
}
