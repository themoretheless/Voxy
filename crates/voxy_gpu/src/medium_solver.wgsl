// Participating-medium species transport, one substep per dispatch.
// Gather formulation: every cell evaluates the six face fluxes of the
// previous mass field with the same expression its neighbour uses, so the
// scheme conserves mass exactly without atomics. Mirrors
// `physics::medium::ParticipatingMedium::advance` (advective upwind with the
// face velocity = mean gas velocity - settling along up, explicit diffusion,
// settling outflow through the bottom face).

struct Config {
    shape: vec3<u32>,
    cells: u32,
    spacing: vec3<f32>,
    h: f32,
    up: vec3<f32>,
    settling: f32,
    diffusivity: f32,
    volume: f32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<uniform> config: Config;
@group(0) @binding(1) var<storage, read> velocity: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> mass_in: array<f32>;
@group(0) @binding(3) var<storage, read_write> mass_out: array<f32>;
@group(0) @binding(4) var<storage, read_write> settled: array<f32>;

fn index_of(ijk: vec3<u32>) -> u32 {
    return ijk.x + config.shape.x * (ijk.y + config.shape.y * ijk.z);
}

// Flux through the face between `lower` and `upper = lower + stride(axis)`,
// positive from lower to upper, in kilograms per substep.
fn face_flux(lower: u32, upper: u32, axis: u32) -> f32 {
    let face_area = config.volume / config.spacing[axis];
    let face_v = 0.5 * (velocity[lower][axis] + velocity[upper][axis])
        - config.settling * config.up[axis];
    var flux: f32;
    if (face_v >= 0.0) {
        flux = face_v * mass_in[lower] / config.volume;
    } else {
        flux = face_v * mass_in[upper] / config.volume;
    }
    flux = flux * face_area * config.h;
    if (config.diffusivity > 0.0) {
        let d = config.diffusivity * (mass_in[upper] - mass_in[lower]) / config.volume
            / config.spacing[axis] * face_area * config.h;
        flux = flux - d;
    }
    return flux;
}

@compute @workgroup_size(64)
fn cs_medium_substep(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let cell = global_id.x;
    if (cell >= config.cells) {
        return;
    }
    let ijk = vec3<u32>(
        cell % config.shape.x,
        (cell / config.shape.x) % config.shape.y,
        cell / (config.shape.x * config.shape.y),
    );
    var next = mass_in[cell];
    var out_settled = 0.0;
    for (var axis = 0u; axis < 3u; axis = axis + 1u) {
        let n = config.shape[axis];
        if (n < 2u && config.settling == 0.0) {
            continue;
        }
        let index = ijk[axis];
        var stride = 1u;
        if (axis == 1u) {
            stride = config.shape.x;
        } else if (axis == 2u) {
            stride = config.shape.x * config.shape.y;
        }
        if (index + 1u < n) {
            next = next - face_flux(cell, cell + stride, axis);
        }
        if (index > 0u) {
            next = next + face_flux(cell - stride, cell, axis);
        }
        if (config.settling > 0.0 && config.up[axis] != 0.0) {
            var outflow_face = false;
            if (config.up[axis] > 0.0) {
                outflow_face = index == 0u;
            } else {
                outflow_face = index + 1u == n;
            }
            if (outflow_face) {
                let face_area = config.volume / config.spacing[axis];
                let out = config.settling * abs(config.up[axis]) * mass_in[cell] / config.volume
                    * face_area * config.h;
                next = next - out;
                out_settled = out_settled + out;
            }
        }
    }
    mass_out[cell] = max(next, 0.0);
    settled[cell] = settled[cell] + out_settled;
}
