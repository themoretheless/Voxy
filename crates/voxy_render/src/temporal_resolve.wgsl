@group(0) @binding(0) var current: texture_2d<f32>;
@group(0) @binding(1) var motion: texture_2d<f32>;
@group(0) @binding(2) var history: texture_2d<f32>;
@group(0) @binding(3) var expected_depth: texture_2d<f32>;
@group(0) @binding(4) var history_depth: texture_2d<f32>;
@group(0) @binding(5) var output: texture_storage_2d<rgba32float,write>;
@group(0) @binding(6) var<uniform> settings: vec4<f32>;
fn finite_channels(v:vec3<f32>)->vec3<bool> {return (bitcast<vec3<u32>>(v)&vec3<u32>(0x7f800000u))!=vec3<u32>(0x7f800000u);}
fn finite3(v:vec3<f32>)->bool {return all(finite_channels(v));}
@compute @workgroup_size(8,8) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    let size=textureDimensions(output);
    if any(id.xy>=size) {return;}
    let pixel=vec2<i32>(id.xy);
    var color=textureLoad(current,pixel,0).rgb;
    color=select(vec3<f32>(0.),max(color,vec3<f32>(0.)),finite_channels(color));
    if settings.z==0. {
        let delta=textureLoad(motion,pixel,0).xy;
        let uv=(vec2<f32>(id.xy)+vec2<f32>(.5))/vec2<f32>(size)+delta;
        // Reject before integer conversion; never clamp/wrap off-screen history.
        if finite3(vec3<f32>(delta,0.)) && finite3(vec3<f32>(uv,0.)) && all(uv>=vec2<f32>(0.)) && all(uv<vec2<f32>(1.)) {
            let old_pixel=vec2<i32>(uv*vec2<f32>(size));
            let expected=textureLoad(expected_depth,pixel,0).x;
            let actual=textureLoad(history_depth,old_pixel,0).x;
            if finite3(vec3<f32>(expected,actual,0.)) && expected>0. && expected<1. && actual>0. && actual<1. && abs(expected-actual)<=settings.y {
                var old=textureLoad(history,old_pixel,0).rgb;
                if finite3(old) && all(old>=vec3<f32>(0.)) {
                    if settings.w!=0. {
                        var low=color; var high=color;
                        for(var y:i32=-1; y<=1; y+=1) {
                            for(var x:i32=-1; x<=1; x+=1) {
                                let neighbor=pixel+vec2<i32>(x,y);
                                if all(neighbor>=vec2<i32>(0)) && all(neighbor<vec2<i32>(size)) {
                                    let sample=textureLoad(current,neighbor,0).rgb;
                                    if finite3(sample) {
                                        low=min(low,max(sample,vec3<f32>(0.)));
                                        high=max(high,max(sample,vec3<f32>(0.)));
                                    }
                                }
                            }
                        }
                        old=clamp(old,low,high);
                    }
                    color=old+(color-old)*(1.-settings.x);
                }
            }
        }
    }
    textureStore(output,pixel,vec4<f32>(color,1.));
}
