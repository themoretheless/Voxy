// Exact procedural_terrain v1. u64 hash arithmetic uses (low, high) u32 limbs,
// preserving all seed bits and signed i64 lattice coordinates on baseline WGSL.
@group(0) @binding(0) var<storage, read_write> words: array<u32>;
fn add64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    let lo = a.x + b.x;
    return vec2<u32>(lo, a.y + b.y + select(0u, 1u, lo < a.x));
}
fn shr64(a: vec2<u32>, n: u32) -> vec2<u32> {
    return vec2<u32>((a.x >> n) | (a.y << (32u - n)), a.y >> n);
}
fn high_mul(a: u32, b: u32) -> u32 {
    let a0 = a & 65535u; let a1 = a >> 16u;
    let b0 = b & 65535u; let b1 = b >> 16u;
    let w0 = a0 * b0;
    let t = a1 * b0 + (w0 >> 16u);
    let w1 = (t & 65535u) + a0 * b1;
    return a1 * b1 + (t >> 16u) + (w1 >> 16u);
}
fn mul64(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(a.x * b.x, high_mul(a.x, b.x) + a.x * b.y + a.y * b.x);
}
fn mix64(value: vec2<u32>) -> vec2<u32> {
    var n = add64(value, vec2<u32>(0x7f4a7c15u, 0x9e3779b9u));
    n = mul64(n ^ shr64(n, 30u), vec2<u32>(0x1ce4e5b9u, 0xbf58476du));
    n = mul64(n ^ shr64(n, 27u), vec2<u32>(0x133111ebu, 0x94d049bbu));
    return n ^ shr64(n, 31u);
}
fn smooth_integer(remainder: u32, period: u32) -> u32 {
    let t = remainder * 65536u / period;
    return (t * t / 65536u) * (196608u - 2u * t) / 65536u;
}
fn lerp_exact(a: u32, b: u32, t: u32) -> u32 {
    // Signed CPU division truncates toward zero; unsigned magnitude avoids
    // overflowing i32 products while retaining exactly that rounding.
    if b >= a { return a + (b - a) * t / 65536u; }
    return a - (a - b) * t / 65536u;
}
fn sample(cx: vec2<u32>, cz: vec2<u32>, seed: vec2<u32>, dx: u32, dz: u32) -> u32 {
    let a = add64(cx, vec2<u32>(dx, 0u));
    let b = add64(cz, vec2<u32>(dz, 0u));
    return mix64(seed ^ mix64(a) ^ mix64(b ^ vec2<u32>(0x517cc1b7u, 0u))).x & 65535u;
}
fn noise(base: u32, period: u32, seed: vec2<u32>) -> u32 {
    let cx = vec2<u32>(words[base], words[base + 1u]);
    let cz = vec2<u32>(words[base + 2u], words[base + 3u]);
    let tx = smooth_integer(words[base + 4u], period);
    let tz = smooth_integer(words[base + 5u], period);
    return lerp_exact(
        lerp_exact(sample(cx, cz, seed, 0u, 0u), sample(cx, cz, seed, 1u, 0u), tx),
        lerp_exact(sample(cx, cz, seed, 0u, 1u), sample(cx, cz, seed, 1u, 1u), tx), tz);
}
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let column = id.x;
    if column >= 1024u { return; }
    let base = 12u + column * 26u;
    let seed = vec2<u32>(words[0], words[1]);
    let continent = noise(base, 128u, seed);
    let mountain_noise = noise(base + 6u, 192u, seed ^ vec2<u32>(0x92a7u, 0u));
    let mountains = (max(mountain_noise, 32768u) - 32768u) * 2u;
    let detail = noise(base + 12u, 32u, seed ^ vec2<u32>(0x7fc3u, 0u));
    let fine = noise(base + 18u, 8u, seed ^ vec2<u32>(0xc421u, 0u));
    let product = mul64(vec2<u32>(mountains, 0u), vec2<u32>(detail, 0u));
    // floor(mountains * detail * 16 / 65536), retaining the low remainder.
    let relief = (product.x >> 12u) | (product.y << 20u);
    let height = (continent * 10u + relief + detail * 4u + fine) / 65536u;
    var biome = 1u;
    if height < 6u { biome = 0u; }
    else if mountains > 65536u / 3u { biome = 2u; }
    words[base + 24u] = height;
    words[base + 25u] = biome;
    for (var y = 0u; y < 32u; y++) {
        let world_y = bitcast<i32>(words[2]) + i32(y);
        var block = words[4]; // air
        if world_y > i32(height) {
            if world_y <= 6 { block = words[8]; } // water
        } else if biome == 2u && height > 18u { block = words[7]; }
        else if world_y == i32(height) && biome != 0u { block = words[5]; }
        else if world_y >= i32(height) - 3 { block = words[6]; }
        else { block = words[7]; }
        words[26636u + column + y * 1024u] = block;
    }
}
