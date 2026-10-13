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
fn effective_density(i:u32)->vec2<f32> {
    var rho=load_pair(8u+11u*i+8u);let support=data[1u];
    if support!=0u {
        let values=support+data[0u]+1u;
        for(var j=data[support+i];j<data[support+i+1u];j++) {rho=add(rho,load_pair(values+2u*j));}
    }
    return rho;
}
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    let i=id.x;if i>=data[0u] {return;}
    let p=8u+11u*i;let pair_base=8u+11u*data[0u];
    let offsets=data[2u];let adjacency=data[3u];var normal:array<vec2<f32>,3>;
    for(var axis=0u;axis<3u;axis++) {normal[axis]=vec2<f32>(0.0);}
    for(var entry=data[offsets+i];entry<data[offsets+i+1u];entry++) {
        let pair=pair_base+2u*data[adjacency+entry];let a=data[pair];let b=data[pair+1u];
        let pa=8u+11u*a;let pb=8u+11u*b;
        if data[pa+10u]!=data[pb+10u] {continue;}
        var delta:array<vec2<f32>,3>;var square=vec2<f32>(0.0);
        for(var axis=0u;axis<3u;axis++) {
            delta[axis]=add(load_pair(pa+2u*axis),neg(load_pair(pb+2u*axis)));
            square=add(square,mul(delta[axis],delta[axis]));
        }
        var t=add(vec2<f32>(1.0,0.0),neg(square));
        if t.x+t.y<0.0 {t=vec2<f32>(0.0);}
        let gradient=mul(load_pair(6u),mul(t,t));
        var weight=divide(load_pair(pb+6u),effective_density(b));
        if i==b {weight=neg(divide(load_pair(pa+6u),effective_density(a)));}
        for(var axis=0u;axis<3u;axis++) {
            normal[axis]=add(normal[axis],mul(mul(weight,gradient),delta[axis]));
        }
    }
    let output=data[4u]+7u*i;var valid=true;
    for(var axis=0u;axis<3u;axis++) {
        valid=valid&&finite_pair(normal[axis]);
        data[output+2u*axis]=bitcast<u32>(normal[axis].x);data[output+2u*axis+1u]=bitcast<u32>(normal[axis].y);
    }
    if data[5u]!=0u {
        let rho=effective_density(i);valid=valid&&finite_pair(rho)&&(rho.x+rho.y)>0.0;
        data[data[5u]+2u*i]=bitcast<u32>(rho.x);data[data[5u]+2u*i+1u]=bitcast<u32>(rho.y);
        let support=data[1u];let laws=support+data[0u]+1u+2u*data[support+data[0u]];
        if support!=0u && data[4u]==laws+5u*data[0u] {
            let law=laws+5u*i;var argument=rho;
            if data[law+4u]==0u {
                argument=add(rho,neg(load_pair(law)));
                if argument.x+argument.y<0.0 {argument=vec2<f32>(0.0);}
            }
            let pressure=mul(load_pair(law+2u),argument);
            valid=valid&&finite_pair(pressure)&&(pressure.x+pressure.y)>=0.0;
            let pressure_output=data[5u]+2u*data[0u]+2u*i;
            data[pressure_output]=bitcast<u32>(pressure.x);data[pressure_output+1u]=bitcast<u32>(pressure.y);
        }

    }
    data[output+6u]=select(1u,0u,valid);
}
