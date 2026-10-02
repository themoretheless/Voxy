// Six input doubles and six output doubles per projectile. Absolute voxel
// coordinates never enter the device ABI; each thread owns one output record.
extern "C" __global__ void projectile_motion(const double *input, double *output,
                                            unsigned count, double dt) {
    unsigned i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= count) return;
    for (unsigned axis = 0; axis < 3; ++axis) {
        double velocity = input[6 * i + axis] + input[6 * i + 3 + axis] * dt;
        output[6 * i + axis] = velocity;
        output[6 * i + 3 + axis] = velocity * dt;
    }
}
