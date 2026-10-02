// Resident f32 velocity-Verlet. Immutable input -> predicted -> output -> commit.
// Failure is sticky; commit publishes nothing when any invocation fails.
struct State {
    constant: f32,
    softening_squared: f32,
    dt: f32,
    count: u32,
    uniform_acceleration: vec3<f32>,
    failure: atomic<u32>,
    words: array<u32>,
}
@group(0) @binding(0) var<storage, read_write> state: State;
struct Body { position_mass: vec4<f32>, velocity: vec4<f32> }
struct Acceleration { value: vec3<f32>, failure: u32 }
fn finite(v: vec3<f32>) -> bool {
    return all((bitcast<vec3<u32>>(v) & vec3<u32>(0x7f800000u)) != vec3<u32>(0x7f800000u));
}
fn read_body(base: u32, index: u32) -> Body {
    let b = base + index * 8u;
    return Body(bitcast<vec4<f32>>(vec4<u32>(state.words[b],state.words[b+1u],state.words[b+2u],state.words[b+3u])),
                bitcast<vec4<f32>>(vec4<u32>(state.words[b+4u],state.words[b+5u],state.words[b+6u],state.words[b+7u])));
}
fn write_body(base: u32, index: u32, body: Body) {
    let b = base + index * 8u;
    let p = bitcast<vec4<u32>>(body.position_mass);
    let v = bitcast<vec4<u32>>(body.velocity);
    for (var k=0u; k<4u; k++) { state.words[b+k] = p[k]; state.words[b+4u+k] = v[k]; }
}
fn accelerations(base: u32, index: u32) -> Acceleration {
    var a = state.uniform_acceleration;
    if state.constant == 0.0 { return Acceleration(a,0u); }
    let body = read_body(base,index);
    for (var other=0u; other<state.count; other++) {
        if other == index { continue; }
        let source = read_body(base,other);
        let delta = source.position_mass.xyz - body.position_mass.xyz;
        let r2 = dot(delta,delta) + state.softening_squared;
        if r2 == 0.0 { return Acceleration(a,1u); }
        if (bitcast<u32>(r2) & 0x7f800000u) == 0x7f800000u { return Acceleration(a,2u); }
        let scale = state.constant / r2 / sqrt(r2);
        a += delta * scale * source.position_mass.w;
        if !finite(a) { return Acceleration(a,2u); }
    }
    return Acceleration(a,0u);
}
@compute @workgroup_size(64)
fn predict(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= state.count || atomicLoad(&state.failure) != 0u { return; }
    let body = read_body(0u,id.x);
    let acceleration = accelerations(0u,id.x);
    if acceleration.failure != 0u { atomicMax(&state.failure,acceleration.failure); return; }
    let velocity = body.velocity.xyz + acceleration.value * (0.5 * state.dt);
    let position = body.position_mass.xyz + velocity * state.dt;
    if !finite(position) || !finite(velocity) { atomicMax(&state.failure,2u); return; }
    write_body(state.count*8u,id.x,Body(vec4<f32>(position,body.position_mass.w),vec4<f32>(velocity,0.0)));
}
@compute @workgroup_size(64)
fn correct(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= state.count || atomicLoad(&state.failure) != 0u { return; }
    let body = read_body(state.count*8u,id.x);
    let acceleration = accelerations(state.count*8u,id.x);
    if acceleration.failure != 0u { atomicMax(&state.failure,acceleration.failure); return; }
    let velocity = body.velocity.xyz + acceleration.value * (0.5 * state.dt);
    if !finite(velocity) { atomicMax(&state.failure,2u); return; }
    write_body(state.count*16u,id.x,Body(body.position_mass,vec4<f32>(velocity,0.0)));
}
@compute @workgroup_size(64)
fn commit(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= state.count || atomicLoad(&state.failure) != 0u { return; }
    write_body(0u,id.x,read_body(state.count*16u,id.x));
}
