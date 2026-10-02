@group(0) @binding(0) var current: texture_2d<f32>;
@group(0) @binding(1) var motion: texture_2d<f32>;
@group(0) @binding(2) var expected_depth: texture_2d<f32>;
@group(0) @binding(3) var history: texture_2d<f32>;
@group(0) @binding(4) var history_depth: texture_2d<f32>;
@group(0) @binding(5) var resolved: texture_2d<f32>;
@group(0) @binding(6) var<storage,read> pixels: array<vec2<u32>>;
struct Sample { current:vec4<f32>, old:vec4<f32>, resolved:vec4<f32>, guide:vec4<f32>, neighbors:array<vec4<f32>,9> }
@group(0) @binding(7) var<storage,read_write> samples: array<Sample>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=arrayLength(&pixels) {return;}
    let pixel=vec2<i32>(pixels[id.x]);
    let size=textureDimensions(current);
    let delta=textureLoad(motion,pixel,0).xy;
    let uv=(vec2<f32>(pixel)+0.5)/vec2<f32>(size)+delta;
    var sample:Sample;
    sample.current=textureLoad(current,pixel,0);
    sample.resolved=textureLoad(resolved,pixel,0);
    sample.guide=vec4<f32>(delta,textureLoad(expected_depth,pixel,0).r,0.);
    sample.old=vec4<f32>(0.);
    let bits=bitcast<vec2<u32>>(uv)&vec2<u32>(0x7f800000u);
    if all(bits!=vec2<u32>(0x7f800000u)) && all(uv>=vec2<f32>(0.)) && all(uv<vec2<f32>(1.)) {
        let old_pixel=vec2<i32>(uv*vec2<f32>(size));
        sample.old=textureLoad(history,old_pixel,0);
        sample.guide.w=textureLoad(history_depth,old_pixel,0).r;
    }
    for(var y:i32=-1;y<=1;y++) {
        for(var x:i32=-1;x<=1;x++) {
            let p=pixel+vec2<i32>(x,y);
            let index=u32((y+1)*3+x+1);
            sample.neighbors[index]=vec4<f32>(0.);
            if all(p>=vec2<i32>(0)) && all(p<vec2<i32>(size)) {
                sample.neighbors[index]=vec4<f32>(textureLoad(current,p,0).rgb,1.);
            }
        }
    }
    samples[id.x]=sample;
}
