// Fixed f64 velocity-Verlet ABI. Three ordered kernels, private resident memory.
// Header: G, epsilon^2, dt, count, uniform xyz, atomic failure bits (8 doubles).
// Then count bodies each of 8 doubles: position xyz, mass, velocity xyz, padding;
// followed by equally sized predicted and output regions.
struct GravityBody { double position[3], mass, velocity[3]; };
__device__ bool gravity_finite(double value) {
    return (static_cast<unsigned long long>(__double_as_longlong(value)) & 0x7ff0000000000000ULL) != 0x7ff0000000000000ULL;
}
__device__ unsigned long long* gravity_failure(double* words) {
    return reinterpret_cast<unsigned long long*>(words + 7);
}
__device__ GravityBody gravity_body(const double* words, unsigned int base, unsigned int index) {
    const unsigned int offset = base + index * 8;
    GravityBody body;
    for (unsigned int k = 0; k < 3; ++k) {
        body.position[k] = words[offset + k];
        body.velocity[k] = words[offset + 4 + k];
    }
    body.mass = words[offset + 3];
    return body;
}
__device__ void gravity_store(double* words, unsigned int base, unsigned int index, const GravityBody& body) {
    const unsigned int offset = base + index * 8;
    for (unsigned int k = 0; k < 3; ++k) { words[offset+k] = body.position[k]; words[offset+4+k] = body.velocity[k]; }
    words[offset+3] = body.mass; words[offset+7] = 0.0;
}
__device__ unsigned int gravity_acceleration(const double* words, unsigned int base, unsigned int index, double* acceleration) {
    for (unsigned int k=0; k<3; ++k) acceleration[k] = words[4+k];
    if (words[0] == 0.0) return 0;
    const unsigned int count = static_cast<unsigned int>(words[3]);
    const GravityBody body = gravity_body(words,base,index);
    for (unsigned int other=0; other<count; ++other) {
        if (other == index) continue;
        const GravityBody source = gravity_body(words,base,other);
        double delta[3]; double radius_squared = 0.0;
        for (unsigned int k=0; k<3; ++k) { delta[k] = source.position[k]-body.position[k]; radius_squared += delta[k]*delta[k]; }
        radius_squared += words[1];
        if (radius_squared == 0.0) return 1;
        if (!gravity_finite(radius_squared)) return 2;
        const double scale = words[0] / radius_squared / sqrt(radius_squared);
        for (unsigned int k=0; k<3; ++k) {
            acceleration[k] += delta[k] * scale * source.mass;
            if (!gravity_finite(acceleration[k])) return 2;
        }
    }
    return 0;
}
extern "C" __global__ void gravity_predict(double* words) {
    const unsigned int index = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned int count = static_cast<unsigned int>(words[3]);
    if (index >= count || atomicAdd(gravity_failure(words),0ULL) != 0ULL) return;
    GravityBody body = gravity_body(words,8,index);
    double acceleration[3];
    const unsigned int failure = gravity_acceleration(words,8,index,acceleration);
    if (failure) { atomicMax(gravity_failure(words),static_cast<unsigned long long>(failure)); return; }
    for (unsigned int k=0; k<3; ++k) {
        body.velocity[k] += acceleration[k] * (0.5 * words[2]);
        body.position[k] += body.velocity[k] * words[2];
        if (!gravity_finite(body.position[k]) || !gravity_finite(body.velocity[k])) { atomicMax(gravity_failure(words),2ULL); return; }
    }
    gravity_store(words,8+count*8,index,body);
}
extern "C" __global__ void gravity_correct(double* words) {
    const unsigned int index = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned int count = static_cast<unsigned int>(words[3]);
    if (index >= count || atomicAdd(gravity_failure(words),0ULL) != 0ULL) return;
    GravityBody body = gravity_body(words,8+count*8,index);
    double acceleration[3];
    const unsigned int failure = gravity_acceleration(words,8+count*8,index,acceleration);
    if (failure) { atomicMax(gravity_failure(words),static_cast<unsigned long long>(failure)); return; }
    for (unsigned int k=0; k<3; ++k) {
        body.velocity[k] += acceleration[k] * (0.5 * words[2]);
        if (!gravity_finite(body.velocity[k])) { atomicMax(gravity_failure(words),2ULL); return; }
    }
    gravity_store(words,8+count*16,index,body);
}
extern "C" __global__ void gravity_commit(double* words) {
    const unsigned int index = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned int count = static_cast<unsigned int>(words[3]);
    if (index >= count || atomicAdd(gravity_failure(words),0ULL) != 0ULL) return;
    gravity_store(words,8,index,gravity_body(words,8+count*16,index));
}
// Render-only f32 view: 8 header words then 8 words per committed body.
// Validation is a separate pass so a conversion failure leaves the view intact.
extern "C" __global__ void gravity_view_validate(double* words, unsigned int* status) {
    const unsigned int index = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned int count = static_cast<unsigned int>(words[3]);
    if (index >= count) return;
    const unsigned long long failure = atomicAdd(gravity_failure(words),0ULL);
    if (failure) { atomicMax(status,static_cast<unsigned int>(failure)); return; }
    for (unsigned int k=0; k<7; ++k) {
        const float value = static_cast<float>(words[8+index*8+k]);
        if ((__float_as_uint(value) & 0x7f800000u) == 0x7f800000u || (k==3 && value<=0.0f)) {
            atomicMax(status,3u); return;
        }
    }
}
extern "C" __global__ void gravity_view_commit(const double* words, const unsigned int* status, unsigned int* view) {
    const unsigned int index = blockIdx.x * blockDim.x + threadIdx.x;
    const unsigned int count = static_cast<unsigned int>(words[3]);
    if (index >= count || *status != 0u) return;
    if (index==0) { for (unsigned int k=0; k<8; ++k) view[k] = k==3 ? count : 0u; }
    for (unsigned int k=0; k<7; ++k) view[8+index*8+k] = __float_as_uint(static_cast<float>(words[8+index*8+k]));
    view[8+index*8+7] = 0u;
}
