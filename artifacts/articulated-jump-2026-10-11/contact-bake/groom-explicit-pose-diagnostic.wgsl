// Packed words avoid vec3 alignment differences with the native vertex ABI.
@group(0) @binding(0) var<storage,read_write> data:array<u32>;
fn scalar(i:u32)->f32 { return bitcast<f32>(data[i]); }
fn vector(i:u32)->vec3f { return vec3f(scalar(i),scalar(i+1u),scalar(i+2u)); }
fn store(i:u32,v:vec3f) { data[i]=bitcast<u32>(v.x);data[i+1u]=bitcast<u32>(v.y);data[i+2u]=bitcast<u32>(v.z); }
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3u) {
 let vertex=id.x;
 if vertex>=data[0] { return; }
 let follower=vertex/4u;
 let frame=data[2]+(follower/data[1])*16u;
 let p=vector(frame); let arc=scalar(frame+3u);
 let u=vector(frame+4u);let v=vector(frame+8u);let w=vector(frame+12u);
 let offset=vector(data[3]+follower*4u);
 let center=p+(u*offset.x+v*offset.y+w*offset.z);
 let t=data[7];
 let pose=mat4x4f(vec4f(vector(t),scalar(t+3u)),vec4f(vector(t+4u),scalar(t+7u)),vec4f(vector(t+8u),scalar(t+11u)),vec4f(vector(t+12u),scalar(t+15u)));
 var normal=u;
 switch vertex%4u { case 1u:{normal=v;} case 2u:{normal=-u;} case 3u:{normal=-v;} default:{} }
 let out=data[4]+vertex*12u;
 let local_point=center+normal*(scalar(5u)*(1.0-scalar(6u)*arc));
 store(out,((pose[0].xyz*local_point.x+pose[1].xyz*local_point.y)+pose[2].xyz*local_point.z)+pose[3].xyz);
 data[out+3u]=bitcast<u32>(-7.0);data[out+4u]=bitcast<u32>(arc);
 store(out+5u,vec3f(scalar(frame+7u),scalar(frame+11u),scalar(frame+15u))*scalar(data[3]+follower*4u+3u));
 data[out+8u]=bitcast<u32>(1.0);
 // Untransformed simulation frames retain their authored normals exactly.
 var draw_normal=normal;
 if any(pose[0]!=vec4f(1.0,0.0,0.0,0.0)) || any(pose[1]!=vec4f(0.0,1.0,0.0,0.0))
    || any(pose[2]!=vec4f(0.0,0.0,1.0,0.0)) || any(pose[3]!=vec4f(0.0,0.0,0.0,1.0)) {
    draw_normal=normalize(((pose[0].xyz*normal.x+pose[1].xyz*normal.y)+pose[2].xyz*normal.z));
 }
 store(out+9u,draw_normal);
}
