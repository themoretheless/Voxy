// Query entry using the shared admitted optical geometry kernel.
@compute @workgroup_size(64) fn cs_main(@builtin(global_invocation_id) id:vec3u) {
    if id.x>=words[1] {return;}
    let r=words[2]+8u*id.x;let out=words[3]+16u*id.x;
    for(var k=0u;k<16u;k+=1u) {words[out+k]=0u;}
    words[out+7u]=2u;
    let origin=vector(r);let direction=normalize(vector(r+4u));let minimum=scalar(r+7u);
    let hit=optical_geometry_hit(origin,direction,minimum,scalar(r+3u));
    words[out+15u]=hit.error;
    if !hit.found {return;}
    let base=4u+24u*hit.index;
    words[out]=bitcast<u32>(hit.distance);
    for(var k=0u;k<3u;k+=1u) {
        words[out+1u+k]=bitcast<u32>(hit.position[k]);words[out+4u+k]=bitcast<u32>(hit.normal[k]);
        words[out+12u+k]=words[base+16u+k];
    }
    words[out+7u]=words[base+20u];words[out+8u]=hit.index;words[out+9u]=words[base+23u];
    words[out+10u]=select(words[base+21u],words[base+22u],hit.entering);
    words[out+11u]=select(words[base+22u],words[base+21u],hit.entering);
}
