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
 let center=p+u*offset.x+v*offset.y+w*offset.z;
 var normal=u;
 switch vertex%4u { case 1u:{normal=v;} case 2u:{normal=-u;} case 3u:{normal=-v;} default:{} }
 let out=data[4]+vertex*12u;
 store(out,center+normal*(scalar(5u)*(1.0-scalar(6u)*arc)));
 data[out+3u]=bitcast<u32>(-7.0);data[out+4u]=bitcast<u32>(arc);
 store(out+5u,vec3f(scalar(frame+7u),scalar(frame+11u),scalar(frame+15u))*scalar(data[3]+follower*4u+3u));
 data[out+8u]=bitcast<u32>(1.0);
 store(out+9u,normal);
}
