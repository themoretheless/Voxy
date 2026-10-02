// Ordered integer water graph ABI shared with voxy_gpu/water.wgsl.
// One thread preserves canonical CPU transfer order. No world writes occur here;
// the caller must validate status/read provenance before publishing a transaction.
__device__ unsigned int water_read(unsigned int* data, unsigned int index) {
    if (index == 4294967294u) { data[5] = 4u; return 9u; }
    if (index >= data[0]) {
        ++data[6];
        data[5] = data[6] > data[4] ? 2u : 1u;
        return 9u;
    }
    const unsigned int base = 8u + index * 8u;
    if (data[base + 2u] == 0u) {
        ++data[6];
        if (data[6] > data[4]) { data[5] = 2u; return 9u; }
        data[base + 2u] = 1u;
    }
    if (data[base] == 10u) { data[5] = 6u; data[6] = index; return 9u; }
    if (data[base] == 11u) { data[5] = 5u; data[6] = index; return 9u; }
    return data[base];
}
extern "C" __global__ void water_transfer(unsigned int* data) {
    if (blockIdx.x != 0 || blockIdx.y != 0 || blockIdx.z != 0 ||
        threadIdx.x != 0 || threadIdx.y != 0 || threadIdx.z != 0) return;
    const unsigned int active_base = 8u + data[0] * 8u;
    for (unsigned int work = 0; work < data[1]; ++work) {
        const unsigned int index = data[active_base + work];
        unsigned int amount = water_read(data, index);
        if (data[5] != 0u) return;
        if (amount == 0u || amount == 9u) continue;
        const unsigned int base = 8u + index * 8u;
        const unsigned int below = data[base + 3u];
        const unsigned int below_amount = water_read(data, below);
        if (data[5] != 0u) return;
        if (below_amount != 9u) {
            unsigned int transfer = amount;
            if (transfer > 8u - below_amount) transfer = 8u - below_amount;
            if (transfer > data[2]) transfer = data[2];
            amount -= transfer;
            data[base] = amount;
            data[8u + below * 8u] = below_amount + transfer;
        }
        if (amount == 0u) continue;
        unsigned int horizontal = 0;
        for (unsigned int direction = 0; direction < 4; ++direction) {
            const unsigned int neighbor = data[base + 4u + direction];
            const unsigned int neighbor_amount = water_read(data, neighbor);
            if (data[5] != 0u) return;
            if (neighbor_amount != 9u && horizontal < data[3] && amount > 1u && neighbor_amount + 1u < amount) {
                ++horizontal; --amount;
                data[base] = amount;
                data[8u + neighbor * 8u] = neighbor_amount + 1u;
            }
        }
    }
    unsigned int writes = 0;
    for (unsigned int index = 0; index < data[0]; ++index) {
        const unsigned int base = 8u + index * 8u;
        if (data[base] != data[base + 1u]) ++writes;
    }
    if (writes > data[7]) data[5] = 3u;
}
