struct Parameters { count:u32, controls:u32, pad0:u32, pad1:u32 }
struct Weight { control:u32, weight:f32 }
@group(0) @binding(0) var<uniform> parameters:Parameters;
@group(0) @binding(1) var<storage,read> rest:array<f32>;
@group(0) @binding(2) var<storage,read> rows:array<u32>;
@group(0) @binding(3) var<storage,read> weights:array<Weight>;
@group(0) @binding(4) var<storage,read> controls:array<vec4f>;
@group(0) @binding(5) var<storage,read_write> output:array<f32>;
@compute @workgroup_size(64)
fn deform(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;
    if i>=parameters.count { return; }
    var delta=vec3f(0.0);
    for(var j=rows[i]; j<rows[i+1u]; j++) {
        let w=weights[j];
        delta+=w.weight*controls[w.control].xyz;
    }
    // SceneVertex is a packed 36-byte vertex stream, not a WGSL vec3 struct.
    let base=9u*i;
    for(var k=0u; k<9u; k++) { output[base+k]=rest[base+k]; }
    output[base]+=delta.x;
    output[base+1u]+=delta.y;
    output[base+2u]+=delta.z;
}
