extern "C" __global__ void box_sweep(const double* input, double* output, unsigned int count) {
    const unsigned int id = blockIdx.x * blockDim.x + threadIdx.x;
    if (id >= count) return;
    const double* r = input + id * 15u;
    double* out = output + id * 5u;
    const double infinity = __longlong_as_double(0x7ff0000000000000LL);
    double enter = -infinity, exit = infinity;
    int axis_normal = -1, sign = 0;
    for (int axis = 0; axis < 3; ++axis) {
        const double velocity = r[6 + axis];
        if (velocity == 0.0) {
            if (r[3 + axis] <= r[9 + axis] || r[axis] >= r[12 + axis]) return;
            continue;
        }
        const double first = (r[9 + axis] - r[3 + axis]) / velocity;
        const double second = (r[12 + axis] - r[axis]) / velocity;
        const double axis_enter = fmin(first, second), axis_exit = fmax(first, second);
        if (axis_enter > enter) { enter = axis_enter; axis_normal = axis; sign = velocity > 0.0 ? -1 : 1; }
        exit = fmin(exit, axis_exit);
        if (enter > exit) return;
    }
    if (exit < 0.0 || enter > 1.0) return;
    out[0] = 1.0;
    out[1] = enter < 0.0 ? 0.0 : enter;
    if (enter >= 0.0 && axis_normal >= 0) out[2 + axis_normal] = sign;
}
