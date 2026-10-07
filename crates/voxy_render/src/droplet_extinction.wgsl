@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=words[4] { return; }
    let base=12u+words[3]+8u*id.x;
    let tau=optical_depth(id.x);
    words[base+6u]=bitcast<u32>(tau);
    words[base+7u]=bitcast<u32>(exp(-tau));
}
