@group(0) @binding(0) var<storage,read_write> words:array<u32>;
fn scalar(i:u32)->f32 {return bitcast<f32>(words[i]);}
fn vector(i:u32)->vec3f {return vec3f(scalar(i),scalar(i+1u),scalar(i+2u));}
fn finite(x:f32)->bool {return abs(x)<=3.402823e38;}
fn finite3(x:vec3f)->bool {return all(abs(x)<=vec3f(3.402823e38));}
struct OpticalGeometryHit {
    distance:f32,position:vec3f,normal:vec3f,index:u32,found:bool,entering:bool,error:u32,
}
fn optical_geometry_hit(origin:vec3f,direction:vec3f,minimum:f32,maximum:f32)->OpticalGeometryHit {
    var best=maximum;var found=false;var index=0u;
    var point=vec3f(0.0);var normal=vec3f(0.0);var entering=false;var error=0u;
    for(var i=0u;i<words[0];i+=1u) {
        let base=4u+24u*i;
        let a=vector(base);let b=vector(base+4u);let c=vector(base+8u);let outward=vector(base+12u);
        let denominator=dot(direction,outward);
        if denominator==0.0 {continue;}
        let distance=dot(a-origin,outward)/denominator;
        if !finite(distance) {error=2u;break;}
        if distance<=minimum || distance>best {continue;}
        var p=origin+direction*distance;
        var axis=0u;if abs(outward.y)>abs(outward.x) {axis=1u;}
        if abs(outward.z)>abs(outward[axis]) {axis=2u;}
        var residual=0.0;
        for(var k=0u;k<3u;k+=1u) {if k!=axis {residual+=outward[k]*(p[k]-a[k]);}}
        p[axis]=a[axis]-residual/outward[axis];
        if !finite3(p) {error=2u;break;}
        let edges=vec3f(dot(cross(b-a,p-a),outward),dot(cross(c-b,p-b),outward),dot(cross(a-c,p-c),outward));
        if !finite3(edges) {error=2u;break;}
        if any(edges<vec3f(0.0)) {continue;}
        let is_entering=denominator<0.0;let n=select(-outward,outward,is_entering);
        if found && distance==best {
            let old=4u+24u*index;
            if words[old+20u]!=words[base+20u] || words[old+23u]!=words[base+23u]
                || (words[base+20u]==0u && dot(normal,n)<0.999999) {error=1u;}
            continue;
        }
        found=true;best=distance;index=i;point=p;normal=n;entering=is_entering;error=0u;
    }
    return OpticalGeometryHit(best,point,normal,index,found,entering,error);
}
