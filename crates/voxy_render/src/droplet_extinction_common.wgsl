@group(0) @binding(0) var<storage, read_write> words: array<u32>;
fn value(i: u32) -> f32 { return bitcast<f32>(words[i]); }
fn optical_depth(ray: u32) -> f32 {
    let shape=vec3<u32>(words[0],words[1],words[2]);
    let origin=vec3<f32>(value(6u),value(7u),value(8u));
    let spacing=vec3<f32>(value(9u),value(10u),value(11u));
    let base=12u+words[3]+8u*ray;
    let start=vec3<f32>(value(base),value(base+1u),value(base+2u));
    let end=vec3<f32>(value(base+3u),value(base+4u),value(base+5u));
    let delta=end-start;
    let distance=length(delta);
    var lo=0.0; var hi=1.0;
    let upper=origin+vec3<f32>(shape)*spacing;
    for (var axis=0u; axis<3u; axis+=1u) {
        if delta[axis]==0.0 {
            if start[axis]<origin[axis] || start[axis]>=upper[axis] { hi=lo; break; }
        } else {
            let a=(origin[axis]-start[axis])/delta[axis];
            let b=(upper[axis]-start[axis])/delta[axis];
            lo=max(lo,min(a,b)); hi=min(hi,max(a,b));
        }
    }
    var tau=0.0;
    if hi>lo && distance>0.0 {
        let entry=(start+lo*delta-origin)/spacing;
        var cell=vec3<i32>(floor(entry));
        for (var axis=0u; axis<3u; axis+=1u) {
            if delta[axis]<0.0 && entry[axis]==floor(entry[axis]) { cell[axis]-=1; }
        }
        cell=clamp(cell,vec3<i32>(0),vec3<i32>(shape)-vec3<i32>(1));
        var t=lo;
        // Each transition crosses at least one plane; finite bound includes rounding ties.
        let limit=shape.x+shape.y+shape.z+3u;
        for (var visit=0u; visit<limit; visit+=1u) {
            var crossing=vec3<f32>(hi);
            for (var axis=0u; axis<3u; axis+=1u) {
                if delta[axis]!=0.0 {
                    var boundary=f32(cell[axis]);
                    if delta[axis]>0.0 { boundary+=1.0; }
                    crossing[axis]=(origin[axis]+boundary*spacing[axis]-start[axis])/delta[axis];
                }
            }
            let next=min(hi,min(crossing.x,min(crossing.y,crossing.z)));
            let index=u32(cell.x)+shape.x*(u32(cell.y)+shape.y*u32(cell.z));
            tau+=value(12u+index)*distance*max(0.0,next-t);
            if next>=hi { break; }
            for (var axis=0u; axis<3u; axis+=1u) {
                if crossing[axis]<=next && delta[axis]!=0.0 {
                    if delta[axis]>0.0 { cell[axis]+=1; } else { cell[axis]-=1; }
                }
            }
            if any(cell<vec3<i32>(0)) || any(cell>=vec3<i32>(shape)) { break; }
            t=max(t,next);
        }
    }
    return tau;
}
