// Cross-platform GPU rest-material search metric.
// Storage ABI: [n, e, pins(n), weights(n), direction(3n), elements(19e), offsets(n+1), refs(4e), forces(12e), output(3n)]
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(64)
fn cs_elements(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let id = global_id.x;
    let n = bitcast<u32>(data[0]);
    let count = bitcast<u32>(data[1]);
    if (id >= count) {
        return;
    }
    let base = 2u + 5u * n + 19u * id;
    var h = mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    for (var corner = 0u; corner < 4u; corner += 1u) {
        let node = bitcast<u32>(data[base + corner]);
        if (data[2u + node] == 0.0) {
            let u = vec3<f32>(
                data[2u + 2u * n + 3u * node],
                data[2u + 2u * n + 3u * node + 1u],
                data[2u + 2u * n + 3u * node + 2u]
            );
            let g = vec3<f32>(
                data[base + 4u + 3u * corner],
                data[base + 4u + 3u * corner + 1u],
                data[base + 4u + 3u * corner + 2u]
            );
            h[0] += u * g.x;
            h[1] += u * g.y;
            h[2] += u * g.z;
        }
    }
    let trace = h[0][0] + h[1][1] + h[2][2];
    let shear = data[base + 17u];
    let bulk = data[base + 18u];
    let volume = data[base + 16u];

    var stress = mat3x3<f32>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    for (var a = 0u; a < 3u; a += 1u) {
        for (var b = 0u; b < 3u; b += 1u) {
            let sym = 0.5 * (h[b][a] + h[a][b]);
            var dev = sym;
            if (a == b) {
                dev -= trace / 3.0;
            }
            var s = 2.0 * shear * dev;
            if (a == b) {
                s += bulk * trace;
            }
            stress[b][a] = s;
        }
    }

    let offsets_base = 2u + 5u * n + 19u * count;
    let refs_base = offsets_base + n + 1u;
    let force_base = refs_base + 4u * count;

    for (var corner = 0u; corner < 4u; corner += 1u) {
        let node = bitcast<u32>(data[base + corner]);
        let g = vec3<f32>(
            data[base + 4u + 3u * corner],
            data[base + 4u + 3u * corner + 1u],
            data[base + 4u + 3u * corner + 2u]
        );
        let product = stress * g;
        let corner_force = select(volume * product, vec3<f32>(0.0), data[2u + node] != 0.0);
        let out_idx = force_base + 12u * id + 3u * corner;
        data[out_idx] = corner_force.x;
        data[out_idx + 1u] = corner_force.y;
        data[out_idx + 2u] = corner_force.z;
    }
}

@compute @workgroup_size(64)
fn cs_nodes(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node = global_id.x;
    let n = bitcast<u32>(data[0]);
    let count = bitcast<u32>(data[1]);
    if (node >= n) {
        return;
    }
    let offsets_base = 2u + 5u * n + 19u * count;
    let refs_base = offsets_base + n + 1u;
    let force_base = refs_base + 4u * count;
    let output_base = force_base + 12u * count;

    var val = vec3<f32>(0.0);
    if (data[2u + node] == 0.0) {
        let inertia = data[2u + n + node];
        let dir = vec3<f32>(
            data[2u + 2u * n + 3u * node],
            data[2u + 2u * n + 3u * node + 1u],
            data[2u + 2u * n + 3u * node + 2u]
        );
        val = inertia * dir;
        let start = bitcast<u32>(data[offsets_base + node]);
        let end = bitcast<u32>(data[offsets_base + node + 1u]);
        for (var i = start; i < end; i += 1u) {
            let ref_idx = bitcast<u32>(data[refs_base + i]);
            let f_idx = force_base + 3u * ref_idx;
            val += vec3<f32>(data[f_idx], data[f_idx + 1u], data[f_idx + 2u]);
        }
    }
    let out_idx = output_base + 3u * node;
    data[out_idx] = val.x;
    data[out_idx + 1u] = val.y;
    data[out_idx + 2u] = val.z;
}
