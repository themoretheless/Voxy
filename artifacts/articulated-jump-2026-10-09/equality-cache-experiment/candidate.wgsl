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
var<workgroup> invalid:atomic<u32>;
// Cache bounded triangular intermediates without changing arithmetic order.
// Larger operators retain storage reads and the original numerical limits.
var<workgroup> cached_z:array<vec2<f32>,256>;
var<workgroup> cached_reaction:array<vec2<f32>,256>;
fn read_z(offset:u32,index:u32,cached:bool)->vec2<f32> {
    if cached {return cached_z[index];}
    return load_pair(offset+index*2u);
}
fn read_reaction(offset:u32,index:u32,cached:bool)->vec2<f32> {
    if cached {return cached_reaction[index];}
    return load_pair(offset+index*2u);
}
fn store_pair(offset:u32,v:vec2<f32>) {data[offset]=bitcast<u32>(v.x);data[offset+1u]=bitcast<u32>(v.y);}
@compute @workgroup_size(64)
fn cs_main(@builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3<u32>) {
    if group.x!=0u {return;}
    let width=data[0];let count=data[1];let cached=count<=256u;let q=4u+count*width*2u;let r=q+count*width*2u;
    let rhs=r+count*count*2u;let z=rhs+count*2u;let reaction=z+count*2u;
    let output=reaction+count*2u;let status=output+width*2u;
    if lane==0u {
        data[status]=1u;atomicStore(&invalid,0u);
        if data[2]!=count||data[3]!=0u {atomicStore(&invalid,1u);}
        else {
            // R-transpose z = scaled original bounds.
            for(var i=0u;i<count;i++) {
                var sum=vec2<f32>(0.0);
                for(var j=0u;j<i;j++) {sum=add(sum,mul(load_pair(r+(i*count+j)*2u),read_z(z,j,cached)));}
                let value=divide(add(load_pair(rhs+i*2u),neg(sum)),load_pair(r+(i*count+i)*2u));
                if !finite_pair(value) {atomicStore(&invalid,1u);}
                store_pair(z+i*2u,value);
                if cached {cached_z[i]=value;}
            }
            // R lambda = z; R is stored by columns.
            for(var reverse=0u;reverse<count;reverse++) {
                let i=count-1u-reverse;var sum=vec2<f32>(0.0);
                for(var j=i+1u;j<count;j++) {sum=add(sum,mul(load_pair(r+(j*count+i)*2u),read_reaction(reaction,j,cached)));}
                let value=divide(add(read_z(z,i,cached),neg(sum)),load_pair(r+(i*count+i)*2u));
                if !finite_pair(value) {atomicStore(&invalid,1u);}
                store_pair(reaction+i*2u,value);
                if cached {cached_reaction[i]=value;}
            }
        }
    }
    storageBarrier();workgroupBarrier();
    for(var i=lane;i<width;i+=64u) {
        var sum=vec2<f32>(0.0);
        for(var j=0u;j<count;j++) {sum=add(sum,mul(load_pair(q+(j*width+i)*2u),read_z(z,j,cached)));}
        if !finite_pair(sum) {atomicStore(&invalid,1u);}
        store_pair(output+i*2u,sum);
    }
    storageBarrier();workgroupBarrier();
    if lane==0u && atomicLoad(&invalid)==0u {data[status]=0u;}
}
