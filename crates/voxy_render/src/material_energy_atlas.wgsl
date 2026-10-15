// Version-two immutable atlas with a balanced, threaded spatial index.
// Returns region id, visited-node count, and validated-header marker.
fn find_ggx_energy_region(image:texture_2d<f32>,uv:vec2<f32>) -> vec3<u32> {
    let missing=0xffffffffu;
    if !all(uv>=vec2<f32>(0.0)) || !all(uv<=vec2<f32>(1.0)){return vec3<u32>(missing,0u,0u);}
    let dimensions=textureDimensions(image,0);
    if dimensions.x!=6u || dimensions.y<10u{return vec3<u32>(missing,0u,0u);}
    let header=textureLoad(image,vec2<i32>(0),0);
    let tree=textureLoad(image,vec2<i32>(1,0),0);
    if header.x!=4137.0 || header.y!=2.0 || header.w!=8.0
        || !(header.z>=1.0 && header.z<=f32((dimensions.y-1u)/8u))
        || floor(header.z)!=header.z{return vec3<u32>(missing,0u,0u);}
    let count=u32(header.z);let start=1u+8u*count;let nodes=2u*count-1u;
    if tree.x!=f32(start) || tree.y!=f32(nodes) || tree.z!=1.0
        || dimensions.y!=start+(nodes+2u)/3u{return vec3<u32>(missing,0u,0u);}
    // Exact delta boundary requires no sampled patch coefficients.
    if uv.y==0.0{return vec3<u32>(missing,0u,1u);}
    // A small canonical scan avoids tree-fetch overhead for sparse atlases.
    // This changes only region selection; the interpolation is shared.
    if count<=64u {
        for(var i=0u;i<count;i++) {
            let bounds=textureLoad(image,vec2<i32>(0,i32(1u+8u*i)),0);
            if all(uv>=bounds.xy) && all(uv<=bounds.zw){return vec3<u32>(i,i+1u,1u);}
        }
        return vec3<u32>(missing,count,1u);
    }
    var index=0u;var visited=0u;var best=missing;
    while index<nodes {
        visited++;
        let row=i32(start+index/3u);let x=i32(2u*(index%3u));
        let bounds=textureLoad(image,vec2<i32>(x,row),0);
        let links=textureLoad(image,vec2<i32>(x+1,row),0);
        if !(links.x>f32(index) && links.x<=f32(nodes)) || floor(links.x)!=links.x
            || !(links.z==0.0 || links.z==1.0){return vec3<u32>(missing,visited,0u);}
        let hit=all(uv>=bounds.xy) && all(uv<=bounds.zw);
        if !hit {index=u32(links.x);}
        else if links.z==1.0 {
            if !(links.y>=0.0 && links.y<f32(count)) || floor(links.y)!=links.y{return vec3<u32>(missing,visited,0u);}
            // Shared closed boundaries retain the previous canonical tie rule.
            best=min(best,u32(links.y));index=u32(links.x);
        }else{index++;}
    }
    return vec3<u32>(best,visited,1u);
}
fn lookup_ggx_energy_atlas(image:texture_2d<f32>,uv:vec2<f32>) -> vec4<f32> {
    let found=find_ggx_energy_region(image,uv);
    if found.z!=1u{return vec4<f32>(0.0);}
    if uv.y==0.0 {
        let c=1.0-uv.x;let c2=c*c;let b=c2*c2*c;
        let a=uv.x*(1.0+c+c2+c2*c+c2*c2);
        if a<=b{return vec4<f32>(a,1.0-a,0.0,1.0);}
        return vec4<f32>(1.0-b,b,0.0,1.0);
    }
    if found.x==0xffffffffu{return vec4<f32>(0.0);}
    let row=i32(1u+8u*found.x);
    var region:GgxEnergyPatch;
    for(var x=0u;x<6u;x++) {
        let coordinate=textureLoad(image,vec2<i32>(i32(x),row+1),0);
        if coordinate.w!=1.0{return vec4<f32>(0.0);}
        region.view_nodes[x]=coordinate.x;region.roughness_nodes[x]=coordinate.y;
        for(var y=0u;y<6u;y++){region.moments[y*6u+x]=textureLoad(image,vec2<i32>(i32(x),row+2+i32(y)),0);}
    }
    return interpolate_ggx_energy_patch(region,uv);
}
