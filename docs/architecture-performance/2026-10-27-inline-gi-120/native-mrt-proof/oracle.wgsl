// Original RGBA16Float radiance and Depth32Float history, without loss.
// Blue stays a Float32 word; R/G were already half values at their source.
struct ArchiveHistoryRecord {rg:u32,blue:u32,depth:u32}
fn archive_history_record(colour:vec3<f32>,depth:f32)->ArchiveHistoryRecord {
 return ArchiveHistoryRecord(pack2x16float(colour.rg),bitcast<u32>(colour.b),bitcast<u32>(depth));
}
fn archive_history_record_colour(record:ArchiveHistoryRecord)->vec3<f32> {
 return vec3<f32>(unpack2x16float(record.rg),bitcast<f32>(record.blue));
}
// Metal RGBA16Float render-target writes truncate finite values toward zero.
// Derive the adjacent representable half from pack's nearest rounding.
fn archive_history_target_half(v:f32)->f32 {
 let bits=pack2x16float(vec2<f32>(v,0.))&65535u;
 let rounded=unpack2x16float(bits).x;
 return unpack2x16float(select(bits,bits-1u,abs(rounded)>abs(v))).x;
}
struct ArchiveHistoryOutput {@location(0) colour:vec4<f32>,@location(1) history:vec4<u32>}
fn archive_history_output(colour:vec4<f32>,depth:f32)->ArchiveHistoryOutput {
 // Match the original RGBA16Float target's rounding, without reducing depth.
 let record=archive_history_record(vec3<f32>(archive_history_target_half(colour.r),archive_history_target_half(colour.g),archive_history_target_half(colour.b)),depth);
 return ArchiveHistoryOutput(colour,vec4<u32>(record.rg,record.blue,record.depth,0u));
}


@group(0) @binding(0) var fixture:texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.,1.);}
struct Output {@location(0) colour:vec4<f32>,@location(1) history:vec4<u32>,@builtin(frag_depth) depth:f32}
@fragment fn fragment(@builtin(position) p:vec4<f32>)->Output {
 if p.y>=255. {discard;}
 let output=archive_history_output(textureLoad(fixture,vec2<i32>(p.xy),0),0.375);
 return Output(output.colour,output.history,0.375);
}
