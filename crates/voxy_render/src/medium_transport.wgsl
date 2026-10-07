// Pure transfer operations: no simulation ownership, textures or storage bindings.
// Coefficients/length/source are nonnegative finite values admitted by the caller.
struct MediumTransfer { transmission: vec3f, source: vec3f }
fn medium_extinction_weight(tau: f32) -> f32 {
    if tau < 0.01 {
        return tau*(1.0+tau*(-0.5+tau*(1.0/6.0+tau*(-1.0/24.0+tau/120.0))));
    }
    return 1.0-exp(-tau);
}
fn medium_homogeneous(extinction: vec3f, source_per_m: vec3f, length_m: f32) -> MediumTransfer {
    let tau=extinction*length_m;
    var source=vec3f(0.0);
    for (var channel=0u;channel<3u;channel+=1u) {
        var integral=length_m;
        if tau[channel]>0.0 && tau[channel]<1.0 {
            integral=length_m*(medium_extinction_weight(tau[channel])/tau[channel]);
        } else if tau[channel]>=1.0 {
            integral=medium_extinction_weight(tau[channel])/extinction[channel];
        }
        if tau[channel]>=1.0 && extinction[channel]>=1.0 {
            // Avoid a subnormal reciprocal before multiplication by a large
            // source. The ratio is bounded by the admitted finite source here.
            let q=frexp(source_per_m[channel]);
            let s=frexp(extinction[channel]);
            let quotient=ldexp(q.fract/s.fract,q.exp-s.exp);
            source[channel]=quotient*medium_extinction_weight(tau[channel]);
        } else {
            source[channel]=source_per_m[channel]*integral;
        }
    }
    return MediumTransfer(exp(-tau),source);
}
// front is closer to the viewer; this operation is generally not commutative.
fn medium_compose(front: MediumTransfer, back: MediumTransfer) -> MediumTransfer {
    return MediumTransfer(front.transmission*back.transmission,
        front.source+front.transmission*back.source);
}
fn medium_apply(segment: MediumTransfer, background: vec3f) -> vec3f {
    return segment.source+segment.transmission*background;
}
