// Original RGBA16Float radiance and Depth32Float history, without loss.
// Blue stays a Float32 word; R/G were already half values at their source.
struct ArchiveHistoryRecord {rg:u32,blue:u32,depth:u32}
fn archive_history_record(colour:vec3<f32>,depth:f32)->ArchiveHistoryRecord {
 return ArchiveHistoryRecord(pack2x16float(colour.rg),bitcast<u32>(colour.b),bitcast<u32>(depth));
}
fn archive_history_record_colour(record:ArchiveHistoryRecord)->vec3<f32> {
 return vec3<f32>(unpack2x16float(record.rg),bitcast<f32>(record.blue));
}


@group(0) @binding(0) var colour:texture_2d<f32>;
@group(0) @binding(1) var original:texture_depth_2d;
@group(0) @binding(2) var<storage,read_write> records:array<ArchiveHistoryRecord>;
@group(0) @binding(3) var copied:texture_depth_2d;
@group(0) @binding(4) var<storage,read_write> errors:array<vec4<u32>>;
@compute @workgroup_size(8,8) fn pack(@builtin(global_invocation_id) id:vec3<u32>){let size=textureDimensions(colour);if any(id.xy>=size){return;}let p=vec2<i32>(id.xy);records[id.y*size.x+id.x]=archive_history_record(textureLoad(colour,p,0).rgb,textureLoad(original,p,0));}
@compute @workgroup_size(8,8) fn check(@builtin(global_invocation_id) id:vec3<u32>){let size=textureDimensions(colour);if any(id.xy>=size){return;}let p=vec2<i32>(id.xy);let i=id.y*size.x+id.x;let record=records[i];let a=vec4<f32>(textureLoad(colour,p,0).rgb,textureLoad(copied,p,0));let b=vec4<f32>(archive_history_record_colour(record),bitcast<f32>(record.depth));errors[i]=vec4<u32>(select(vec4<u32>(0u),vec4<u32>(1u),bitcast<vec4<u32>>(a)!=bitcast<vec4<u32>>(b)));}
