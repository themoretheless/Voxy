// Header: node count, active count, down limit, horizontal limit, sample budget,
// error, sample count, write budget. Node: amount, initial amount, visited,
// below/-X/+X/-Z/+Z indices. Active indices follow all nodes.
@group(0) @binding(0) var<storage, read_write> data: array<u32>;
fn read_amount(index: u32) -> u32 {
    // CPU computes coordinates before requesting a sample.
    if index == 4294967294u { data[5] = 4u; return 9u; }
    if index >= data[0] {
        data[6] += 1u;
        data[5] = select(1u, 2u, data[6] > data[4]);
        return 9u;
    }
    let base = 8u + index * 8u;
    if data[base + 2u] == 0u {
        data[6] += 1u;
        if data[6] > data[4] { data[5] = 2u; return 9u; }
        data[base + 2u] = 1u;
    }
    if data[base] == 10u { data[5] = 6u; data[6] = index; return 9u; }
    if data[base] == 11u { data[5] = 5u; data[6] = index; return 9u; }
    return data[base];
}
@compute @workgroup_size(1)
fn cs_main() {
    let active_base = 8u + data[0] * 8u;
    for (var work_index = 0u; work_index < data[1]; work_index += 1u) {
        let index = data[active_base + work_index];
        var amount = read_amount(index);
        if data[5] != 0u { return; }
        if amount == 0u || amount == 9u { continue; }
        let base = 8u + index * 8u;
        let below = data[base + 3u];
        let below_amount = read_amount(below);
        if data[5] != 0u { return; }
        if below_amount != 9u {
            let transfer = min(min(amount, 8u - below_amount), data[2]);
            amount -= transfer;
            data[base] = amount;
            data[8u + below * 8u] = below_amount + transfer;
        }
        if amount == 0u { continue; }
        var horizontal = 0u;
        for (var direction = 0u; direction < 4u; direction += 1u) {
            let neighbor = data[base + 4u + direction];
            let neighbor_amount = read_amount(neighbor);
            if data[5] != 0u { return; }
            if neighbor_amount != 9u && horizontal < data[3] && amount > 1u && neighbor_amount + 1u < amount {
                horizontal += 1u;
                amount -= 1u;
                data[base] = amount;
                data[8u + neighbor * 8u] = neighbor_amount + 1u;
            }
        }
    }
    var writes = 0u;
    for (var index = 0u; index < data[0]; index += 1u) {
        let base = 8u + index * 8u;
        if data[base] != data[base + 1u] { writes += 1u; }
    }
    if writes > data[7] { data[5] = 3u; }
}
