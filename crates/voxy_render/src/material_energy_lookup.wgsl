// Nonfiltering lookup for RGBA32F moments: N.V=u^2, roughness=v.
// RGB = A, B, directly integrated loss; Alpha=1 is the producer validity marker.
// Incomplete/invalid texels and invalid physical coordinates fail closed.
fn lookup_ggx_energy(image: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    if !all(uv>=vec2<f32>(0.0)) || !all(uv<=vec2<f32>(1.0)) {
        return vec4<f32>(0.0);
    }
    let dimensions=textureDimensions(image,0);
    let maximum=vec2<i32>(dimensions)-vec2<i32>(1);
    let position=clamp(vec2<f32>(sqrt(uv.x),uv.y)*vec2<f32>(maximum),vec2<f32>(0.0),vec2<f32>(maximum));
    let base=vec2<i32>(floor(position));
    let next=min(base+vec2<i32>(1),maximum);
    let weight=position-vec2<f32>(base);
    let aa=textureLoad(image,base,0);
    let ba=textureLoad(image,vec2<i32>(next.x,base.y),0);
    let ab=textureLoad(image,vec2<i32>(base.x,next.y),0);
    let bb=textureLoad(image,next,0);
    let texels=array<vec4<f32>,4>(aa,ba,ab,bb);
    for(var i=0u;i<4u;i++) {
        if texels[i].w!=1.0 || !all(texels[i].xyz>=vec3<f32>(0.0)) || !all(texels[i].xyz<=vec3<f32>(1.0)) {
            return vec4<f32>(0.0);
        }
    }
    let first=aa.xyz*(1.0-weight.x)+ba.xyz*weight.x;
    let second=ab.xyz*(1.0-weight.x)+bb.xyz*weight.x;
    return vec4<f32>(first*(1.0-weight.y)+second*weight.y,1.0);
}

// Candidate four-point Lagrange interpolation per axis. Physical range and
// producer validity are checked; overshoots fail rather than being clamped.
fn ggx_cubic_weights(t:f32) -> vec4<f32> {
    return vec4<f32>(-t*(t-1.0)*(t-2.0)/6.0,
        (t+1.0)*(t-1.0)*(t-2.0)*0.5,
        -(t+1.0)*t*(t-2.0)*0.5,
        (t+1.0)*t*(t-1.0)/6.0);
}
fn lookup_ggx_energy_cubic(image:texture_2d<f32>,uv:vec2<f32>) -> vec4<f32> {
    if !all(uv>=vec2<f32>(0.0)) || !all(uv<=vec2<f32>(1.0)) {return vec4<f32>(0.0);}
    let dimensions=textureDimensions(image,0);
    let maximum=vec2<i32>(dimensions)-vec2<i32>(1);
    let position=clamp(vec2<f32>(sqrt(uv.x),uv.y)*vec2<f32>(maximum),vec2<f32>(0.0),vec2<f32>(maximum));
    // Four distinct nodes, including one-sided stencils at either endpoint.
    // The weight polynomial is expressed relative to the second node.
    if any(dimensions<vec2<u32>(4u)) {return vec4<f32>(0.0);}
    let origin=clamp(vec2<i32>(floor(position))-vec2<i32>(1),
        vec2<i32>(0),maximum-vec2<i32>(3));
    let t=position-vec2<f32>(origin)-vec2<f32>(1.0);
    let wx=ggx_cubic_weights(t.x);let wy=ggx_cubic_weights(t.y);
    var total=vec3<f32>(0.0);var correction=vec3<f32>(0.0);
    for(var y=0u;y<4u;y++){for(var x=0u;x<4u;x++){
        let coordinate=origin+vec2<i32>(i32(x),i32(y));
        let texel=textureLoad(image,coordinate,0);
        if texel.w!=1.0 || !all(texel.xyz>=vec3<f32>(0.0)) || !all(texel.xyz<=vec3<f32>(1.0)){return vec4<f32>(0.0);}
        let increment=texel.xyz*(wx[x]*wy[y])-correction;
        let next=total+increment;correction=(next-total)-increment;total=next;
    }}
    if !all(total>=vec3<f32>(0.0)) || !all(total<=vec3<f32>(1.0)){return vec4<f32>(0.0);}
    return vec4<f32>(total,1.0);
}

// Experimental loss representation. Extrapolates loss/roughness^4 from
// positive roughness nodes; its zero-limit and grazing corner are not admitted.
fn ggx_lookup_scaled_loss(image:texture_2d<f32>,uv:vec2<f32>) -> vec2<f32> {
    if !all(uv>=vec2<f32>(0.0)) || !all(uv<=vec2<f32>(1.0)){return vec2<f32>(0.0);}
    if uv.y==0.0 {return vec2<f32>(0.0,1.0);}
    let dimensions=textureDimensions(image,0);
    if any(dimensions<vec2<u32>(5u)) {return vec2<f32>(0.0);}
    let maximum=vec2<i32>(dimensions)-vec2<i32>(1);
    let position=vec2<f32>(sqrt(uv.x),uv.y)*vec2<f32>(maximum);
    let origin=clamp(vec2<i32>(floor(position))-vec2<i32>(1),
        vec2<i32>(0,1),maximum-vec2<i32>(3));
    let t=position-vec2<f32>(origin)-vec2<f32>(1.0);
    let wx=ggx_cubic_weights(t.x);let wy=ggx_cubic_weights(t.y);
    var sum=0.0;var correction=0.0;
    for(var y=0u;y<4u;y++){for(var x=0u;x<4u;x++){
        let coordinate=origin+vec2<i32>(i32(x),i32(y));
        let node=textureLoad(image,coordinate,0);
        if node.w!=1.0 || !all(node.xyz>=vec3<f32>(0.0)) || !all(node.xyz<=vec3<f32>(1.0)){return vec2<f32>(0.0);}
        let r=f32(coordinate.y)/f32(maximum.y);let r2=r*r;
        let increment=(node.z/(r2*r2))*(wx[x]*wy[y])-correction;
        let next=sum+increment;correction=(next-sum)-increment;sum=next;
    }}
    let r2=uv.y*uv.y;let loss=sum*(r2*r2);
    if !(loss>=0.0 && loss<=1.0){return vec2<f32>(0.0);}
    return vec2<f32>(loss,1.0);
}

fn lookup_ggx_energy_loss_scaled(image:texture_2d<f32>,uv:vec2<f32>) -> vec4<f32> {
    let raw=lookup_ggx_energy_cubic(image,uv);
    let energy=ggx_lookup_scaled_loss(image,uv);
    if raw.w!=1.0 || energy.y!=1.0 {return vec4<f32>(0.0);}
    let loss=energy.x;
    var value=vec3<f32>(raw.xy,loss);
    // Preserve the smaller Fresnel channel, reconstruct its complement using
    // the independently interpolated loss. Never clamp overshoot into range.
    if value.x<=value.y {value.y=(1.0-loss)-value.x;}else{value.x=(1.0-loss)-value.y;}
    if !all(value>=vec3<f32>(0.0)) || !all(value<=vec3<f32>(1.0)){return vec4<f32>(0.0);}
    return vec4<f32>(value,1.0);
}

// Experimental six-node interpolation of B in physical N.V and roughness.
// The near-smooth/grazing corner is not qualified; reject overshoot explicitly.
fn ggx_fresnel_six_mode(image:texture_2d<f32>,uv:vec2<f32>,residual:bool) -> vec4<f32> {
    let energy=ggx_lookup_scaled_loss(image,uv);
    if energy.y!=1.0 {return vec4<f32>(0.0);}
    if residual && uv.y==0.0 {
        let c=1.0-uv.x;let c2=c*c;let b=c2*c2*c;
        let a=uv.x*(1.0+c+c2+c2*c+c2*c2);
        if a<=b {return vec4<f32>(a,1.0-a,0.0,1.0);}
        return vec4<f32>(1.0-b,b,0.0,1.0);
    }
    let dimensions=textureDimensions(image,0);
    if any(dimensions<vec2<u32>(6u)) {return vec4<f32>(0.0);}
    let maximum=vec2<i32>(dimensions)-vec2<i32>(1);
    let position=vec2<f32>(sqrt(uv.x),uv.y)*vec2<f32>(maximum);
    var origin=clamp(vec2<i32>(floor(position))-vec2<i32>(2),
        vec2<i32>(0),maximum-vec2<i32>(5));
    if residual {
        if dimensions.y<7u {return vec4<f32>(0.0);}
        origin.y=max(origin.y,1);
    }
    var view_nodes:array<f32,6>;var rough_nodes:array<f32,6>;
    for(var i=0u;i<6u;i++) {
        let u=f32(origin.x+i32(i))/f32(maximum.x);
        view_nodes[i]=u*u;
        rough_nodes[i]=f32(origin.y+i32(i))/f32(maximum.y);
    }
    var wx:array<f32,6>;var wy:array<f32,6>;
    for(var i=0u;i<6u;i++) {
        wx[i]=1.0;wy[i]=1.0;
        for(var j=0u;j<6u;j++) {if j!=i {
            wx[i]*=(uv.x-view_nodes[j])/(view_nodes[i]-view_nodes[j]);
            wy[i]*=(uv.y-rough_nodes[j])/(rough_nodes[i]-rough_nodes[j]);
        }}
    }
    var bias=0.0;var correction=0.0;
    for(var y=0u;y<6u;y++){for(var x=0u;x<6u;x++) {
        let node=textureLoad(image,origin+vec2<i32>(i32(x),i32(y)),0);
        if node.w!=1.0 || !all(node.xyz>=vec3<f32>(0.0)) || !all(node.xyz<=vec3<f32>(1.0)){return vec4<f32>(0.0);}
        var sample=node.y;
        if residual {
            let c=1.0-view_nodes[x];let c2=c*c;
            let r2=rough_nodes[y]*rough_nodes[y];
            sample=(sample-c2*c2*c)/(r2*r2);
        }
        let increment=sample*(wx[x]*wy[y])-correction;
        let next=bias+increment;correction=(next-bias)-increment;bias=next;
    }}
    if residual {
        let c=1.0-uv.x;let c2=c*c;let r2=uv.y*uv.y;
        bias=c2*c2*c+bias*(r2*r2);
    }
    let value=vec3<f32>((1.0-energy.x)-bias,bias,energy.x);
    if !all(value>=vec3<f32>(0.0)) || !all(value<=vec3<f32>(1.0)){return vec4<f32>(0.0);}
    return vec4<f32>(value,1.0);
}

fn lookup_ggx_energy_fresnel_six(image:texture_2d<f32>,uv:vec2<f32>) -> vec4<f32> {
    return ggx_fresnel_six_mode(image,uv,false);
}
fn lookup_ggx_energy_fresnel_residual(image:texture_2d<f32>,uv:vec2<f32>) -> vec4<f32> {
    return ggx_fresnel_six_mode(image,uv,true);
}
