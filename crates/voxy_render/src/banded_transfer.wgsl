// Bit-exact transport between resident factors and compact RHS/status words.
@group(0) @binding(0) var<storage,read_write> resident:array<u32>;
@group(0) @binding(1) var<storage,read_write> compact:array<u32>;
fn resident_word(index:u32)->u32 {
    let n=resident[1];let stride=2u*n+1u;
    let system=(index-4u)/stride;let local=(index-4u)%stride;
    return 4u+system*(20u*n+1u)+18u*n+local;
}
@compute @workgroup_size(64)
fn gather(@builtin(global_invocation_id) id:vec3<u32>) {
    let index=id.x;
    if index>=arrayLength(&compact) {return;}
    if index<4u {compact[index]=resident[index];return;}
    compact[index]=resident[resident_word(index)];
}
@compute @workgroup_size(64)
fn upload(@builtin(global_invocation_id) id:vec3<u32>) {
    let index=id.x;
    if index<4u || index>=arrayLength(&compact) {return;}
    resident[resident_word(index)]=compact[index];
}
