// Header: X,Y,Z,query count,cell count. Cells use canonical x/y/z order.
// Each query has inclusive min/max XYZ and count/first-solid/first-fault outputs.
@group(0) @binding(0) var<storage, read_write> data: array<u32>;
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) invocation: vec3<u32>) {
    if invocation.x >= data[3] { return; }
    let base = 5u + data[4] + invocation.x * 9u;
    var count = 0u;
    var first_solid = 4294967295u;
    var first_fault = 4294967295u;
    for (var x = data[base]; x <= data[base + 3u]; x += 1u) {
        for (var y = data[base + 1u]; y <= data[base + 4u]; y += 1u) {
            for (var z = data[base + 2u]; z <= data[base + 5u]; z += 1u) {
                let index = (x * data[1] + y) * data[2] + z;
                let classification = data[5u + index];
                if classification == 1u {
                    count += 1u;
                    first_solid = min(first_solid, index);
                }
                if classification >= 2u { first_fault = min(first_fault, index); }
            }
        }
    }
    data[base + 6u] = count;
    data[base + 7u] = first_solid;
    data[base + 8u] = first_fault;
}
