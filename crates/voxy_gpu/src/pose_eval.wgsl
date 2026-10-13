// GPU pose evaluation: one invocation per blend job samples two resampled
// clips at a shared phase, blends them, walks the parent-first hierarchy and
// writes an instance-major skin palette. Arithmetic order matches
// `voxy_animation::gpu_layout::cpu_reference_palette`.

struct Config {
    job_count: u32,
    joint_count: u32,
    _pad0: u32,
    _pad1: u32,
};

struct ClipHeader {
    key_offset: u32,
    sample_count: u32,
    joint_count: u32,
    looping: u32,
    duration: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct Job {
    instance: u32,
    clip_a: u32,
    clip_b: u32,
    phase: f32,
    weight: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<uniform> config: Config;
@group(0) @binding(1) var<storage, read> headers: array<ClipHeader>;
@group(0) @binding(2) var<storage, read> keys: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> jobs: array<Job>;
@group(0) @binding(4) var<storage, read> parents: array<u32>;
@group(0) @binding(5) var<storage, read> inverse_binds: array<mat4x4<f32>>;
@group(0) @binding(6) var<storage, read_write> palettes: array<mat4x4<f32>>;

const NO_PARENT: u32 = 0xffffffffu;
const MAX_JOINTS: u32 = 256u;

var<private> globals: array<mat4x4<f32>, 256>;

fn nlerp(a: vec4<f32>, b_in: vec4<f32>, t: f32) -> vec4<f32> {
    var b = b_in;
    if (dot(a, b) < 0.0) {
        b = -b;
    }
    let x = a.x + (b.x - a.x) * t;
    let y = a.y + (b.y - a.y) * t;
    let z = a.z + (b.z - a.z) * t;
    let w = a.w + (b.w - a.w) * t;
    let len = sqrt(x * x + y * y + z * z + w * w);
    return vec4<f32>(x / len, y / len, z / len, w / len);
}

struct Trs {
    translation: vec3<f32>,
    rotation: vec4<f32>,
    scale: vec3<f32>,
};

fn sample_clip(header: ClipHeader, phase_in: f32, joint: u32) -> Trs {
    let phase = clamp(phase_in, 0.0, 1.0);
    let scaled = phase * f32(header.sample_count - 1u);
    let lower = min(u32(floor(scaled)), header.sample_count - 1u);
    let upper = min(lower + 1u, header.sample_count - 1u);
    let t = scaled - f32(lower);
    let ia = header.key_offset + (lower * header.joint_count + joint) * 3u;
    let ib = header.key_offset + (upper * header.joint_count + joint) * 3u;
    let ta = keys[ia].xyz;
    let ra = keys[ia + 1u];
    let sa = keys[ia + 2u].xyz;
    let tb = keys[ib].xyz;
    let rb = keys[ib + 1u];
    let sb = keys[ib + 2u].xyz;
    var out: Trs;
    out.translation = ta + (tb - ta) * t;
    out.rotation = nlerp(ra, rb, t);
    out.scale = sa + (sb - sa) * t;
    return out;
}

fn trs_matrix(trs: Trs) -> mat4x4<f32> {
    let q = trs.rotation;
    let x2 = q.x + q.x;
    let y2 = q.y + q.y;
    let z2 = q.z + q.z;
    let xx = q.x * x2;
    let xy = q.x * y2;
    let xz = q.x * z2;
    let yy = q.y * y2;
    let yz = q.y * z2;
    let zz = q.z * z2;
    let wx = q.w * x2;
    let wy = q.w * y2;
    let wz = q.w * z2;
    let c0 = vec4<f32>(1.0 - (yy + zz), xy + wz, xz - wy, 0.0) * trs.scale.x;
    let c1 = vec4<f32>(xy - wz, 1.0 - (xx + zz), yz + wx, 0.0) * trs.scale.y;
    let c2 = vec4<f32>(xz + wy, yz - wx, 1.0 - (xx + yy), 0.0) * trs.scale.z;
    let c3 = vec4<f32>(trs.translation, 1.0);
    return mat4x4<f32>(c0, c1, c2, c3);
}

@compute @workgroup_size(64)
fn cs_pose_eval(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= config.job_count) {
        return;
    }
    let job = jobs[index];
    let a = headers[job.clip_a];
    let b = headers[job.clip_b];
    let weight = clamp(job.weight, 0.0, 1.0);
    let joint_count = min(config.joint_count, MAX_JOINTS);
    let base = job.instance * config.joint_count;
    for (var joint = 0u; joint < joint_count; joint = joint + 1u) {
        let sa = sample_clip(a, job.phase, joint);
        let sb = sample_clip(b, job.phase, joint);
        var blended: Trs;
        blended.translation = sa.translation + (sb.translation - sa.translation) * weight;
        blended.rotation = nlerp(sa.rotation, sb.rotation, weight);
        blended.scale = sa.scale + (sb.scale - sa.scale) * weight;
        let local = trs_matrix(blended);
        let parent = parents[joint];
        var global = local;
        if (parent != NO_PARENT) {
            global = globals[parent] * local;
        }
        globals[joint] = global;
        palettes[base + joint] = global * inverse_binds[joint];
    }
}
