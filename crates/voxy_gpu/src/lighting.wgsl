// GPU Cellular Automata Voxel Lighting Shader.
// Replaces serial BFS with 15-step 3D stencil propagation across 34x34x34 padded volume.

struct LightingConfig {
    preserve_direct_down: u32,
    sky_from_above: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> config: LightingConfig;
@group(0) @binding(1) var<storage, read> opaque: array<u32>;
@group(0) @binding(2) var<storage, read_write> light_in: array<u32>;
@group(0) @binding(3) var<storage, read_write> light_out: array<u32>;
@group(0) @binding(4) var<storage, read_write> packed_output: array<u32>;

fn padded_index(x: u32, y: u32, z: u32) -> u32 {
    return x + y * 34u + z * 1156u;
}

@compute @workgroup_size(8, 8, 1)
fn cs_seed_sky(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let z = global_id.y;
    if (x >= 34u || z >= 34u) {
        return;
    }

    if (config.sky_from_above == 0u) {
        for (var y = 0u; y < 34u; y += 1u) {
            light_in[padded_index(x, y, z)] = 0u;
        }
        return;
    }

    var level = 15u;
    for (var i = 0u; i < 34u; i += 1u) {
        let y = 33u - i;
        let idx = padded_index(x, y, z);
        if (opaque[idx] != 0u) {
            level = 0u;
        }
        light_in[idx] = level;
    }
}

@compute @workgroup_size(4, 4, 4)
fn cs_step_light(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let z = global_id.z;
    if (x >= 34u || y >= 34u || z >= 34u) {
        return;
    }

    let idx = padded_index(x, y, z);
    if (opaque[idx] != 0u) {
        light_out[idx] = 0u;
        return;
    }

    var max_val = light_in[idx];

    // Down neighbor (above us in Y -> shines down onto us)
    if (y < 33u) {
        let above_val = light_in[padded_index(x, y + 1u, z)];
        if (config.preserve_direct_down != 0u && above_val == 15u) {
            max_val = max(max_val, 15u);
        } else if (above_val > 1u) {
            max_val = max(max_val, above_val - 1u);
        }
    }

    // Up neighbor (below us in Y)
    if (y > 0u) {
        let below_val = light_in[padded_index(x, y - 1u, z)];
        if (below_val > 1u) {
            max_val = max(max_val, below_val - 1u);
        }
    }

    // Left neighbor
    if (x > 0u) {
        let left_val = light_in[padded_index(x - 1u, y, z)];
        if (left_val > 1u) {
            max_val = max(max_val, left_val - 1u);
        }
    }

    // Right neighbor
    if (x < 33u) {
        let right_val = light_in[padded_index(x + 1u, y, z)];
        if (right_val > 1u) {
            max_val = max(max_val, right_val - 1u);
        }
    }

    // Back neighbor
    if (z > 0u) {
        let back_val = light_in[padded_index(x, y, z - 1u)];
        if (back_val > 1u) {
            max_val = max(max_val, back_val - 1u);
        }
    }

    // Forward neighbor
    if (z < 33u) {
        let fwd_val = light_in[padded_index(x, y, z + 1u)];
        if (fwd_val > 1u) {
            max_val = max(max_val, fwd_val - 1u);
        }
    }

    light_out[idx] = max_val;
}

@compute @workgroup_size(4, 4, 4)
fn cs_pack(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let lx = global_id.x;
    let ly = global_id.y;
    let lz = global_id.z;
    if (lx >= 32u || ly >= 32u || lz >= 32u) {
        return;
    }

    let local_raw = lx + 32u * (lz + 32u * ly);
    let px = lx + 1u;
    let py = ly + 1u;
    let pz = lz + 1u;
    let p_idx = padded_index(px, py, pz);

    // light_in contains sky, light_out contains block light
    let sky = light_in[p_idx] & 0x0fu;
    let blk = light_out[p_idx] & 0x0fu;
    let byte_val = (sky << 4u) | blk;

    // Packed output packs 4 voxels per u32 word
    let word_idx = local_raw >> 2u;
    let byte_offset = (local_raw & 3u) * 8u;

    // atomic or direct write: word index is partitioned by local_raw % 4
    // We can write to a byte array directly or bytecast
    let mask = 0xffu << byte_offset;
    // We write into byte buffer
    packed_output[local_raw] = byte_val;
}
