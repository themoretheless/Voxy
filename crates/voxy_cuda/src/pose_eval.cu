// Crowd skeletal pose evaluation CUDA kernel sharing the exact TRS sampling,
// shortest-path quaternion nlerp, parent-first hierarchy traversal, and
// skin-palette multiplication order of `voxy_gpu/src/pose_eval.wgsl` and
// `voxy_animation::gpu_layout::cpu_reference_palette`.
//
// Packed word ABI:
// - Header (8 words):
//   [0] job_count
//   [1] joint_count (<= 256)
//   [2] clip_count
//   [3] total_key_vec4s
//   [4] instance_count
//   [5..8] reserved (0)
// - Clip headers: clip_count * 8 words
//   [key_offset (in vec4 units), sample_count, joint_count, looping, duration_bits, 0, 0, 0]
// - Keys: total_key_vec4s * 4 words (3 vec4s = 12 floats per key)
// - Jobs: job_count * 8 words
//   [instance, clip_a, clip_b, phase_bits, weight_bits, 0, 0, 0]
// - Parents: joint_count words (0xffffffffu = root)
// - Inverse binds: joint_count * 16 words (column-major 4x4 f32)
// - Output palettes: instance_count * joint_count * 16 words (column-major 4x4 f32)

#define NO_PARENT 0xffffffffu
#define MAX_JOINTS 256u

struct Vec3 {
    float x, y, z;
};

struct Vec4 {
    float x, y, z, w;
};

struct Mat4 {
    float m[16];
};

struct Trs {
    Vec3 translation;
    Vec4 rotation;
    Vec3 scale;
};

__device__ float word_as_f32(unsigned int w) {
    return __uint_as_float(w);
}

__device__ unsigned int f32_as_word(float v) {
    return __float_as_uint(v);
}

__device__ float clamp01(float x) {
    return x < 0.0f ? 0.0f : (x > 1.0f ? 1.0f : x);
}

__device__ Vec4 quat_nlerp(Vec4 a, Vec4 b_in, float t) {
    Vec4 b = b_in;
    const float dot = a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
    if (dot < 0.0f) {
        b.x = -b.x;
        b.y = -b.y;
        b.z = -b.z;
        b.w = -b.w;
    }
    const float x = a.x + (b.x - a.x) * t;
    const float y = a.y + (b.y - a.y) * t;
    const float z = a.z + (b.z - a.z) * t;
    const float w = a.w + (b.w - a.w) * t;
    const float len = sqrtf(x * x + y * y + z * z + w * w);
    Vec4 out = { x / len, y / len, z / len, w / len };
    return out;
}

__device__ Trs sample_clip(
    const unsigned int* header,
    const unsigned int* keys_words,
    float phase_in,
    unsigned int joint
) {
    const unsigned int key_offset = header[0];
    const unsigned int sample_count = header[1];
    const unsigned int joint_count = header[2];
    const float phase = clamp01(phase_in);
    const float scaled = phase * (float)(sample_count - 1u);
    const unsigned int floored = (unsigned int)floorf(scaled);
    const unsigned int lower = floored < (sample_count - 1u) ? floored : (sample_count - 1u);
    const unsigned int upper = (lower + 1u) < (sample_count - 1u) ? (lower + 1u) : (sample_count - 1u);
    const float t = scaled - (float)lower;

    const unsigned int ia = (key_offset + (lower * joint_count + joint) * 3u) * 4u;
    const unsigned int ib = (key_offset + (upper * joint_count + joint) * 3u) * 4u;

    const Vec3 ta = { word_as_f32(keys_words[ia + 0u]), word_as_f32(keys_words[ia + 1u]), word_as_f32(keys_words[ia + 2u]) };
    const Vec4 ra = { word_as_f32(keys_words[ia + 4u]), word_as_f32(keys_words[ia + 5u]), word_as_f32(keys_words[ia + 6u]), word_as_f32(keys_words[ia + 7u]) };
    const Vec3 sa = { word_as_f32(keys_words[ia + 8u]), word_as_f32(keys_words[ia + 9u]), word_as_f32(keys_words[ia + 10u]) };

    const Vec3 tb = { word_as_f32(keys_words[ib + 0u]), word_as_f32(keys_words[ib + 1u]), word_as_f32(keys_words[ib + 2u]) };
    const Vec4 rb = { word_as_f32(keys_words[ib + 4u]), word_as_f32(keys_words[ib + 5u]), word_as_f32(keys_words[ib + 6u]), word_as_f32(keys_words[ib + 7u]) };
    const Vec3 sb = { word_as_f32(keys_words[ib + 8u]), word_as_f32(keys_words[ib + 9u]), word_as_f32(keys_words[ib + 10u]) };

    Trs out;
    out.translation.x = ta.x + (tb.x - ta.x) * t;
    out.translation.y = ta.y + (tb.y - ta.y) * t;
    out.translation.z = ta.z + (tb.z - ta.z) * t;
    out.rotation = quat_nlerp(ra, rb, t);
    out.scale.x = sa.x + (sb.x - sa.x) * t;
    out.scale.y = sa.y + (sb.y - sa.y) * t;
    out.scale.z = sa.z + (sb.z - sa.z) * t;
    return out;
}

__device__ Mat4 trs_matrix(Trs trs) {
    const Vec4 q = trs.rotation;
    const float x2 = q.x + q.x;
    const float y2 = q.y + q.y;
    const float z2 = q.z + q.z;
    const float xx = q.x * x2;
    const float xy = q.x * y2;
    const float xz = q.x * z2;
    const float yy = q.y * y2;
    const float yz = q.y * z2;
    const float zz = q.z * z2;
    const float wx = q.w * x2;
    const float wy = q.w * y2;
    const float wz = q.w * z2;

    Mat4 out;
    out.m[0]  = (1.0f - (yy + zz)) * trs.scale.x;
    out.m[1]  = (xy + wz) * trs.scale.x;
    out.m[2]  = (xz - wy) * trs.scale.x;
    out.m[3]  = 0.0f;

    out.m[4]  = (xy - wz) * trs.scale.y;
    out.m[5]  = (1.0f - (xx + zz)) * trs.scale.y;
    out.m[6]  = (yz + wx) * trs.scale.y;
    out.m[7]  = 0.0f;

    out.m[8]  = (xz + wy) * trs.scale.z;
    out.m[9]  = (yz - wx) * trs.scale.z;
    out.m[10] = (1.0f - (xx + yy)) * trs.scale.z;
    out.m[11] = 0.0f;

    out.m[12] = trs.translation.x;
    out.m[13] = trs.translation.y;
    out.m[14] = trs.translation.z;
    out.m[15] = 1.0f;
    return out;
}

__device__ Mat4 mat4_mul(Mat4 a, Mat4 b) {
    Mat4 out;
    for (unsigned int col = 0u; col < 4u; ++col) {
        const float bx = b.m[col * 4u + 0u];
        const float by = b.m[col * 4u + 1u];
        const float bz = b.m[col * 4u + 2u];
        const float bw = b.m[col * 4u + 3u];
        for (unsigned int row = 0u; row < 4u; ++row) {
            out.m[col * 4u + row] =
                a.m[0u * 4u + row] * bx +
                a.m[1u * 4u + row] * by +
                a.m[2u * 4u + row] * bz +
                a.m[3u * 4u + row] * bw;
        }
    }
    return out;
}

extern "C" __global__ void pose_eval_crowd(unsigned int* data) {
    const unsigned int job_count = data[0];
    const unsigned int joint_count = data[1];
    const unsigned int clip_count = data[2];
    const unsigned int total_key_vec4s = data[3];

    const unsigned int index = blockIdx.x * blockDim.x + threadIdx.x;
    if (index >= job_count) {
        return;
    }

    const unsigned int headers_base = 8u;
    const unsigned int keys_base = headers_base + clip_count * 8u;
    const unsigned int jobs_base = keys_base + total_key_vec4s * 4u;
    const unsigned int parents_base = jobs_base + job_count * 8u;
    const unsigned int inv_binds_base = parents_base + joint_count;
    const unsigned int palettes_base = inv_binds_base + joint_count * 16u;

    const unsigned int* job = data + jobs_base + index * 8u;
    const unsigned int instance = job[0];
    const unsigned int clip_a = job[1];
    const unsigned int clip_b = job[2];
    const float phase = word_as_f32(job[3]);
    const float weight = clamp01(word_as_f32(job[4]));

    const unsigned int* header_a = data + headers_base + clip_a * 8u;
    const unsigned int* header_b = data + headers_base + clip_b * 8u;
    const unsigned int* keys_words = data + keys_base;
    const unsigned int* parents = data + parents_base;
    const unsigned int* inv_binds_words = data + inv_binds_base;
    unsigned int* palettes_words = data + palettes_base + (instance * joint_count) * 16u;

    Mat4 globals[MAX_JOINTS];
    const unsigned int count = joint_count < MAX_JOINTS ? joint_count : MAX_JOINTS;
    for (unsigned int joint = 0u; joint < count; ++joint) {
        const Trs sa = sample_clip(header_a, keys_words, phase, joint);
        const Trs sb = sample_clip(header_b, keys_words, phase, joint);
        Trs blended;
        blended.translation.x = sa.translation.x + (sb.translation.x - sa.translation.x) * weight;
        blended.translation.y = sa.translation.y + (sb.translation.y - sa.translation.y) * weight;
        blended.translation.z = sa.translation.z + (sb.translation.z - sa.translation.z) * weight;
        blended.rotation = quat_nlerp(sa.rotation, sb.rotation, weight);
        blended.scale.x = sa.scale.x + (sb.scale.x - sa.scale.x) * weight;
        blended.scale.y = sa.scale.y + (sb.scale.y - sa.scale.y) * weight;
        blended.scale.z = sa.scale.z + (sb.scale.z - sa.scale.z) * weight;

        const Mat4 local = trs_matrix(blended);
        const unsigned int parent = parents[joint];
        const Mat4 global = (parent == NO_PARENT) ? local : mat4_mul(globals[parent], local);
        globals[joint] = global;

        Mat4 inv_bind;
        for (unsigned int k = 0u; k < 16u; ++k) {
            inv_bind.m[k] = word_as_f32(inv_binds_words[joint * 16u + k]);
        }
        const Mat4 skin = mat4_mul(global, inv_bind);
        for (unsigned int k = 0u; k < 16u; ++k) {
            palettes_words[joint * 16u + k] = f32_as_word(skin.m[k]);
        }
    }
}
