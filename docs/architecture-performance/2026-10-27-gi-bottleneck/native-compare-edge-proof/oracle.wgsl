
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{
 let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.5,1.);
}
@fragment fn fragment(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{
 let index=u32(p.x)+u32(p.y)*1920u;
 if index%6u==0u{return 0.;}if index%6u==1u{return 1.;}if index%6u==2u{return bitcast<f32>(0x80000000u);}
 return bitcast<f32>(0x3f000000u|((index*16777619u)&0x007fffffu));
}
@group(0) @binding(0) var original:texture_depth_2d;
@group(0) @binding(1) var comparison:sampler_comparison;
@group(0) @binding(2) var<storage,read_write> result:array<vec2<u32>>;
@compute @workgroup_size(8,8) fn compare(@builtin(global_invocation_id) id:vec3<u32>){
 if any(id.xy>=textureDimensions(original)){return;}
 let p=vec2<i32>(id.xy);let index=id.x+id.y*1920u;
 let z=textureLoad(original,p,0);let bits=bitcast<u32>(z);
 let references=array<f32,11>(-0.,0.,1.,bitcast<f32>(max(bits,1u)-1u),z,bitcast<f32>(bits+1u),1.1,-0.1,bitcast<f32>(0x7fc00000u),bitcast<f32>(0x7f800000u),bitcast<f32>(0xff800000u));
 var mask=0u;
 for(var c=0u;c<11u;c++) {let expected=z<1.&&references[c]>=z;let actual=archive_gi_depth_crossed(p,references[c]);if expected!=actual {mask|=1u<<c;}}
 result[index]=vec2<u32>(mask,bits);
}



// Exact nearest comparison preserves all original binary-search decisions.

fn archive_gi_depth_crossed(pixel:vec2<i32>,reference:f32)->bool {
 let uv=(vec2<f32>(pixel)+0.5)/vec2<f32>(textureDimensions(original));
 let crossed=textureSampleCompareLevel(original,comparison,uv,reference)>0.;
 // A clear-depth texel must remain a miss even beyond the far clip plane.
 return crossed&&(reference<1.||textureLoad(original,pixel,0)<1.);
}
