// Shared display transforms for the archive post and temporal resolve passes.
// mode: 0 = ACES fit (SDR), 1 = hue-preserving EDR knee (HDR), 2 = AgX (SDR).
// SDR results are linear; the sRGB target applies the only transfer function.
// AgX base (Sobotka), sigmoid polynomial fit. Returns linear sRGB so the
// sRGB render target applies the only transfer function.
fn agx(x:vec3<f32>)->vec3<f32> {
    let inset=mat3x3<f32>(vec3<f32>(0.842479062253094,0.0423282422610123,0.0423756549057051),vec3<f32>(0.0784335999999992,0.878468636469772,0.0784336),vec3<f32>(0.0792237451477643,0.0791661274605434,0.879142973793104));
    let outset=mat3x3<f32>(vec3<f32>(1.19687900512017,-0.0528968517574562,-0.0529716355144438),vec3<f32>(-0.0980208811401368,1.15190312990417,-0.0980434501171241),vec3<f32>(-0.0990297440797205,-0.0989611768448433,1.15107367264116));
    let min_ev=-12.47393;
    let max_ev=4.026069;
    var v=clamp(log2(max(inset*x,vec3<f32>(1e-10))),vec3<f32>(min_ev),vec3<f32>(max_ev));
    v=(v-min_ev)/(max_ev-min_ev);
    let v2=v*v;
    let v4=v2*v2;
    v=15.5*v4*v2-40.14*v4*v+31.96*v4-6.868*v2*v+0.4298*v2+0.1191*v-0.00232;
    return pow(max(outset*v,vec3<f32>(0.)),vec3<f32>(2.2));
}
fn display_transform(x:vec3<f32>,mode:f32,headroom:f32)->vec3<f32> {
    if mode>1.5 {return agx(x);}
    if mode<0.5 {
        return clamp((x*(2.51*x+0.03))/(x*(2.43*x+0.59)+0.14),vec3<f32>(0.),vec3<f32>(1.));
    }
    // Preserve linear midtones and hue; smoothly compress only highlights into
    // the current display headroom, rather than clipping everything above 1.
    let peak=max(x.r,max(x.g,x.b));
    let knee=0.75;
    if peak<=knee { return x; }
    let span=max(headroom-knee,0.0001);
    let mapped=knee+span*(1.-exp(-(peak-knee)/span));
    return x*(mapped/peak);
}
