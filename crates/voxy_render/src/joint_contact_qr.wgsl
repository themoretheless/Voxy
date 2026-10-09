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
var<workgroup> scalar:vec2<f32>;
var<workgroup> current:u32;
var<workgroup> columns:u32;
var<workgroup> invalid:atomic<u32>;
fn store_pair(offset:u32,v:vec2<f32>) {data[offset]=bitcast<u32>(v.x);data[offset+1u]=bitcast<u32>(v.y);}
@compute @workgroup_size(64)
fn cs_main(@builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3<u32>) {
    if group.x!=0u {return;}
    if lane==0u {current=data[2];columns=data[1];atomicStore(&invalid,data[3]);}
    workgroupBarrier();
    let column=workgroupUniformLoad(&current);let count=workgroupUniformLoad(&columns);
    if column>=count {return;}
    let width=data[0];let input=4u;let q=input+count*width*2u;let r=q+count*width*2u;
    for(var i=lane;i<width;i+=64u) {store_pair(q+(column*width+i)*2u,load_pair(input+(column*width+i)*2u));}
    for(var j=lane;j<count;j+=64u) {store_pair(r+(column*count+j)*2u,vec2<f32>(0.0));}
    storageBarrier();workgroupBarrier();
    // Twice modified Gram-Schmidt: every projection updates the current vector
    // before the next basis product. Basis and triangular coefficients stay resident.
    for(var sweep=0u;sweep<2u;sweep++) {
        for(var j=0u;j<column;j++) {
            var sum=vec2<f32>(0.0);
            for(var i=lane;i<width;i+=64u) {sum=add(sum,mul(load_pair(q+(j*width+i)*2u),load_pair(q+(column*width+i)*2u)));}
            partial[lane]=sum;workgroupBarrier();
            for(var stride=32u;stride>0u;stride/=2u) {
                if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
                workgroupBarrier();
            }
            if lane==0u {scalar=partial[0];store_pair(r+(column*count+j)*2u,add(load_pair(r+(column*count+j)*2u),scalar));}
            workgroupBarrier();
            for(var i=lane;i<width;i+=64u) {
                let offset=q+(column*width+i)*2u;
                let value=add(load_pair(offset),neg(mul(load_pair(q+(j*width+i)*2u),scalar)));
                if !finite_pair(value) {atomicStore(&invalid,1u);}
                store_pair(offset,value);
            }
            storageBarrier();workgroupBarrier();
        }
    }
    var sum=vec2<f32>(0.0);
    for(var i=lane;i<width;i+=64u) {let v=load_pair(q+(column*width+i)*2u);sum=add(sum,mul(v,v));}
    partial[lane]=sum;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
        workgroupBarrier();
    }
    if lane==0u {
        let total=partial[0];scalar=vec2<f32>(1.0,0.0);
        if !finite_pair(total)||total.x<=1e-24 {atomicStore(&invalid,1u);}
        else {
            let first=sqrt(total.x);
            let error=add(total,neg(mul(vec2<f32>(first,0.0),vec2<f32>(first,0.0))));
            var norm=add(vec2<f32>(first,0.0),vec2<f32>((error.x+error.y)/(2.0*first),0.0));
            norm=add(norm,divide(add(total,neg(mul(norm,norm))),mul(vec2<f32>(2.0,0.0),norm)));
            if !finite_pair(norm)||norm.x<=0.0 {atomicStore(&invalid,1u);}
            else {scalar=norm;store_pair(r+(column*count+column)*2u,norm);}
        }
    }
    workgroupBarrier();
    for(var i=lane;i<width;i+=64u) {
        let offset=q+(column*width+i)*2u;let v=divide(load_pair(offset),scalar);
        if !finite_pair(v) {atomicStore(&invalid,1u);}
        store_pair(offset,v);
    }
    storageBarrier();workgroupBarrier();
    if lane==0u {data[3]=atomicLoad(&invalid);if data[3]==0u {data[2]=column+1u;}}
}
