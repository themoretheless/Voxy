// Independently recompute r=A*x-b, then smooth a signed error correction z.
// In exact arithmetic ||x-x*||inf <= ||z||inf + max_i |r-A*z|/mass_i.
// Roundoff allowances are included; this is not interval certification.
struct Parameters { lambda:vec4f, count:u32, groups:u32, absolute_error:f32, relative_squared:f32 }
struct Edge { neighbor:u32, weight:f32 }
struct Header { geometry:u32, admitted:u32, has_accepted:u32, generation:u32,
                maximum_error_estimate:f32, relative_squared:f32, residual_squared:f32, rhs_squared:f32 }
struct Work { residual:vec4f, correction0:vec4f, correction1:vec4f, stats:vec4f }
@group(0) @binding(0) var<uniform> params:Parameters;
@group(0) @binding(1) var<storage,read> mass:array<f32>;
@group(0) @binding(2) var<storage,read> rows:array<u32>;
@group(0) @binding(3) var<storage,read> edges:array<Edge>;
@group(0) @binding(4) var<storage,read> candidate:array<vec4f>;
@group(0) @binding(5) var<storage,read> incident:array<vec4f>;
@group(0) @binding(6) var<storage,read_write> work:array<Work>;
@group(0) @binding(7) var<storage,read_write> accepted:array<vec4f>;
@group(0) @binding(8) var<storage,read_write> header:Header;
var<workgroup> sums:array<vec4f,64>;
var<workgroup> correction_max:array<f32,64>;
fn finite(v:vec4f)->bool {
    return all((bitcast<vec4u>(v)&vec4u(0x7fffffffu))<vec4u(0x7f800000u));
}
fn maximum(v:vec3f)->f32 {return max(v.x,max(v.y,v.z));}
// x=max tail estimate, y=sum residual squared, z=sum rhs squared, w=invalid.
fn combine(a:vec4f,b:vec4f)->vec4f {return vec4f(max(a.x,b.x),a.y+b.y,a.z+b.z,max(a.w,b.w));}
fn reduce(value:vec4f,correction:f32,lane:u32)->vec4f {
    sums[lane]=value;correction_max[lane]=correction;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
        if lane<stride {
            sums[lane]=combine(sums[lane],sums[lane+stride]);
            correction_max[lane]=max(correction_max[lane],correction_max[lane+stride]);
        }
        workgroupBarrier();
    }
    return sums[0];
}
@compute @workgroup_size(64)
fn initialize(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;if i>=params.count {return;}
    let x=candidate[i];let s=incident[i];let m=mass[i];
    var r=vec3f(0.0);var invalid=0.0;
    if !finite(x)||!finite(s)||!finite(vec4f(m))||m<0.0
        ||any(x<vec4f(0.0))||any(s<vec4f(0.0))||(m==0.0&&rows[i]!=rows[i+1u]) {
        invalid=1.0;
    } else {
        r=m*(x.xyz-s.xyz);
        for(var j=rows[i];j<rows[i+1u];j++) {
            let e=edges[j];let neighbor=candidate[e.neighbor];
            if !finite(neighbor)||!finite(vec4f(e.weight))||e.weight<0.0 {invalid=1.0;}
            r+=params.lambda.xyz*e.weight*(x.xyz-neighbor.xyz);
        }
        if !finite(vec4f(r,0.0)) {invalid=1.0;r=vec3f(0.0);}
    }
    work[i]=Work(vec4f(r,invalid),vec4f(0.0),vec4f(0.0),vec4f(0.0));
}
fn correction(i:u32,side:u32) {
    if i>=params.count {return;}
    var sum=work[i].residual.xyz;var degree=0.0;
    for(var j=rows[i];j<rows[i+1u];j++) {
        let e=edges[j];degree+=e.weight;
        var value=work[e.neighbor].correction0.xyz;
        if side==1u {value=work[e.neighbor].correction1.xyz;}
        sum+=params.lambda.xyz*e.weight*value;
    }
    let diagonal=vec3f(mass[i])+params.lambda.xyz*degree;
    var old=work[i].correction0.xyz;
    if side==1u {old=work[i].correction1.xyz;}
    var value=old+(2.0/3.0)*(sum/max(diagonal,vec3f(1e-30))-old);
    if work[i].residual.w!=0.0||mass[i]==0.0 {value=vec3f(0.0);}
    if side==0u {work[i].correction1=vec4f(value,0.0);}
    else {work[i].correction0=vec4f(value,0.0);}
}
@compute @workgroup_size(64)
fn correct_even(@builtin(global_invocation_id) id:vec3u) {correction(id.x,0u);}
@compute @workgroup_size(64)
fn correct_odd(@builtin(global_invocation_id) id:vec3u) {correction(id.x,1u);}
@compute @workgroup_size(64)
fn audit(@builtin(global_invocation_id) id:vec3u,@builtin(local_invocation_index) lane:u32,
         @builtin(workgroup_id) group:vec3u) {
    let i=id.x;var result=vec4f(0.0);var correction_bound=0.0;
    if i<params.count {
        let x=candidate[i].xyz;let s=incident[i].xyz;let m=mass[i];
        if work[i].residual.w!=0.0 {result=vec4f(1e30,0.0,0.0,1.0);}
        else if m==0.0 {result=vec4f(maximum(abs(x-s)),0.0,0.0,0.0);}
        else {
            let z=work[i].correction0.xyz;
            let r=work[i].residual.xyz;var az=m*z;
            var absolute_terms=abs(m*(x-s))+abs(az);
            for(var j=rows[i];j<rows[i+1u];j++) {
                let e=edges[j];let factor=params.lambda.xyz*e.weight;
                let original_flux=factor*(x-candidate[e.neighbor].xyz);
                let correction_flux=factor*(z-work[e.neighbor].correction0.xyz);
                az+=correction_flux;absolute_terms+=abs(original_flux)+abs(correction_flux);
            }
            let allowance=(f32(rows[i+1u]-rows[i])+12.0)*1.1920929e-7*absolute_terms;
            let tail=(abs(r-az)+allowance)/m;let rhs=m*s;
            result=vec4f(maximum(tail),dot(r,r),dot(rhs,rhs),0.0);
            correction_bound=maximum(abs(z));
            if !finite(result)||!finite(vec4f(correction_bound)) {result=vec4f(1e30,0.0,0.0,1.0);correction_bound=0.0;}
        }
    }
    let sum=reduce(result,correction_bound,lane);
    if lane==0u {
        // Final correction is in correction0. Reuse correction1 only after all
        // smoothing dispatches, preserving neighbor reads during this audit.
        work[group.x].stats=sum;work[group.x].correction1=vec4f(correction_max[0],0.0,0.0,0.0);
    }
}
@compute @workgroup_size(64)
fn decide(@builtin(local_invocation_index) lane:u32) {
    var value=vec4f(0.0);var correction_bound=0.0;
    for(var i=lane;i<params.groups;i+=64u) {
        value=combine(value,work[i].stats);correction_bound=max(correction_bound,work[i].correction1.x);
    }
    let sum=reduce(value,correction_bound,lane);
    if lane==0u {
        let estimate=sum.x+correction_max[0];
        let ratio=select(0.0,sum.y/max(sum.z,1e-38),sum.z>0.0);
        let valid=header.geometry==0u&&sum.w==0.0&&finite(sum)&&estimate<params.absolute_error
            &&sum.y<=params.relative_squared*sum.z;
        header.admitted=select(0u,1u,valid);
        header.maximum_error_estimate=estimate;header.relative_squared=ratio;
        header.residual_squared=sum.y;header.rhs_squared=sum.z;
        if valid {header.has_accepted=1u;header.generation+=1u;}
    }
}
@compute @workgroup_size(64)
fn publish(@builtin(global_invocation_id) id:vec3u) {
    let i=id.x;if i<params.count&&header.admitted==1u {accepted[i]=candidate[i];accepted[params.count+i]=incident[i];}
}
