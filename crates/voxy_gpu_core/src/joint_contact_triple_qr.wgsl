// Qualification-only serial QR of the ORIGINAL rectangular active operator.
// Arithmetic library is prepended by the test; no production dispatch uses this.
@group(0) @binding(0) var<storage,read_write> words:array<u32>;
fn read3(offset:u32)->vec3<f32> {
 return vec3<f32>(bitcast<f32>(words[offset]),bitcast<f32>(words[offset+1u]),bitcast<f32>(words[offset+2u]));
}
fn write3(offset:u32,value:vec3<f32>) {
 for(var part=0u;part<3u;part++) {words[offset+part]=bitcast<u32>(value[part]);}
}
fn finite3(value:vec3<f32>)->bool {return all(abs(value)<=vec3<f32>(3.402823466e38));}
@compute @workgroup_size(1)
fn cs_main() {
 let count=words[0];let width=words[1];let input=2u;
 let bounds=input+3u*count*width;
 let q=bounds+3u*count;let r=q+3u*count*width;
 let state=r+3u*count*count;let reaction=state+3u*width;
 let status=reaction+3u*count;
 words[status]=1u;
 for(var column=0u;column<count;column++) {
  // Exact duplicate/opposite input columns are dependent. Do not use a
  // magnitude cutoff: arbitrarily small independent directions remain eligible.
  for(var previous=0u;previous<column;previous++) {
   var identical=true;var opposite=true;
   for(var coord=0u;coord<width;coord++) {
    let a=read3(input+3u*(column*width+coord));let b=read3(input+3u*(previous*width+coord));
    identical=identical&&all(a==b);opposite=opposite&&all(a==-b);
   }
   if identical||opposite {words[status]=8u;return;}
  }
  for(var coord=0u;coord<width;coord++) {write3(q+3u*(column*width+coord),read3(input+3u*(column*width+coord)));}
  for(var sweep=0u;sweep<2u;sweep++) {
   for(var previous=0u;previous<column;previous++) {
    var dot=vec3<f32>(0.0);
    for(var coord=0u;coord<width;coord++) {dot=triple_add(dot,triple_mul(read3(q+3u*(previous*width+coord)),read3(q+3u*(column*width+coord))));}
    let entry=r+3u*(previous*count+column);write3(entry,triple_add(read3(entry),dot));
    for(var coord=0u;coord<width;coord++) {
     let offset=q+3u*(column*width+coord);
     write3(offset,triple_add(read3(offset),-triple_mul(read3(q+3u*(previous*width+coord)),dot)));
    }
   }
  }
  var squared=vec3<f32>(0.0);
  for(var coord=0u;coord<width;coord++) {let value=read3(q+3u*(column*width+coord));squared=triple_add(squared,triple_mul(value,value));}
  if !finite3(squared)||squared.x<=0.0 {words[status]=2u;return;}
  let norm=triple_sqrt(squared);if !finite3(norm)||norm.x<=0.0 {words[status]=3u;return;}
  write3(r+3u*(column*count+column),norm);
  for(var coord=0u;coord<width;coord++) {
   let offset=q+3u*(column*width+coord);let value=triple_div(read3(offset),norm);
   if !finite3(value) {words[status]=4u;return;}write3(offset,value);
  }
 }
 // R^T y = original normalized bounds. Reaction slots hold y temporarily.
 for(var row=0u;row<count;row++) {
  var rhs=read3(bounds+3u*row);
  for(var previous=0u;previous<row;previous++) {rhs=triple_add(rhs,-triple_mul(read3(r+3u*(previous*count+row)),read3(reaction+3u*previous)));}
  let value=triple_div(rhs,read3(r+3u*(row*count+row)));if !finite3(value) {words[status]=5u;return;}
  write3(reaction+3u*row,value);
 }
 for(var coord=0u;coord<width;coord++) {
  var value=vec3<f32>(0.0);
  for(var column=0u;column<count;column++) {value=triple_add(value,triple_mul(read3(q+3u*(column*width+coord)),read3(reaction+3u*column)));}
  if !finite3(value) {words[status]=6u;return;}write3(state+3u*coord,value);
 }
 // R lambda = y, descending in place.
 for(var reverse=0u;reverse<count;reverse++) {
  let row=count-1u-reverse;var rhs=read3(reaction+3u*row);
  for(var next=row+1u;next<count;next++) {rhs=triple_add(rhs,-triple_mul(read3(r+3u*(row*count+next)),read3(reaction+3u*next)));}
  let value=triple_div(rhs,read3(r+3u*(row*count+row)));if !finite3(value) {words[status]=7u;return;}
  write3(reaction+3u*row,value);
 }
 words[status]=0u;
}
