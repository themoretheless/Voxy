// Qualification kernel: paired-f32 norm and vector normalization.
// Fixed 64-lane reduction tree; persistent storage is separate work.
@group(0) @binding(0) var<storage, read_write> data: array<u32>;
fn rounded_add(a:f32,b:f32)->f32 { return fma(a,1.0,b); }
fn rounded_mul(a:f32,b:f32)->f32 { return fma(a,b,0.0); }
fn add(a:vec2<f32>,b:vec2<f32>)->vec2<f32> {
    let s=rounded_add(a.x,b.x); let v=rounded_add(s,-a.x);
    let left=rounded_add(a.x,-rounded_add(s,-v));
    let right=rounded_add(b.x,-v);
    let e=rounded_add(rounded_add(rounded_add(left,right),a.y),b.y);
    let hi=rounded_add(s,e);return vec2<f32>(hi,rounded_add(e,-rounded_add(hi,-s)));
}
fn neg(a:vec2<f32>)->vec2<f32> { return -a; }
fn mul(a:vec2<f32>,b:vec2<f32>)->vec2<f32> {
    let p=rounded_mul(a.x,b.x);
    let error=fma(a.x,b.x,-p);
    let cross=rounded_add(rounded_add(error,rounded_mul(a.x,b.y)),rounded_mul(a.y,b.x));
    let e=rounded_add(cross,rounded_mul(a.y,b.y));
    let hi=rounded_add(p,e);return vec2<f32>(hi,rounded_add(e,-rounded_add(hi,-p)));
}

fn load_pair(i:u32)->vec2<f32> {return vec2<f32>(bitcast<f32>(data[i]),bitcast<f32>(data[i+1u]));}
fn finite_pair(v:vec2<f32>)->bool {return all(abs(v)<=vec2<f32>(3.402823466e38));}
fn divide(a:vec2<f32>,b:vec2<f32>)->vec2<f32> {
    let q=a.x/b.x;
    let remainder=add(a,neg(mul(b,vec2<f32>(q,0.0))));
    return add(vec2<f32>(q,0.0),vec2<f32>((remainder.x+remainder.y)/b.x,0.0));
}
var<workgroup> partial:array<vec2<f32>,64>;
var<workgroup> shared_norm:vec2<f32>;
var<workgroup> invalid:atomic<u32>;
@compute @workgroup_size(64)
fn cs_main(@builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3<u32>) {
    if group.x!=0u {return;}
    let rows=data[0];let width=data[1];
    let output=4u+width*2u;let status=output+rows*2u;
    if lane==0u {atomicStore(&invalid,0u);shared_norm=vec2<f32>(1.0,0.0);}
    for(var i=lane;i<rows;i+=64u) {data[status+i]=1u;}
    workgroupBarrier();
    var squared=vec2<f32>(0.0);
    for(var i=lane;i<width;i+=64u) {
        let v=load_pair(4u+i*2u);
        if !finite_pair(v) {atomicStore(&invalid,1u);}
        squared=add(squared,mul(v,v));
    }
    if !finite_pair(squared) {atomicStore(&invalid,1u);}
    partial[lane]=squared;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
        workgroupBarrier();
    }
    if lane==0u {
        let total=partial[0];
        if !finite_pair(total)||total.x<=0.0 {atomicStore(&invalid,1u);}
        else {
            let first=sqrt(total.x);
            let error=add(total,neg(mul(vec2<f32>(first,0.0),vec2<f32>(first,0.0))));
            var norm=add(vec2<f32>(first,0.0),vec2<f32>((error.x+error.y)/(2.0*first),0.0));
            let remaining=add(total,neg(mul(norm,norm)));
            norm=add(norm,divide(remaining,mul(vec2<f32>(2.0,0.0),norm)));
            if !finite_pair(norm)||norm.x<=0.0 {atomicStore(&invalid,1u);}
            else {shared_norm=norm;data[output]=bitcast<u32>(norm.x);data[output+1u]=bitcast<u32>(norm.y);}
        }
    }
    workgroupBarrier();
    for(var i=lane;i<width;i+=64u) {
        let v=divide(load_pair(4u+i*2u),shared_norm);
        if !finite_pair(v) {atomicStore(&invalid,1u);}
        data[output+(i+1u)*2u]=bitcast<u32>(v.x);
        data[output+(i+1u)*2u+1u]=bitcast<u32>(v.y);
    }
    workgroupBarrier();storageBarrier();
    if atomicLoad(&invalid)==0u {
        for(var i=lane;i<rows;i+=64u) {data[status+i]=0u;}
    }
}
