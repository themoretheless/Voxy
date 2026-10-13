// Experimental SPH particle density; physical owner must admit current output.
@group(0) @binding(0) var<storage, read_write> data:array<u32>;
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

fn root(v:vec2<f32>)->vec2<f32> {
    if v.x<=0.0 {return vec2<f32>(0.0);}
    var r=vec2<f32>(sqrt(v.x),0.0);
    for(var k=0u;k<2u;k++) {r=mul(add(r,divide(v,r)),vec2<f32>(0.5,0.0));}
    return r;
}
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    let i=id.x;if i>=data[0u] {return;}
    let mass=load_pair(12u+2u*i);let inverse=divide(vec2<f32>(1.0,0.0),mass);
    let pair_base=12u+2u*data[0u];let offsets=data[2u];let adjacency=data[3u];
    var acceleration:array<vec2<f32>,3>;var rate=vec2<f32>(0.0);
    for(var axis=0u;axis<3u;axis++) {acceleration[axis]=load_pair(6u+2u*axis);}
    for(var entry=data[offsets+i];entry<data[offsets+i+1u];entry++) {
        let pair=pair_base+10u*data[adjacency+entry];let a=data[pair];
        rate=add(rate,divide(load_pair(pair+8u),mass));
        for(var axis=0u;axis<3u;axis++) {
            var contribution=mul(load_pair(pair+2u+2u*axis),inverse);
            if i!=a {contribution=neg(contribution);}
            acceleration[axis]=add(acceleration[axis],contribution);
        }
    }
    let output=data[4u]+9u*i;var valid=finite_pair(rate)&&(rate.x+rate.y)>=0.0;
    for(var axis=0u;axis<3u;axis++) {
        valid=valid&&finite_pair(acceleration[axis]);
        data[output+2u*axis]=bitcast<u32>(acceleration[axis].x);data[output+2u*axis+1u]=bitcast<u32>(acceleration[axis].y);
    }
    data[output+6u]=bitcast<u32>(rate.x);data[output+7u]=bitcast<u32>(rate.y);data[output+8u]=select(1u,0u,valid);
}

// Extended ABI: original accumulation storage, contiguous force output (9 words
// per pair), followed by [pair_count, copied_force_offset].
@compute @workgroup_size(64)
fn cs_prepare_forces(@builtin(global_invocation_id) id:vec3<u32>) {
    let pair=id.x;if pair>=data[1u] {return;}
    let metadata=arrayLength(&data)-2u;
    let force_source=data[metadata+1u]+9u*pair;
    let destination=12u+2u*data[0u]+10u*pair+2u;
    for(var word=0u;word<8u;word++) {data[destination+word]=data[force_source+word];}
}
