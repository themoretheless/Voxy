// Participating-medium species transport ABI shared with voxy_gpu/medium_solver.wgsl.
// Words: [0] cells, [1..4] shape, [4] substeps, [5] h, [6..9] spacing, [9..12] up,
// [12] settling, [13] diffusivity, [14] volume, [15] parity (written by the host),
// then velocity (3 per cell), mass_a (cells), mass_b (cells), settled (cells).
// Each launch performs one substep from mass[parity] into mass[1 - parity].
__device__ float word_f(const unsigned int* data, unsigned int index) {
    return __uint_as_float(data[index]);
}
__device__ unsigned int stride_of(const unsigned int* data, unsigned int axis) {
    if (axis == 0u) return 1u;
    if (axis == 1u) return data[1];
    return data[1] * data[2];
}
__device__ float face_flux(const unsigned int* data, const float* velocity, const float* mass,
                           unsigned int lower, unsigned int upper, unsigned int axis) {
    const float volume = word_f(data, 14);
    const float h = word_f(data, 5);
    const float spacing = word_f(data, 6u + axis);
    const float face_area = volume / spacing;
    const float face_v = 0.5f * (velocity[lower * 3u + axis] + velocity[upper * 3u + axis])
        - word_f(data, 12) * word_f(data, 9u + axis);
    float flux = face_v >= 0.0f ? face_v * mass[lower] / volume : face_v * mass[upper] / volume;
    flux = flux * face_area * h;
    const float diffusivity = word_f(data, 13);
    if (diffusivity > 0.0f) {
        const float d = diffusivity * (mass[upper] - mass[lower]) / volume / spacing * face_area * h;
        flux = flux - d;
    }
    return flux;
}
extern "C" __global__ void medium_substep(unsigned int* data) {
    const unsigned int cells = data[0];
    const unsigned int cell = blockIdx.x * blockDim.x + threadIdx.x;
    if (cell >= cells) return;
    const unsigned int parity = data[15];
    const float* velocity = (const float*)(data + 16u);
    const float* mass = (const float*)(data + 16u + cells * 3u + parity * cells);
    float* out = (float*)(data + 16u + cells * 3u + (1u - parity) * cells);
    float* settled = (float*)(data + 16u + cells * 5u);
    const unsigned int shape[3] = { data[1], data[2], data[3] };
    const unsigned int ijk[3] = { cell % shape[0], (cell / shape[0]) % shape[1], cell / (shape[0] * shape[1]) };
    const float settling = word_f(data, 12);
    const float volume = word_f(data, 14);
    const float h = word_f(data, 5);
    float next = mass[cell];
    float out_settled = 0.0f;
    for (unsigned int axis = 0; axis < 3; ++axis) {
        const unsigned int n = shape[axis];
        if (n < 2u && settling == 0.0f) continue;
        const unsigned int index = ijk[axis];
        const unsigned int stride = stride_of(data, axis);
        if (index + 1u < n) next -= face_flux(data, velocity, mass, cell, cell + stride, axis);
        if (index > 0u) next += face_flux(data, velocity, mass, cell - stride, cell, axis);
        const float up = word_f(data, 9u + axis);
        if (settling > 0.0f && up != 0.0f) {
            const bool outflow_face = up > 0.0f ? index == 0u : index + 1u == n;
            if (outflow_face) {
                const float face_area = volume / word_f(data, 6u + axis);
                const float o = settling * fabsf(up) * mass[cell] / volume * face_area * h;
                next -= o;
                out_settled += o;
            }
        }
    }
    out[cell] = next > 0.0f ? next : 0.0f;
    settled[cell] += out_settled;
}
