// Packed normalized contact-column dot products; no Gram construction.
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
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    let row=id.x;let rows=data[0];let width=data[1];
    if row>=rows {return;}
    let columns=4u;let vector=columns+rows*width*2u;
    let output=vector+width*2u;let status=output+rows*2u;
    data[status+row]=1u;
    var result=vec2<f32>(0.0);
    for(var i=0u;i<width;i++) {
        let a=load_pair(columns+(row*width+i)*2u);let b=load_pair(vector+i*2u);
        if !finite_pair(a)||!finite_pair(b) {return;}
        result=add(result,mul(a,b));
    }
    if !finite_pair(result) {return;}
    data[output+row*2u]=bitcast<u32>(result.x);
    data[output+row*2u+1u]=bitcast<u32>(result.y);
    data[status+row]=0u;
}
