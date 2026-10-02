extern "C" __global__ void voxel_regions(unsigned int* data) {
    const unsigned int query = blockIdx.x * blockDim.x + threadIdx.x;
    if (query >= data[3]) return;
    const unsigned int base = 5u + data[4] + query * 9u;
    unsigned int count = 0, solid = 0xffffffffu, fault = 0xffffffffu;
    for (unsigned int x = data[base]; x <= data[base + 3]; ++x)
        for (unsigned int y = data[base + 1]; y <= data[base + 4]; ++y)
            for (unsigned int z = data[base + 2]; z <= data[base + 5]; ++z) {
                const unsigned int index = (x * data[1] + y) * data[2] + z;
                const unsigned int classification = data[5 + index];
                if (classification == 1) { ++count; if (index < solid) solid = index; }
                if (classification >= 2 && index < fault) fault = index;
            }
    data[base + 6] = count;
    data[base + 7] = solid;
    data[base + 8] = fault;
}
