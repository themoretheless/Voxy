// Appended to the canonical compensated arithmetic prefix.
var<workgroup> values:array<vec2<f32>,1260>;
var<workgroup> header:vec4<u32>;
var<workgroup> failure:u32;
fn load(i:u32)->vec2<f32> { return values[i/2u]; }
fn save(i:u32,v:vec2<f32>) { values[i/2u]=v; }
fn solve_vectors(n:u32,first:u32,end:u32)->u32 {
    let base=0u; let rhs=n*18u;
    for(var i=first;i<end;i++) {
        var value=load(rhs+i*2u);
        let begin=max(first,select(0u,i-8u,i>=8u));
        for(var j=begin;j<i;j++) { value=add(value,neg(mul(load(base+(i*9u+i-j)*2u),load(rhs+j*2u)))); }
        save(rhs+i*2u,divide(value,load(base+i*18u)));
    }
    for(var cursor=end;cursor>first;cursor--) {
        let i=cursor-1u; var value=load(rhs+i*2u);
        for(var j=i+1u;j<min(i+9u,end);j++) { value=add(value,neg(mul(load(base+(j*9u+j-i)*2u),load(rhs+j*2u)))); }
        save(rhs+i*2u,divide(value,load(base+i*18u)));
    }
    return 0u;
}
fn factor_shared(first:u32,end:u32,lane:u32) {
    if lane==0u {failure=0u;}
    workgroupBarrier();
    for(var j=first;j<end;j++) {
        if lane==0u {
            var sum=load(j*18u);
            let start=max(first,select(0u,j-8u,j>=8u));
            for(var k=start;k<j;k++) {let value=load((j*9u+j-k)*2u);sum=add(sum,neg(mul(value,value)));}
            if !(sum.x>0.0) {failure=1u;save(j*18u,vec2<f32>(1.0,0.0));}
            else {save(j*18u,root(sum));}
        }
        workgroupBarrier();
        let i=j+1u+lane;
        if lane<8u && i<end {
            var sum=load((i*9u+i-j)*2u);
            let start=max(first,select(0u,i-8u,i>=8u));
            for(var k=start;k<j;k++) {sum=add(sum,neg(mul(load((i*9u+i-k)*2u),load((j*9u+j-k)*2u))));}
            save((i*9u+i-j)*2u,divide(sum,load(j*18u)));
        }
        workgroupBarrier();
    }
}
@compute @workgroup_size(32)
fn cs_main(@builtin(workgroup_id) group:vec3<u32>,@builtin(local_invocation_index) lane:u32) {
    if lane==0u {header=vec4<u32>(data[0],data[1],data[2],data[3]);}
    workgroupBarrier();
    let parameters=workgroupUniformLoad(&header);
    if group.x>=parameters.x {return;}
    let n=parameters.y; let base=4u+group.x*(n*20u+1u);
    if n>126u || parameters.z>=parameters.w || parameters.w>n {
        if lane==0u {data[base+n*20u]=2u;}
        return;
    }
    for(var k=lane;k<n*10u;k+=32u) {
        values[k]=vec2<f32>(bitcast<f32>(data[base+2u*k]),bitcast<f32>(data[base+2u*k+1u]));
    }
    workgroupBarrier();
    factor_shared(parameters.z,parameters.w,lane);
    if lane==0u && failure==0u {failure=solve_vectors(n,parameters.z,parameters.w);}
    workgroupBarrier();
    for(var k=lane;k<n*10u;k+=32u) {
        data[base+2u*k]=bitcast<u32>(values[k].x);
        data[base+2u*k+1u]=bitcast<u32>(values[k].y);
    }
    if lane==0u {data[base+n*20u]=failure;}
}
