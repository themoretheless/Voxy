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
fn value(base:u32,index:u32)->vec2<f32> {return load_pair(base+2u*index);}
fn square(v:vec2<f32>)->vec2<f32> {return mul(v,v);}
fn cube(v:vec2<f32>)->vec2<f32> {return mul(square(v),v);}
fn clamp_positive(v:vec2<f32>)->vec2<f32> {
    if v.x+v.y<=0.0 {return vec2<f32>(0.0);}return v;
}
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    let i=id.x;if i>=data[0u] {return;}
    let base=2u+51u*i;let mode=data[base+50u];let formulation=mode&3u;
    let zero=vec2<f32>(0.0);let one=vec2<f32>(1.0,0.0);
    var squared=zero;
    for(var axis=0u;axis<3u;axis++) {squared=add(squared,square(value(base,axis)));}
    let r=root(squared);let h=value(base,17u);
    var direction:array<vec2<f32>,3>;
    direction[0]=one;direction[1]=zero;direction[2]=zero;
    if r.x+r.y>1e-12 {
        for(var axis=0u;axis<3u;axis++) {direction[axis]=divide(value(base,axis),r);}
    }
    let m0=value(base,9u);let m1=value(base,10u);
    let rho0=value(base,11u);let rho1=value(base,12u);
    let p0=value(base,13u);let p1=value(base,14u);
    let ref0=value(base,15u);let ref1=value(base,16u);
    let remaining=clamp_positive(add(h,neg(r)));let q=divide(r,h);
    var ratio=one;var gradient=mul(value(base,21u),square(remaining));
    if formulation!=0u || (mode&4u)!=0u {
        if formulation==2u {
            gradient=mul(mul(value(base,23u),q),cube(clamp_positive(add(one,neg(q)))));
        } else {
            gradient=mul(mul(value(base,22u),r),square(clamp_positive(add(square(h),neg(square(r))))));
        }
    }
    if formulation!=0u {ratio=divide(ref0,ref1);}
    let pressure=mul(mul(mul(m0,m1),add(mul(divide(p0,square(rho0)),ratio),divide(divide(p1,square(rho1)),ratio))),gradient);
    var viscosity=zero;
    if r.x+r.y>1e-12 {
        viscosity=divide(mul(mul(mul(mul(mul(vec2<f32>(5.0,0.0),m0),m1),value(base,18u)),value(base,21u)),remaining),mul(rho0,rho1));
    }
    let correction=divide(add(ref0,ref1),add(rho0,rho1));let surface=value(base,19u);
    var cohesion=zero;
    if r.x+r.y>1e-12 && q.x+q.y<1.0 {
        let shape=mul(cube(add(one,neg(q))),cube(q));var adjusted=shape;
        if q.x+q.y<=0.5 {adjusted=add(mul(vec2<f32>(2.0,0.0),shape),vec2<f32>(-0.015625,0.0));}
        cohesion=mul(mul(mul(mul(surface,m0),m1),correction),mul(value(base,24u),adjusted));
    }
    let curvature=mul(mul(surface,correction),divide(mul(mul(vec2<f32>(2.0,0.0),m0),m1),add(m0,m1)));
    var curvature_normal=zero;var relative_normal=zero;
    for(var axis=0u;axis<3u;axis++) {
        curvature_normal=add(curvature_normal,mul(value(base,6u+axis),direction[axis]));
        relative_normal=add(relative_normal,mul(value(base,3u+axis),direction[axis]));
    }
    let interface_force=mul(mul(value(base,20u),r),square(remaining));
    let radial=add(add(add(pressure,neg(cohesion)),interface_force),neg(mul(curvature,curvature_normal)));
    var visc=zero;if (mode&8u)==0u {visc=mul(viscosity,relative_normal);}
    let total=add(radial,visc);let output=data[1u]+9u*i;var valid=finite_pair(viscosity)&&(viscosity.x+viscosity.y)>=0.0;
    for(var axis=0u;axis<3u;axis++) {
        let force=mul(total,direction[axis]);valid=valid&&finite_pair(force);
        data[output+2u*axis]=bitcast<u32>(force.x);data[output+2u*axis+1u]=bitcast<u32>(force.y);
    }
    data[output+6u]=bitcast<u32>(viscosity.x);data[output+7u]=bitcast<u32>(viscosity.y);
    data[output+8u]=select(1u,0u,valid);
}

// Extended storage: original force ABI, copied particle outputs (11 words per
// particle), ordered pair topology, and [particle_count, outputs, topology, end].
// Both kernels execute on the same allocation; only cs_main writes force output.
@compute @workgroup_size(64)
fn cs_prepare(@builtin(global_invocation_id) id:vec3<u32>) {
    let pair=id.x;if pair>=data[0u] {return;}
    let metadata=arrayLength(&data)-4u;
    let n=data[metadata];let fields=data[metadata+1u];let topology=data[metadata+2u];
    let i=data[topology+2u*pair];let j=data[topology+2u*pair+1u];
    let base=2u+51u*pair;
    for(var axis=0u;axis<3u;axis++) {
        let difference=add(load_pair(fields+7u*i+2u*axis),neg(load_pair(fields+7u*j+2u*axis)));
        data[base+12u+2u*axis]=bitcast<u32>(difference.x);
        data[base+13u+2u*axis]=bitcast<u32>(difference.y);
    }
    for(var endpoint=0u;endpoint<2u;endpoint++) {
        let particle=data[topology+2u*pair+endpoint];
        let density=fields+7u*n+2u*particle;
        let pressure=fields+9u*n+2u*particle;
        data[base+22u+2u*endpoint]=data[density];
        data[base+23u+2u*endpoint]=data[density+1u];
        data[base+26u+2u*endpoint]=data[pressure];
        data[base+27u+2u*endpoint]=data[pressure+1u];
    }
}
