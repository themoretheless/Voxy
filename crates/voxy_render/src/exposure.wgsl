struct Settings { dimensions: vec4<u32>, values: vec4<f32>, rates: vec4<f32> }
struct Partial { sum: f32, count: u32 }
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(5) var second_eye: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> partials: array<Partial>;
@group(0) @binding(2) var<uniform> settings: Settings;
@group(0) @binding(3) var<uniform> previous: vec4<f32>;
@group(0) @binding(4) var<storage, read_write> result: vec4<f32>;
var<workgroup> sums: array<f32, 64>;
var<workgroup> counts: array<u32, 64>;
fn reduce(local: u32) {
    workgroupBarrier();
    for (var step = 32u; step > 0u; step /= 2u) {
        if local < step { sums[local] += sums[local + step]; counts[local] += counts[local + step]; }
        workgroupBarrier();
    }
}
fn sample_luminance(rgb: vec3<f32>) -> Partial {
    if all((bitcast<vec3<u32>>(rgb) & vec3<u32>(0x7f800000u)) != vec3<u32>(0x7f800000u)) {
        let luminance = dot(max(rgb, vec3(0.0)), vec3(0.2126, 0.7152, 0.0722));
        if luminance > 0.0 { return Partial(log2(clamp(luminance, 0.000001, 1000000.0)), 1u); }
    }
    return Partial(0.0, 0u);
}
@compute @workgroup_size(8, 8)
fn meter(@builtin(global_invocation_id) pixel: vec3<u32>, @builtin(local_invocation_index) local: u32, @builtin(workgroup_id) tile: vec3<u32>) {
    var sum = 0.0;
    var count = 0u;
    if all(pixel.xy < settings.dimensions.xy) {
        let primary = sample_luminance(textureLoad(source, vec2<i32>(pixel.xy), 0).rgb);
        sum = primary.sum; count = primary.count;
        if settings.rates.z > 1.0 {
            let secondary = sample_luminance(textureLoad(second_eye, vec2<i32>(pixel.xy), 0).rgb);
            sum += secondary.sum; count += secondary.count;
        }
    }
    sums[local] = sum; counts[local] = count;
    reduce(local);
    if local == 0u { partials[tile.y * settings.dimensions.z + tile.x] = Partial(sums[0], counts[0]); }
}
@compute @workgroup_size(64)
fn adapt(@builtin(local_invocation_index) local: u32) {
    var sum = 0.0;
    var count = 0u;
    for (var tile = local; tile < settings.dimensions.w; tile += 64u) {
        sum += partials[tile].sum; count += partials[tile].count;
    }
    sums[local] = sum; counts[local] = count;
    reduce(local);
    if local == 0u {
        let old = clamp(previous.x, settings.values.y, settings.values.z);
        var desired = old;
        if counts[0] > 0u {
            desired = clamp(settings.values.x / exp2(sums[0] / f32(counts[0])), settings.values.y, settings.values.z);
        }
        var exposure = desired;
        if settings.rates.w > 0.0 {
            let rate = select(settings.rates.y, settings.rates.x, desired > old);
            let weight = 1.0 - exp(-rate * settings.values.w);
            exposure = mix(old, desired, weight);
        }
        result = vec4(clamp(exposure, settings.values.y, settings.values.z), 0.0, 0.0, 0.0);
    }
}
