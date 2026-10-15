// Experimental locally refined physical-coordinate region. No production
// table admission is implied; every node must already be producer-qualified.
struct GgxEnergyPatch {
    view_nodes:array<f32,6>,
    roughness_nodes:array<f32,6>,
    moments:array<vec4<f32>,36>,
}
fn interpolate_ggx_energy_patch(region:GgxEnergyPatch,uv:vec2<f32>) -> vec4<f32> {
    if !(uv.x>=0.0 && uv.x<=1.0 && uv.y>0.0 && uv.y<=1.0){return vec4<f32>(0.0);}
    for(var i=0u;i<6u;i++) {
        if !(region.view_nodes[i]>=0.0 && region.view_nodes[i]<=1.0
            && region.roughness_nodes[i]>0.0 && region.roughness_nodes[i]<=1.0){return vec4<f32>(0.0);}
        if i>0u && !(region.view_nodes[i]>region.view_nodes[i-1u]
            && region.roughness_nodes[i]>region.roughness_nodes[i-1u]){return vec4<f32>(0.0);}
    }
    if !(uv.x>=region.view_nodes[0] && uv.x<=region.view_nodes[5]
        && uv.y>=region.roughness_nodes[0] && uv.y<=region.roughness_nodes[5]){return vec4<f32>(0.0);}
    var wx:array<f32,6>;var wy:array<f32,6>;
    for(var i=0u;i<6u;i++) {
        wx[i]=1.0;wy[i]=1.0;
        for(var j=0u;j<6u;j++) {if j!=i {
            wx[i]*=(uv.x-region.view_nodes[j])/(region.view_nodes[i]-region.view_nodes[j]);
            wy[i]*=(uv.y-region.roughness_nodes[j])/(region.roughness_nodes[i]-region.roughness_nodes[j]);
        }}
    }
    var sum=vec3<f32>(0.0);var correction=vec3<f32>(0.0);
    for(var y=0u;y<6u;y++){for(var x=0u;x<6u;x++) {
        let node=region.moments[y*6u+x];
        if node.w!=1.0 || !all(node.xyz>=vec3<f32>(0.0)) || !all(node.xyz<=vec3<f32>(1.0))
            || abs(node.x+node.y+node.z-1.0)>0.000005{return vec4<f32>(0.0);}
        let c=1.0-region.view_nodes[x];let c2=c*c;
        let r2=region.roughness_nodes[y]*region.roughness_nodes[y];let r4=r2*r2;
        if !(r4>0.0){return vec4<f32>(0.0);}
        let term=vec3<f32>(node.x,node.z/r4,(node.y-c2*c2*c)/r4)*(wx[x]*wy[y]);
        let increment=term-correction;let next=sum+increment;
        correction=(next-sum)-increment;sum=next;
    }}
    let c=1.0-uv.x;let c2=c*c;let r2=uv.y*uv.y;let r4=r2*r2;
    if !(r4>0.0){return vec4<f32>(0.0);}
    var value=vec3<f32>(sum.x,c2*c2*c+sum.z*r4,sum.y*r4);
    // Preserve the small directly evaluated channel at either angular boundary.
    if value.x<=value.y {value.y=(1.0-value.z)-value.x;}else{value.x=(1.0-value.z)-value.y;}
    if !all(value>=vec3<f32>(0.0)) || !all(value<=vec3<f32>(1.0))
        || abs(value.x+value.y+value.z-1.0)>0.000005{return vec4<f32>(0.0);}
    return vec4<f32>(value,1.0);
}
