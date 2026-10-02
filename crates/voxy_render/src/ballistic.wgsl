// Constant-acceleration ballistic integration. This pass handles independent
// bodies only; collisions and mutual forces require separate ordered passes.
struct Body {
    position: vec4<f32>,
    velocity: vec4<f32>,
}
struct State {
    // xyz acceleration, w positive fixed timestep
    acceleration_dt: vec4<f32>,
    bodies: array<Body>,
}
@group(0) @binding(0) var<storage, read_write> state: State;
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= arrayLength(&state.bodies) { return; }
    let dt = state.acceleration_dt.w;
    let a = state.acceleration_dt.xyz;
    let p = state.bodies[id.x].position.xyz;
    let v = state.bodies[id.x].velocity.xyz;
    state.bodies[id.x].position = vec4<f32>(p + v * dt + a * (0.5 * dt * dt), state.bodies[id.x].position.w);
    state.bodies[id.x].velocity = vec4<f32>(v + a * dt, state.bodies[id.x].velocity.w);
}
