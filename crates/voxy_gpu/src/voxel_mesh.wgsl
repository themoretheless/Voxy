// GPU Compute Voxel Mesher with Indirect Drawing.
// Evaluates face visibility on 34x34x34 padded chunk volume and emits
// stream-compacted vertices directly into GPU indirect draw buffers.

struct IndirectArgs {
    vertex_count: atomic<u32>,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

struct VoxelMeshConfig {
    max_quads: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

struct GpuVoxelVertex {
    position: vec3<f32>,
    face_dir: u32,
    material: u32,
    ao: u32,
    uv: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> config: VoxelMeshConfig;
@group(0) @binding(1) var<storage, read> voxels: array<u32>;
@group(0) @binding(2) var<storage, read_write> indirect_args: IndirectArgs;
@group(0) @binding(3) var<storage, read_write> vertices: array<GpuVoxelVertex>;

@compute @workgroup_size(1)
fn cs_reset() {
    atomicStore(&indirect_args.vertex_count, 0u);
    indirect_args.instance_count = 1u;
    indirect_args.first_vertex = 0u;
    indirect_args.first_instance = 0u;
}

fn padded_index(x: u32, y: u32, z: u32) -> u32 {
    return x + y * 34u + z * 1156u;
}

fn sample_solid(x: u32, y: u32, z: u32) -> u32 {
    return select(0u, 1u, voxels[padded_index(x, y, z)] != 0u);
}

fn vertex_ao(s1: u32, s2: u32, c: u32) -> u32 {
    if (s1 != 0u && s2 != 0u) {
        return 0u;
    }
    return 3u - (s1 + s2 + c);
}

@compute @workgroup_size(4, 4, 4)
fn cs_mesh(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let lx = global_id.x;
    let ly = global_id.y;
    let lz = global_id.z;

    if (lx >= 32u || ly >= 32u || lz >= 32u) {
        return;
    }

    // Coordinates in 34x34x34 padded volume
    let px = lx + 1u;
    let py = ly + 1u;
    let pz = lz + 1u;

    let voxel_id = voxels[padded_index(px, py, pz)];
    if (voxel_id == 0u) {
        return;
    }

    let fx = f32(lx);
    let fy = f32(ly);
    let fz = f32(lz);

    // Direction offsets: -X, +X, -Y, +Y, -Z, +Z
    // Face 0: NegX (-X)
    if (voxels[padded_index(px - 1u, py, pz)] == 0u) {
        let ao0 = vertex_ao(sample_solid(px - 1u, py - 1u, pz), sample_solid(px - 1u, py, pz - 1u), sample_solid(px - 1u, py - 1u, pz - 1u));
        let ao1 = vertex_ao(sample_solid(px - 1u, py + 1u, pz), sample_solid(px - 1u, py, pz - 1u), sample_solid(px - 1u, py + 1u, pz - 1u));
        let ao2 = vertex_ao(sample_solid(px - 1u, py + 1u, pz), sample_solid(px - 1u, py, pz + 1u), sample_solid(px - 1u, py + 1u, pz + 1u));
        let ao3 = vertex_ao(sample_solid(px - 1u, py - 1u, pz), sample_solid(px - 1u, py, pz + 1u), sample_solid(px - 1u, py - 1u, pz + 1u));

        emit_quad(
            vec3<f32>(fx, fy, fz),
            vec3<f32>(fx, fy + 1.0, fz),
            vec3<f32>(fx, fy + 1.0, fz + 1.0),
            vec3<f32>(fx, fy, fz + 1.0),
            0u, voxel_id, ao0, ao1, ao2, ao3
        );
    }

    // Face 1: PosX (+X)
    if (voxels[padded_index(px + 1u, py, pz)] == 0u) {
        let ao0 = vertex_ao(sample_solid(px + 1u, py - 1u, pz), sample_solid(px + 1u, py, pz + 1u), sample_solid(px + 1u, py - 1u, pz + 1u));
        let ao1 = vertex_ao(sample_solid(px + 1u, py + 1u, pz), sample_solid(px + 1u, py, pz + 1u), sample_solid(px + 1u, py + 1u, pz + 1u));
        let ao2 = vertex_ao(sample_solid(px + 1u, py + 1u, pz), sample_solid(px + 1u, py, pz - 1u), sample_solid(px + 1u, py + 1u, pz - 1u));
        let ao3 = vertex_ao(sample_solid(px + 1u, py - 1u, pz), sample_solid(px + 1u, py, pz - 1u), sample_solid(px + 1u, py - 1u, pz - 1u));

        emit_quad(
            vec3<f32>(fx + 1.0, fy, fz + 1.0),
            vec3<f32>(fx + 1.0, fy + 1.0, fz + 1.0),
            vec3<f32>(fx + 1.0, fy + 1.0, fz),
            vec3<f32>(fx + 1.0, fy, fz),
            1u, voxel_id, ao0, ao1, ao2, ao3
        );
    }

    // Face 2: NegY (-Y)
    if (voxels[padded_index(px, py - 1u, pz)] == 0u) {
        let ao0 = vertex_ao(sample_solid(px - 1u, py - 1u, pz), sample_solid(px, py - 1u, pz + 1u), sample_solid(px - 1u, py - 1u, pz + 1u));
        let ao1 = vertex_ao(sample_solid(px + 1u, py - 1u, pz), sample_solid(px, py - 1u, pz + 1u), sample_solid(px + 1u, py - 1u, pz + 1u));
        let ao2 = vertex_ao(sample_solid(px + 1u, py - 1u, pz), sample_solid(px, py - 1u, pz - 1u), sample_solid(px + 1u, py - 1u, pz - 1u));
        let ao3 = vertex_ao(sample_solid(px - 1u, py - 1u, pz), sample_solid(px, py - 1u, pz - 1u), sample_solid(px - 1u, py - 1u, pz - 1u));

        emit_quad(
            vec3<f32>(fx, fy, fz + 1.0),
            vec3<f32>(fx + 1.0, fy, fz + 1.0),
            vec3<f32>(fx + 1.0, fy, fz),
            vec3<f32>(fx, fy, fz),
            2u, voxel_id, ao0, ao1, ao2, ao3
        );
    }

    // Face 3: PosY (+Y)
    if (voxels[padded_index(px, py + 1u, pz)] == 0u) {
        let ao0 = vertex_ao(sample_solid(px - 1u, py + 1u, pz), sample_solid(px, py + 1u, pz - 1u), sample_solid(px - 1u, py + 1u, pz - 1u));
        let ao1 = vertex_ao(sample_solid(px + 1u, py + 1u, pz), sample_solid(px, py + 1u, pz - 1u), sample_solid(px + 1u, py + 1u, pz - 1u));
        let ao2 = vertex_ao(sample_solid(px + 1u, py + 1u, pz), sample_solid(px, py + 1u, pz + 1u), sample_solid(px + 1u, py + 1u, pz + 1u));
        let ao3 = vertex_ao(sample_solid(px - 1u, py + 1u, pz), sample_solid(px, py + 1u, pz + 1u), sample_solid(px - 1u, py + 1u, pz + 1u));

        emit_quad(
            vec3<f32>(fx, fy + 1.0, fz),
            vec3<f32>(fx + 1.0, fy + 1.0, fz),
            vec3<f32>(fx + 1.0, fy + 1.0, fz + 1.0),
            vec3<f32>(fx, fy + 1.0, fz + 1.0),
            3u, voxel_id, ao0, ao1, ao2, ao3
        );
    }

    // Face 4: NegZ (-Z)
    if (voxels[padded_index(px, py, pz - 1u)] == 0u) {
        let ao0 = vertex_ao(sample_solid(px + 1u, py, pz - 1u), sample_solid(px, py - 1u, pz - 1u), sample_solid(px + 1u, py - 1u, pz - 1u));
        let ao1 = vertex_ao(sample_solid(px + 1u, py, pz - 1u), sample_solid(px, py + 1u, pz - 1u), sample_solid(px + 1u, py + 1u, pz - 1u));
        let ao2 = vertex_ao(sample_solid(px - 1u, py, pz - 1u), sample_solid(px, py + 1u, pz - 1u), sample_solid(px - 1u, py + 1u, pz - 1u));
        let ao3 = vertex_ao(sample_solid(px - 1u, py, pz - 1u), sample_solid(px, py - 1u, pz - 1u), sample_solid(px - 1u, py - 1u, pz - 1u));

        emit_quad(
            vec3<f32>(fx + 1.0, fy, fz),
            vec3<f32>(fx + 1.0, fy + 1.0, fz),
            vec3<f32>(fx, fy + 1.0, fz),
            vec3<f32>(fx, fy, fz),
            4u, voxel_id, ao0, ao1, ao2, ao3
        );
    }

    // Face 5: PosZ (+Z)
    if (voxels[padded_index(px, py, pz + 1u)] == 0u) {
        let ao0 = vertex_ao(sample_solid(px - 1u, py, pz + 1u), sample_solid(px, py - 1u, pz + 1u), sample_solid(px - 1u, py - 1u, pz + 1u));
        let ao1 = vertex_ao(sample_solid(px - 1u, py, pz + 1u), sample_solid(px, py + 1u, pz + 1u), sample_solid(px - 1u, py + 1u, pz + 1u));
        let ao2 = vertex_ao(sample_solid(px + 1u, py, pz + 1u), sample_solid(px, py + 1u, pz + 1u), sample_solid(px + 1u, py + 1u, pz + 1u));
        let ao3 = vertex_ao(sample_solid(px + 1u, py, pz + 1u), sample_solid(px, py - 1u, pz + 1u), sample_solid(px + 1u, py - 1u, pz + 1u));

        emit_quad(
            vec3<f32>(fx, fy, fz + 1.0),
            vec3<f32>(fx, fy + 1.0, fz + 1.0),
            vec3<f32>(fx + 1.0, fy + 1.0, fz + 1.0),
            vec3<f32>(fx + 1.0, fy, fz + 1.0),
            5u, voxel_id, ao0, ao1, ao2, ao3
        );
    }
}

fn emit_quad(
    p0: vec3<f32>,
    p1: vec3<f32>,
    p2: vec3<f32>,
    p3: vec3<f32>,
    face_dir: u32,
    material: u32,
    ao0: u32,
    ao1: u32,
    ao2: u32,
    ao3: u32
) {
    let quad_index = atomicAdd(&indirect_args.vertex_count, 6u) / 6u;
    if (quad_index >= config.max_quads) {
        return;
    }

    let base_v = quad_index * 6u;

    // Triangle 1: p0, p1, p2
    vertices[base_v + 0u] = GpuVoxelVertex(p0, face_dir, material, ao0, vec2<f32>(0.0, 0.0));
    vertices[base_v + 1u] = GpuVoxelVertex(p1, face_dir, material, ao1, vec2<f32>(0.0, 1.0));
    vertices[base_v + 2u] = GpuVoxelVertex(p2, face_dir, material, ao2, vec2<f32>(1.0, 1.0));

    // Triangle 2: p0, p2, p3
    vertices[base_v + 3u] = GpuVoxelVertex(p0, face_dir, material, ao0, vec2<f32>(0.0, 0.0));
    vertices[base_v + 4u] = GpuVoxelVertex(p2, face_dir, material, ao2, vec2<f32>(1.0, 1.0));
    vertices[base_v + 5u] = GpuVoxelVertex(p3, face_dir, material, ao3, vec2<f32>(1.0, 0.0));
}
