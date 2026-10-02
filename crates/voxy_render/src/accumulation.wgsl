struct Settings { count: u32, history: u32, pad: vec2<u32> }
@group(0) @binding(0) var current: texture_2d<f32>;
@group(0) @binding(1) var previous: texture_2d<f32>;
@group(0) @binding(2) var result: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var<uniform> settings: Settings;
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(result)) { return; }
    var value = textureLoad(current, vec2<i32>(id.xy), 0).rgb;
    let finite = (bitcast<vec3<u32>>(value) & vec3<u32>(0x7f800000u)) != vec3<u32>(0x7f800000u);
    value = select(vec3(0.0), max(value, vec3(0.0)), finite);
    if settings.history != 0u {
        let old = textureLoad(previous, vec2<i32>(id.xy), 0).rgb;
        let weight = 1.0 / f32(settings.count);
        value = old * (1.0 - weight) + value * weight;
    }
    textureStore(result, vec2<i32>(id.xy), vec4(value, 1.0));
}
