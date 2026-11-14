// One independent nine-diagonal native Cosserat linear system per invocation.
// #include "dd_math.wgsl"
@group(0) @binding(0) var<storage, read_write> data: array<u32>;
const division_refinements:u32=1u;
fn divide(a:vec2<f32>,b:vec2<f32>)->vec2<f32> {
    let next=dd_div(a,b);
    if division_refinements==1u {return next;}
    let residual=dd_sub(a,dd_mul(next,b));
    return dd_add(next,vec2<f32>((residual.x+residual.y)/b.x,0.0));
}
const root_refinements:u32=1u;
fn root(a:vec2<f32>)->vec2<f32> {
    var x=vec2<f32>(sqrt(a.x),0.0);
    for(var refinement=0u;refinement<root_refinements;refinement++) {
        x=dd_add(x,divide(dd_sub(a,dd_mul(x,x)),dd_mul(vec2<f32>(2.0,0.0),x)));
    }
    return x;
}
fn load(i:u32)->vec2<f32> { return vec2<f32>(bitcast<f32>(data[i]),bitcast<f32>(data[i+1u])); }
fn save(i:u32,v:vec2<f32>) { data[i]=bitcast<u32>(v.x); data[i+1u]=bitcast<u32>(v.y); }
@compute @workgroup_size(32)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=data[0] { return; }
    let n=data[1]; let first=data[2]; let end=data[3];
    let base=4u+id.x*(n*20u+1u); let rhs=base+n*18u; let status=rhs+n*2u;
    let reuse_factors=data[status]==2u;
    data[status]=0u;
    if !reuse_factors {
    for(var i=first;i<end;i++) {
        let start=max(first,select(0u,i-8u,i>=8u));
        for(var j=start;j<=i;j++) {
            var sum=load(base+(i*9u+i-j)*2u);
            let begin=max(start,select(0u,j-8u,j>=8u));
            for(var k=begin;k<j;k++) { sum=dd_add(sum,dd_neg(dd_mul(load(base+(i*9u+i-k)*2u),load(base+(j*9u+j-k)*2u)))); }
            if i==j {
                if !(sum.x>0.0) { data[status]=1u; return; }
                save(base+i*18u,root(sum));
            } else { save(base+(i*9u+i-j)*2u,divide(sum,load(base+j*18u))); }
        }
    }
    }
    for(var i=first;i<end;i++) {
        var value=load(rhs+i*2u);
        let begin=max(first,select(0u,i-8u,i>=8u));
        for(var j=begin;j<i;j++) { value=dd_add(value,dd_neg(dd_mul(load(base+(i*9u+i-j)*2u),load(rhs+j*2u)))); }
        save(rhs+i*2u,divide(value,load(base+i*18u)));
    }
    for(var cursor=end;cursor>first;cursor--) {
        let i=cursor-1u; var value=load(rhs+i*2u);
        for(var j=i+1u;j<min(i+9u,end);j++) { value=dd_add(value,dd_neg(dd_mul(load(base+(j*9u+j-i)*2u),load(rhs+j*2u)))); }
        save(rhs+i*2u,divide(value,load(base+i*18u)));
    }
}
