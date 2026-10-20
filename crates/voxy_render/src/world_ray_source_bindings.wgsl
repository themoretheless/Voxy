@group(0) @binding(1) var<storage,read> exact_vertices:array<u32>;
@group(0) @binding(2) var<storage,read> exact_indices:array<u32>;
@group(0) @binding(3) var<storage,read> exact_instances:array<u32>;
@group(0) @binding(4) var<storage,read> exact_layout:array<vec4<u32>>;
@group(0) @binding(5) var<storage,read_write> exact_output:array<vec4<u32>>;
