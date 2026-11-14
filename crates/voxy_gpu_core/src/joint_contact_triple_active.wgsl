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

var<private> active_ids:array<u32,1024>;
var<private> flags:array<u32,1024>;
fn negative(value:vec3<f32>)->bool {
 if value.x!=0.0 {return value.x<0.0;}if value.y!=0.0 {return value.y<0.0;}return value.z<0.0;
}
fn less(a:vec3<f32>,b:vec3<f32>)->bool {return negative(triple_add(a,-b));}
fn factor_solve(selected:u32,count:u32,width:u32,input:u32,bounds:u32,q:u32,r:u32,state:u32,reaction:u32,status:u32) {
 words[status]=1u;
 for(var i=0u;i<3u*count*count;i++) {words[r+i]=0u;}
 for(var column=0u;column<selected;column++) {
  // Exact duplicate/opposite input columns are dependent. Do not use a
  // magnitude cutoff: arbitrarily small independent directions remain eligible.
  for(var previous=0u;previous<column;previous++) {
   var identical=true;var opposite=true;
   for(var coord=0u;coord<width;coord++) {
    let a=read3(input+3u*(active_ids[column]*width+coord));let b=read3(input+3u*(active_ids[previous]*width+coord));
    identical=identical&&all(a==b);opposite=opposite&&all(a==-b);
   }
   if identical||opposite {words[status]=8u;return;}
  }
  for(var coord=0u;coord<width;coord++) {write3(q+3u*(column*width+coord),read3(input+3u*(active_ids[column]*width+coord)));}
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
 for(var row=0u;row<selected;row++) {
  var rhs=read3(bounds+3u*active_ids[row]);
  for(var previous=0u;previous<row;previous++) {rhs=triple_add(rhs,-triple_mul(read3(r+3u*(previous*count+row)),read3(reaction+3u*previous)));}
  let value=triple_div(rhs,read3(r+3u*(row*count+row)));if !finite3(value) {words[status]=5u;return;}
  write3(reaction+3u*row,value);
 }
 for(var coord=0u;coord<width;coord++) {
  var value=vec3<f32>(0.0);
  for(var column=0u;column<selected;column++) {value=triple_add(value,triple_mul(read3(q+3u*(column*width+coord)),read3(reaction+3u*column)));}
  if !finite3(value) {words[status]=6u;return;}write3(state+3u*coord,value);
 }
 // R lambda = y, descending in place.
 for(var reverse=0u;reverse<selected;reverse++) {
  let row=selected-1u-reverse;var rhs=read3(reaction+3u*row);
  for(var next=row+1u;next<selected;next++) {rhs=triple_add(rhs,-triple_mul(read3(r+3u*(row*count+next)),read3(reaction+3u*next)));}
  let value=triple_div(rhs,read3(r+3u*(row*count+row)));if !finite3(value) {words[status]=7u;return;}
  write3(reaction+3u*row,value);
 }
 words[status]=0u;
}

@compute @workgroup_size(1)
fn cs_main() {
 let count=words[0];let width=words[1];let limit=words[2];let input=4u;
 let bounds=input+3u*count*width;let tolerance=bounds+3u*count;
 let q=tolerance+3u;let r=q+3u*count*width;
 let candidate=r+3u*count*count;let candidate_reactions=candidate+3u*width;
 let state=candidate_reactions+3u*count;let reactions=state+3u*width;let status=reactions+3u*count;
 words[status]=1u;if count==0u||count>1024u {words[status]=9u;return;}
 let epsilon=read3(tolerance);var selected=0u;
 for(var row=0u;row<count;row++) {flags[row]=0u;write3(reactions+3u*row,vec3<f32>(0.0));}
 for(var coord=0u;coord<width;coord++) {write3(state+3u*coord,vec3<f32>(0.0));}
 for(var iteration=0u;iteration<limit;iteration++) {
  words[3]=iteration+1u;
  if selected>0u {
   factor_solve(selected,count,width,input,bounds,q,r,candidate,candidate_reactions,status);
   if words[status]!=0u {return;}words[status]=1u;
   var released=count;var fraction=vec3<f32>(1.0,0.0,0.0);
   for(var slot=0u;slot<selected;slot++) {
    let next=read3(candidate_reactions+3u*slot);
    if negative(next) {
     let old=read3(reactions+3u*active_ids[slot]);let step=triple_div(old,triple_add(old,-next));
     if !finite3(step)||negative(step)||less(vec3<f32>(1.0,0.0,0.0),step) {words[status]=10u;return;}
     if released==count||less(step,fraction) {released=slot;fraction=step;}
    }
   }
   if released<count {
    for(var coord=0u;coord<width;coord++) {let old=read3(state+3u*coord);write3(state+3u*coord,triple_add(old,triple_mul(fraction,triple_add(read3(candidate+3u*coord),-old))));}
    for(var slot=0u;slot<selected;slot++) {
     let index=active_ids[slot];let old=read3(reactions+3u*index);
     var value=triple_add(old,triple_mul(fraction,triple_add(read3(candidate_reactions+3u*slot),-old)));
     if !finite3(value) {words[status]=11u;return;}if negative(value) {value=vec3<f32>(0.0);}write3(reactions+3u*index,value);
    }
    let index=active_ids[released];flags[index]=0u;write3(reactions+3u*index,vec3<f32>(0.0));
    for(var slot=released;slot+1u<selected;slot++) {active_ids[slot]=active_ids[slot+1u];}selected-=1u;continue;
   }
   for(var coord=0u;coord<width;coord++) {write3(state+3u*coord,read3(candidate+3u*coord));}
   for(var row=0u;row<count;row++) {write3(reactions+3u*row,vec3<f32>(0.0));}
   for(var slot=0u;slot<selected;slot++) {write3(reactions+3u*active_ids[slot],read3(candidate_reactions+3u*slot));}
  }
  var complete=true;var enter=count;var worst=vec3<f32>(0.0);
  for(var row=0u;row<count;row++) {
   var dot=vec3<f32>(0.0);
   for(var coord=0u;coord<width;coord++) {dot=triple_add(dot,triple_mul(read3(input+3u*(row*width+coord)),read3(state+3u*coord)));}
   let gap=triple_add(dot,-read3(bounds+3u*row));if !finite3(gap) {words[status]=12u;return;}
   let positive=less(vec3<f32>(0.0),read3(reactions+3u*row));
   if (positive&&(less(gap,-epsilon)||less(epsilon,gap)))||(!positive&&less(gap,-epsilon)) {complete=false;}
   if flags[row]==0u&&less(gap,-epsilon)&&(enter==count||less(gap,worst)) {enter=row;worst=gap;}
  }
  if complete {words[status]=0u;return;}
  if enter==count||selected==count {words[status]=13u;return;}
  active_ids[selected]=enter;flags[enter]=1u;selected+=1u;
 }
 words[status]=14u;
}
