// Exact procedural_terrain v1. Shared packed ABI with voxy_gpu's WGSL workload.
// All accesses are fixed-size and each invocation owns one column and its voxels.
typedef unsigned int U32;
typedef unsigned long long U64;
typedef long long I64;
__device__ U64 terrain_mix(U64 n) {
    n += 0x9e3779b97f4a7c15ULL;
    n = (n ^ (n >> 30)) * 0xbf58476d1ce4e5b9ULL;
    n = (n ^ (n >> 27)) * 0x94d049bb133111ebULL;
    return n ^ (n >> 31);
}
__device__ U64 terrain_pair(const U32* words) {
    return U64(words[0]) | (U64(words[1]) << 32);
}
__device__ I64 terrain_smooth(U32 remainder, U32 period) {
    I64 t = I64(remainder) * 65536 / period;
    return t * t / 65536 * (3 * 65536 - 2 * t) / 65536;
}
__device__ I64 terrain_sample(U64 x, U64 z, U64 seed, U32 dx, U32 dz) {
    return I64(terrain_mix(seed ^ terrain_mix(x + dx) ^ terrain_mix((z + dz) ^ 0x517cc1b7ULL)) & 0xffffULL);
}
__device__ I64 terrain_lerp(I64 a, I64 b, I64 t) {
    return a + (b - a) * t / 65536;
}
__device__ I64 terrain_noise(const U32* input, U32 period, U64 seed) {
    U64 x = terrain_pair(input), z = terrain_pair(input + 2);
    I64 tx = terrain_smooth(input[4], period), tz = terrain_smooth(input[5], period);
    return terrain_lerp(
        terrain_lerp(terrain_sample(x,z,seed,0,0), terrain_sample(x,z,seed,1,0), tx),
        terrain_lerp(terrain_sample(x,z,seed,0,1), terrain_sample(x,z,seed,1,1), tx), tz);
}
extern "C" __global__ void procedural_terrain(U32* words) {
    U32 column = blockIdx.x * blockDim.x + threadIdx.x;
    if (column >= 1024) return;
    U32 base = 12 + column * 26;
    U64 seed = terrain_pair(words);
    I64 continent = terrain_noise(words + base, 128, seed);
    I64 mountain_noise = terrain_noise(words + base + 6, 192, seed ^ 0x92a7ULL);
    I64 mountains = mountain_noise > 32768 ? (mountain_noise - 32768) * 2 : 0;
    I64 detail = terrain_noise(words + base + 12, 32, seed ^ 0x7fc3ULL);
    I64 fine = terrain_noise(words + base + 18, 8, seed ^ 0xc421ULL);
    I64 height = (continent * 10 + mountains * detail * 16 / 65536 + detail * 4 + fine) / 65536;
    U32 biome = height < 6 ? 0 : (mountains > 65536 / 3 ? 2 : 1);
    words[base + 24] = U32(height);
    words[base + 25] = biome;
    // Header y is a validated signed value in [-64, 32], encoded as u32 bits.
    I64 origin_y = words[2] <= 32 ? I64(words[2]) : I64(words[2]) - (1LL << 32);
    for (U32 y = 0; y < 32; ++y) {
        I64 world_y = origin_y + y;
        U32 block = words[4];
        if (world_y > height) { if (world_y <= 6) block = words[8]; }
        else if (biome == 2 && height > 18) block = words[7];
        else if (world_y == height && biome != 0) block = words[5];
        else if (world_y >= height - 3) block = words[6];
        else block = words[7];
        words[26636 + column + y * 1024] = block;
    }
}
