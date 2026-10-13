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
fn kernel(radius:vec2<f32>)->vec2<f32> {
    let coefficient=load_pair(6u);
    if data[1u]==2u {
        let t=add(vec2<f32>(1.0,0.0),neg(radius));
        if t.x<=0.0 {return vec2<f32>(0.0);}
        let t2=mul(t,t);
        return mul(coefficient,mul(mul(t2,t2),add(vec2<f32>(1.0,0.0),mul(vec2<f32>(4.0,0.0),radius))));
    }
    let t=add(vec2<f32>(1.0,0.0),neg(mul(radius,radius)));
    if t.x<=0.0 {return vec2<f32>(0.0);}
    return mul(coefficient,mul(mul(t,t),t));
}
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    let i=id.x;if i>=data[0u] {return;}
    let p=8u+11u*i;var rho=mul(load_pair(p+6u),kernel(vec2<f32>(0.0)));
    let pair_base=8u+11u*data[0u];let offsets=data[2u];let adjacency=data[3u];
    for(var entry=data[offsets+i];entry<data[offsets+i+1u];entry++) {
        let pair=pair_base+2u*data[adjacency+entry];let a=data[pair];let b=data[pair+1u];
        let pa=8u+11u*a;let pb=8u+11u*b;var square=vec2<f32>(0.0);
        for(var axis=0u;axis<3u;axis++) {
            let delta=add(load_pair(pa+axis*2u),neg(load_pair(pb+axis*2u)));
            square=add(square,mul(delta,delta));
        }
        let w=kernel(root(square));var ratio=vec2<f32>(1.0,0.0);
        if data[1u]!=0u {ratio=divide(load_pair(pa+8u),load_pair(pb+8u));}
        if i==a {rho=add(rho,mul(mul(load_pair(pb+6u),w),ratio));}
        else {rho=add(rho,divide(mul(load_pair(pa+6u),w),ratio));}
    }
    if data[p+10u]!=0u {rho=load_pair(p+8u);}
    let output=data[4u]+3u*i;data[output]=bitcast<u32>(rho.x);data[output+1u]=bitcast<u32>(rho.y);
    data[output+2u]=select(1u,0u,finite_pair(rho)&&(rho.x+rho.y)>0.0);
}
