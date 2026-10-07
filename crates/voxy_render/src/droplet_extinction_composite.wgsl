@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x>=words[4] { return; }
    let base=12u+words[3]+8u*id.x;
    let tau=optical_depth(id.x);
    let transmission=exp(-tau);
    words[base+6u]=bitcast<u32>(tau);
    words[base+7u]=bitcast<u32>(transmission);
    let color=12u+words[3]+8u*words[4]+4u*id.x;
    for (var channel=0u; channel<3u; channel+=1u) {
        words[color+channel]=bitcast<u32>(value(color+channel)*transmission);
    }
}
