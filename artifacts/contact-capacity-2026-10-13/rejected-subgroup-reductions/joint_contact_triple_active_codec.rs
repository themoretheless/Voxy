//! Experimental transport boundary. Original physical admission owns publication.
use super::{ExperimentalTripleContactInput,EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,JointDotError};
use super::super::rescale;
#[derive(Debug)]
struct Layout {immutable:usize,state:usize,reactions:usize,status:usize,total:usize}
impl Layout {
 fn new(rows:usize,width:usize)->Result<Self,JointDotError> {
  if rows==0||width==0 {return Err(JointDotError::Shape);}
  if rows>1024 {return Err(JointDotError::Capacity);}
  let checked=|| {
   let matrix=rows.checked_mul(width)?.checked_mul(3)?;
   let square=rows.checked_mul(rows)?.checked_mul(3)?;
   let vector=width.checked_mul(3)?;let row_words=rows.checked_mul(3)?;
   let immutable=4usize.checked_add(matrix)?.checked_add(row_words)?.checked_add(3)?;
   let state=immutable.checked_add(matrix)?.checked_add(square)?.checked_add(vector)?.checked_add(row_words)?;
   let reactions=state.checked_add(vector)?;let status=reactions.checked_add(row_words)?;
   let total=status.checked_add(1)?;
   (total<=16*1024*1024).then_some(Self {immutable,state,reactions,status,total})
  };
  checked().ok_or(JointDotError::Capacity)
 }
}
/// One complete original operator. A decoded result remains a candidate only.
#[derive(Debug)]
pub struct ExperimentalTripleActiveContactInput {
 words:Vec<u32>,layout:Layout,rows:usize,width:usize,state_scale:f64,inverse_column_scale:f64,
}
/// Private checked QR scratch, valid only for the parallel experimental kernel.
#[derive(Debug)]
pub struct ExperimentalTripleActiveFactors {
 width:usize,columns:Vec<u32>,qr:Vec<u32>,selected:usize,column_scale_bits:u64,
}
impl ExperimentalTripleActiveFactors {
 /// Retained payload size, excluding the small fixed struct overhead.
 pub fn retained_bytes(&self)->usize {(self.columns.len()+self.qr.len())*4}
 /// Exact ordered active-vector identity; inactive rows and bounds do not define QR.
 pub fn same_basis(&self,other:&Self)->bool {
  self.width==other.width&&self.selected==other.selected&&self.column_scale_bits==other.column_scale_bits
   &&self.columns==other.columns
 }

}
/// One workgroup per complete original operator in a single dispatch.
/// No prefix result is returned if any candidate or immutable word fails.
#[derive(Debug)]
pub struct ExperimentalTripleActiveContactBatch {
 inputs:Vec<ExperimentalTripleActiveContactInput>,words:Vec<u32>,offsets:Vec<usize>,
}
impl ExperimentalTripleActiveContactBatch {
 pub fn shader_source()->String {ExperimentalTripleActiveContactInput::parallel_source(true,false)}
 pub fn new(inputs:Vec<ExperimentalTripleActiveContactInput>)->Result<Self,JointDotError> {
  if inputs.is_empty() {return Err(JointDotError::Shape);}
  if inputs.len()>65535 {return Err(JointDotError::Capacity);}
  let prefix=inputs.len()+2;
  let total=inputs.iter().try_fold(prefix,|sum,input|sum.checked_add(input.words.len()).filter(|v|*v<=16*1024*1024)).ok_or(JointDotError::Capacity)?;
  let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|JointDotError::Capacity)?;
  words.resize(prefix,0);words[0]=inputs.len() as u32;
  let mut offsets=Vec::with_capacity(inputs.len()+1);
  for (i,input) in inputs.iter().enumerate() {offsets.push(words.len());words[i+1]=words.len() as u32;words.extend_from_slice(&input.words);}
  offsets.push(words.len());words[prefix-1]=words.len() as u32;
  Ok(Self {inputs,words,offsets})
 }
 pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
 pub fn workgroups(&self)->[u32;3] {[self.inputs.len() as u32,1,1]}
 pub fn inputs(&self)->&[ExperimentalTripleActiveContactInput] {&self.inputs}
 /// Immutable table followed by each operator's independently audited ranges.
 pub fn compact_readback_ranges(&self)->Vec<std::ops::Range<u64>> {
  let mut ranges=vec![0..((self.inputs.len()+2)*4) as u64];
  for (i,input) in self.inputs.iter().enumerate() {
   let base=(self.offsets[i]*4) as u64;
   ranges.extend(input.compact_readback_ranges().into_iter().map(|r|r.start+base..r.end+base));
  }
  ranges
 }
 /// Validate the complete table and length before exposing per-operator bytes.
 /// Each slice still requires its original input's candidate gate.
 pub fn operator_readbacks<'a>(&self,bytes:&'a [u8],compact:bool)->Result<Vec<&'a [u8]>,JointDotError> {
  let prefix=(self.inputs.len()+2)*4;
  let lengths=self.inputs.iter().map(|input|if compact {
   input.compact_readback_ranges().iter().map(|r|(r.end-r.start) as usize).sum()
  } else {input.bytes().len()}).collect::<Vec<usize>>();
  if bytes.len()!=prefix+lengths.iter().sum::<usize>()||bytes[..prefix]!=self.bytes()[..prefix] {return Err(JointDotError::Output);}
  let mut offset=prefix;
  Ok(lengths.into_iter().map(|len|{let start=offset;offset+=len;&bytes[start..offset]}).collect())
 }

 pub fn candidate(&self,bytes:&[u8])->Result<Vec<(Vec<f64>,Vec<f64>)>,JointDotError> {
  let prefix=(self.inputs.len()+2)*4;
  if bytes.len()!=self.words.len()*4||bytes[..prefix]!=self.bytes()[..prefix] {return Err(JointDotError::Output);}
  self.inputs.iter().enumerate().map(|(i,input)|input.candidate(&bytes[4*self.offsets[i]..4*self.offsets[i+1]])).collect()
 }
}
impl ExperimentalTripleActiveContactInput {
 pub fn shader_source()->String {format!("{}\n{}",EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,include_str!("joint_contact_triple_active.wgsl"))}
 /// Qualification-only 256-lane alternative using the identical checked payload.
 pub fn parallel_shader_source()->String {Self::parallel_source(false,false)}
 /// Experimental shuffle transport. Requires SUBGROUP and a checked 32-lane mapping.
 pub fn parallel_subgroup_shader_source(batched:bool)->String {Self::parallel_source(batched,true)}
 fn parallel_source(batched:bool,subgroups:bool)->String {
  let mut body=include_str!("joint_contact_triple_parallel.wgsl").to_owned();
  if subgroups {
   body=body.replace("group:vec3<u32>)", "group:vec3<u32>,@builtin(subgroup_size) subgroup_width:u32,@builtin(subgroup_invocation_id) subgroup_lane:u32,@builtin(subgroup_id) subgroup_index:u32)");
   let marker="    // Hints select rows only.";
   let check="    if subgroup_width!=32u||subgroup_lane!=lane%32u||subgroup_index!=lane/32u {atomicOr(&invalid,32u);}
    workgroupBarrier();
    if atomicLoad(&invalid)!=0u {if lane==0u {data[operator_base+status]=0x120u;}return;}
";
   body=body.replace(marker,&format!("{check}{marker}"));
  }
  format!("{}{}\nconst TRIPLE_BATCHED:bool={batched};\n{}\n{}",
   "",EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,
   if subgroups {include_str!("joint_contact_triple_subgroup_reductions.wgsl")} else {include_str!("joint_contact_triple_reductions.wgsl")},body)
 }
 /// Full checked payload size; dimensions only, before coefficient packing.
 pub fn required_bytes(rows:usize,width:usize)->Result<usize,JointDotError> {Ok(Layout::new(rows,width)?.total*4)}
 pub fn new(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64)->Result<Self,JointDotError> {
  let rows=columns.len();let width=columns.first().map_or(0,Vec::len);
  // Account for input, Q, R and all candidate/live output buffers before packing.
  let layout=Layout::new(rows,width)?;
  let packed=ExperimentalTripleContactInput::new(columns,bounds,tolerance)?;
  let state_scale=packed.bound_scale/packed.column_scale;let inverse_column_scale=packed.column_scale.recip();
  if !state_scale.is_finite()||state_scale<=0.||!inverse_column_scale.is_finite() {return Err(JointDotError::NumericRange);}
  let mut words=Vec::new();words.try_reserve_exact(layout.total).map_err(|_|JointDotError::Capacity)?;
  words.extend([rows as u32,width as u32,512u32.max(rows as u32+1),0]);
  words.extend(packed.columns.iter().flatten().copied());words.extend(packed.bounds.iter().flatten().copied());words.extend(packed.tolerance);
  debug_assert_eq!(words.len(),layout.immutable);words.resize(layout.total,0);words[layout.status]=u32::MAX;
  Ok(Self {words,layout,rows,width,state_scale,inverse_column_scale})
 }
 /// Experimental active-row hints only; magnitudes never enter physical loads.
 /// A rejected warm candidate requires a separate cold solve by the caller.
 pub fn new_seeded(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,seeds:&[f64])->Result<Self,JointDotError> {
  let mut input=Self::new(columns,bounds,tolerance)?;
  if !seeds.is_empty()&&seeds.len()!=input.rows {return Err(JointDotError::Shape);}
  if seeds.iter().any(|v|!v.is_finite()||*v<0.) {return Err(JointDotError::NumericRange);}
  let offset=input.layout.state-3*input.rows;
  for (i,&value) in seeds.iter().enumerate() {if value>0. {input.words[offset+3*i]=1f32.to_bits();}}
  Ok(input)
 }
 /// Retain only a completed one-iteration seeded basis; no physical admission implied.
 pub fn parallel_factors(&self,bytes:&[u8])->Result<ExperimentalTripleActiveFactors,JointDotError> {
  if bytes.len()!=self.words.len()*4 {return Err(JointDotError::Output);}
  self.parallel_factors_words(|i|u32::from_le_bytes(bytes[4*i..4*i+4].try_into().unwrap()))
 }
 fn parallel_factors_words(&self,word:impl Fn(usize)->u32)->Result<ExperimentalTripleActiveFactors,JointDotError> {
  if word(3)!=1 {return Err(JointDotError::Output);}
  let flags=&self.words[self.layout.state-3*self.rows..self.layout.state];
  let selected=flags.chunks_exact(3).filter(|v|v[0]==1f32.to_bits()).count();
  if selected==0 {return Err(JointDotError::Output);}
  self.candidate_words(&word)?;
  let read=|start:usize,end:usize|(start..end).map(&word);
  // Q contains only selected vectors. R stores selected x selected entries;
  // unused rows/columns are fresh zero scratch when restored to the GPU stride.
  let q_words=3*selected*self.width;let r=self.layout.immutable+3*self.rows*self.width;
  let mut qr=read(self.layout.immutable,self.layout.immutable+q_words).collect::<Vec<_>>();
  for column in 0..selected {qr.extend(read(r+3*column*self.rows,r+3*column*self.rows+3*selected));}
  if qr.iter().any(|v|!f32::from_bits(*v).is_finite()) {return Err(JointDotError::Output);}
  let columns=flags.chunks_exact(3).enumerate().filter(|(_,v)|v[0]==1f32.to_bits())
   .flat_map(|(row,_)|self.words[4+3*row*self.width..4+3*(row+1)*self.width].iter().copied()).collect();
  Ok(ExperimentalTripleActiveFactors {width:self.width,columns,qr,selected,column_scale_bits:self.inverse_column_scale.to_bits()})
 }
 /// Apply bit-exact matching active vectors; inactive rows and bounds may change.
 /// Complete original-coordinate and native physical admission remain mandatory.
 /// Use only with parallel_shader_source; a rejected candidate must be retried cold.
 pub fn reuse_parallel_factors(&mut self,factors:&ExperimentalTripleActiveFactors)->Result<(),JointDotError> {
  if self.width!=factors.width||self.inverse_column_scale.to_bits()!=factors.column_scale_bits {return Err(JointDotError::Shape);}
  // QR depends on ordered selected vectors only. Inactive rows may change,
  // move or be appended; their complete original gap checks remain mandatory.
  let flags=&self.words[self.layout.state-3*self.rows..self.layout.state];
  if flags.chunks_exact(3).filter(|v|v[0]==1f32.to_bits()).count()!=factors.selected {return Err(JointDotError::Shape);}
  let selected_columns=flags.chunks_exact(3).enumerate().filter(|(_,v)|v[0]==1f32.to_bits())
   .flat_map(|(row,_)|self.words[4+3*row*self.width..4+3*(row+1)*self.width].iter().copied());
  if !selected_columns.eq(factors.columns.iter().copied()) {return Err(JointDotError::Shape);}
  let q_words=3*factors.selected*self.width;let r=self.layout.immutable+3*self.rows*self.width;
  self.words[self.layout.immutable..r+3*self.rows*self.rows].fill(0);
  self.words[self.layout.immutable..self.layout.immutable+q_words].copy_from_slice(&factors.qr[..q_words]);
  for column in 0..factors.selected {
   let start=q_words+3*column*factors.selected;
   self.words[r+3*column*self.rows..r+3*column*self.rows+3*factors.selected]
    .copy_from_slice(&factors.qr[start..start+3*factors.selected]);
  }
  self.words[3]=factors.selected as u32;
  Ok(())
 }
 /// Every immutable word, live result and selected QR prefix; omitted mutable
 /// scratch is never read by either decoder. R ranges use the current stride.
 pub fn compact_readback_ranges(&self)->Vec<std::ops::Range<u64>> {
  let selected=self.words[self.layout.state-3*self.rows..self.layout.state].chunks_exact(3).filter(|v|v[0]==1f32.to_bits()).count();
  let mut ranges=vec![0..((self.layout.immutable+3*selected*self.width)*4) as u64];
  let r=self.layout.immutable+3*self.rows*self.width;
  for column in 0..selected {
   let range=((r+3*column*self.rows)*4) as u64..((r+3*column*self.rows+3*selected)*4) as u64;
   if ranges.last().unwrap().end==range.start {ranges.last_mut().unwrap().end=range.end;} else {ranges.push(range);}
  }
  ranges.push((self.layout.state*4) as u64..(self.layout.total*4) as u64);ranges
 }
 fn compact_map(&self,bytes:&[u8])->Result<Vec<(std::ops::Range<usize>,usize)>,JointDotError> {
  let mut offset=0;let ranges=self.compact_readback_ranges().into_iter().map(|range| {
   let words=range.start as usize/4..range.end as usize/4;let start=offset;offset+=words.len();(words,start)
  }).collect::<Vec<_>>();
  if bytes.len()!=offset*4 {return Err(JointDotError::Output);}Ok(ranges)
 }
 fn compact_word(bytes:&[u8],map:&[(std::ops::Range<usize>,usize)],index:usize)->u32 {
  let (range,start)=map.iter().find(|(range,_)|range.contains(&index)).expect("decoder reads only audited compact ranges");
  let offset=4*(start+index-range.start);u32::from_le_bytes(bytes[offset..offset+4].try_into().unwrap())
 }
 /// Same transport gate as full readback; no omitted word is reconstructed.
 pub fn candidate_from_compact_readback(&self,bytes:&[u8])->Result<(Vec<f64>,Vec<f64>),JointDotError> {
  let map=self.compact_map(bytes)?;self.candidate_words(|i|Self::compact_word(bytes,&map,i))
 }
 pub fn parallel_factors_from_compact_readback(&self,bytes:&[u8])->Result<ExperimentalTripleActiveFactors,JointDotError> {
  let map=self.compact_map(bytes)?;self.parallel_factors_words(|i|Self::compact_word(bytes,&map,i))
 }
 pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
 /// Transport validation only. The original native owner must admit this candidate.
 pub fn candidate(&self,bytes:&[u8])->Result<(Vec<f64>,Vec<f64>),JointDotError> {
  if bytes.len()!=self.words.len()*4 {return Err(JointDotError::Output);}
  self.candidate_words(|i|u32::from_le_bytes(bytes[i*4..i*4+4].try_into().unwrap()))
 }
 fn candidate_words(&self,word:impl Fn(usize)->u32)->Result<(Vec<f64>,Vec<f64>),JointDotError> {
  if word(self.layout.status)!=0||word(3)==0||word(3)>self.words[2]
   ||(0..self.layout.immutable).any(|i|i!=3&&word(i)!=self.words[i]) {return Err(JointDotError::Output);}
  let scalar=|offset| {
   let a=f32::from_bits(word(offset)) as f64;let b=f32::from_bits(word(offset+1)) as f64;let c=f32::from_bits(word(offset+2)) as f64;
   if !a.is_finite()||!b.is_finite()||!c.is_finite() {return Err(JointDotError::Output);}
   let sum=a+b;let virtual_b=sum-a;Ok(sum+((a-(sum-virtual_b))+(b-virtual_b)+c))
  };
  let mut state=Vec::new();state.try_reserve_exact(self.width).map_err(|_|JointDotError::Capacity)?;
  for i in 0..self.width {
   let normalized=scalar(self.layout.state+3*i)?;let value=rescale(normalized,self.state_scale,1.);
   if !value.is_finite()||(normalized!=0.&&value==0.) {return Err(JointDotError::Output);}state.push(value);
  }
  let mut reactions=Vec::new();reactions.try_reserve_exact(self.rows).map_err(|_|JointDotError::Capacity)?;
  for i in 0..self.rows {
   let normalized=scalar(self.layout.reactions+3*i)?;let value=rescale(normalized,self.state_scale,self.inverse_column_scale);
   if !value.is_finite()||value<0.||(normalized!=0.&&value==0.) {return Err(JointDotError::Output);}reactions.push(value);
  }
  Ok((state,reactions))
 }
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn triple_batch_checks_metadata_and_withholds_valid_prefix_on_late_failure() {
  let inputs=(1..=2).map(|n|ExperimentalTripleActiveContactInput::new(&[vec![1.]],&[n as f64],1e-14).unwrap()).collect();
  let batch=ExperimentalTripleActiveContactBatch::new(inputs).unwrap();let mut output=batch.words.clone();
  for (i,input) in batch.inputs.iter().enumerate() {
   let base=batch.offsets[i];output[base+3]=1;output[base+input.layout.status]=0;
   let value=(i+1) as f64;
   output[base+input.layout.state]=((value/input.state_scale) as f32).to_bits();
   output[base+input.layout.reactions]=((value/(input.state_scale*input.inverse_column_scale)) as f32).to_bits();
  }
  assert_eq!(batch.candidate(bytemuck::cast_slice(&output)).unwrap(),vec![(vec![1.],vec![1.]),(vec![2.],vec![2.])]);
  let full=bytemuck::cast_slice::<u32,u8>(&output);
  let compact=batch.compact_readback_ranges().iter().flat_map(|range|full[range.start as usize..range.end as usize].iter().copied()).collect::<Vec<_>>();
  for (bytes,is_compact) in [(full,false),(compact.as_slice(),true)] {
   let slices=batch.operator_readbacks(bytes,is_compact).unwrap();
   let decoded=batch.inputs.iter().zip(slices).map(|(input,bytes)|if is_compact {input.candidate_from_compact_readback(bytes)} else {input.candidate(bytes)}).collect::<Result<Vec<_>,_>>().unwrap();
   assert_eq!(decoded,batch.candidate(full).unwrap());
   assert!(batch.operator_readbacks(&bytes[..bytes.len()-4],is_compact).is_err());
   let mut extra=bytes.to_vec();extra.extend([0;4]);assert!(batch.operator_readbacks(&extra,is_compact).is_err());
   for index in 0..batch.inputs.len()+2 {let mut bad=bytes.to_vec();bad[index*4]^=1;assert!(batch.operator_readbacks(&bad,is_compact).is_err());}
  }
  for index in 0..batch.inputs.len()+2 {let mut bad=output.clone();bad[index]^=1;assert!(batch.candidate(bytemuck::cast_slice(&bad)).is_err());}
  output[batch.offsets[1]+batch.inputs[1].layout.status]=1;assert!(batch.candidate(bytemuck::cast_slice(&output)).is_err());
  assert!(ExperimentalTripleActiveContactBatch::new(vec![]).is_err());
 }
 #[test]
 fn triple_active_layout_checks_total_workspace_before_allocation() {
  assert!(Layout::new(91,882).is_ok());assert!(Layout::new(1024,1024).is_ok());
  assert!(matches!(Layout::new(1025,1),Err(JointDotError::Capacity)));
  assert!(matches!(Layout::new(1024,10000),Err(JointDotError::Capacity)));
  assert!(matches!(Layout::new(2,usize::MAX),Err(JointDotError::Capacity)));
 }
 #[test]
 fn triple_active_seed_hints_preserve_original_operator_and_validate_shape() {
  let columns=vec![vec![1.,0.],vec![0.,1.]];let bounds=[1.,2.];
  let cold=ExperimentalTripleActiveContactInput::new(&columns,&bounds,1e-14).unwrap();
  let warm=ExperimentalTripleActiveContactInput::new_seeded(&columns,&bounds,1e-14,&[0.,f64::MAX]).unwrap();
  assert_eq!(&cold.words[..cold.layout.immutable],&warm.words[..warm.layout.immutable]);
  assert_eq!(cold.bytes().len(),warm.bytes().len());
  let offset=warm.layout.state-3*warm.rows;assert_eq!(warm.words[offset],0);assert_eq!(warm.words[offset+3],1f32.to_bits());
  for seeds in [&[1.][..],&[1.,f64::NAN],&[-1.,1.]] {assert!(ExperimentalTripleActiveContactInput::new_seeded(&columns,&bounds,1e-14,seeds).is_err());}
  assert_eq!(ExperimentalTripleActiveContactInput::new_seeded(&columns,&bounds,1e-14,&[]).unwrap().bytes(),cold.bytes());
 }
 #[test]
 fn triple_active_factor_reuse_requires_exact_columns_and_flags() {
  let columns=vec![vec![1.,0.],vec![0.,1.]];
  let input=ExperimentalTripleActiveContactInput::new_seeded(&columns,&[1.,2.],1e-14,&[1.,1.]).unwrap();
  let mut output=input.words.clone();output[3]=1;output[input.layout.status]=0;
  let factors=input.parallel_factors(bytemuck::cast_slice(&output)).unwrap();
  let mut changed=ExperimentalTripleActiveContactInput::new_seeded(&columns,&[2.,3.],1e-13,&[1.,1.]).unwrap();
  changed.reuse_parallel_factors(&factors).unwrap();assert_eq!(changed.words[3],2);
  for bad_columns in [vec![vec![f64::from_bits(1f64.to_bits()+1),0.],vec![0.,1.]],vec![vec![1.,-0.],vec![0.,1.]],vec![vec![2.,0.],vec![0.,2.]]] {
   let mut bad=ExperimentalTripleActiveContactInput::new_seeded(&bad_columns,&[1.,2.],1e-14,&[1.,1.]).unwrap();
   assert!(bad.reuse_parallel_factors(&factors).is_err());
  }
  let mut bad=ExperimentalTripleActiveContactInput::new_seeded(&columns,&[1.,2.],1e-14,&[1.,0.]).unwrap();assert!(bad.reuse_parallel_factors(&factors).is_err());
 }
 #[test]
 fn triple_active_factor_snapshot_retains_only_selected_qr_and_restores_stride() {
  let columns=vec![vec![1.,0.],vec![0.,1.],vec![1.,1.]];
  let input=ExperimentalTripleActiveContactInput::new_seeded(&columns,&[1.,2.,3.],1e-14,&[1.,0.,1.]).unwrap();
  let q=input.layout.immutable;let r=q+3*input.rows*input.width;
  let mut output=input.words.clone();output[3]=1;output[input.layout.status]=0;
  for (i,word) in output[q..r+3*input.rows*input.rows].iter_mut().enumerate() {*word=((i+1) as f32).to_bits();}
  let factors=input.parallel_factors(bytemuck::cast_slice(&output)).unwrap();
  assert_eq!(factors.qr.len(),3*2*2+3*2*2);
  assert!(factors.retained_bytes()<(input.rows*input.width*6+input.rows*input.rows*3+input.rows*3)*4);
  let mut restored=ExperimentalTripleActiveContactInput::new_seeded(&columns,&[2.,3.,4.],1e-14,&[1.,0.,1.]).unwrap();
  restored.reuse_parallel_factors(&factors).unwrap();
  assert_eq!(&restored.words[q..q+12],&output[q..q+12]);assert!(restored.words[q+12..r].iter().all(|v|*v==0));
  for column in 0..2 {assert_eq!(&restored.words[r+9*column..r+9*column+6],&output[r+9*column..r+9*column+6]);assert!(restored.words[r+9*column+6..r+9*column+9].iter().all(|v|*v==0));}
  let same=restored.parallel_factors(bytemuck::cast_slice(&output)).err();assert!(same.is_some(),"mutated immutable bounds must reject output");
  let duplicate=input.parallel_factors(bytemuck::cast_slice(&output)).unwrap();assert!(factors.same_basis(&duplicate));
 }
 #[test]
 fn triple_active_basis_key_ignores_only_inactive_rows_and_rejects_order_changes() {
  let columns=vec![vec![1.,0.],vec![0.,1.],vec![0.25,0.25]];
  let input=ExperimentalTripleActiveContactInput::new_seeded(&columns,&[1.,2.,0.],1e-14,&[1.,1.,0.]).unwrap();
  let mut output=input.words.clone();output[3]=1;output[input.layout.status]=0;
  let factors=input.parallel_factors(bytemuck::cast_slice(&output)).unwrap();
  assert_eq!(factors.columns.len(),2*2*3);
  let mut extended=columns.clone();extended[2]=vec![0.5,-0.5];extended.push(vec![0.,0.]);
  let mut changed=ExperimentalTripleActiveContactInput::new_seeded(&extended,&[1.,2.,0.,-1.],1e-14,&[1.,1.,0.,0.]).unwrap();
  changed.reuse_parallel_factors(&factors).unwrap();
  extended.swap(0,1);
  let mut reordered=ExperimentalTripleActiveContactInput::new_seeded(&extended,&[2.,1.,0.,-1.],1e-14,&[1.,1.,0.,0.]).unwrap();
  assert!(reordered.reuse_parallel_factors(&factors).is_err());
 }
 #[test]
 fn triple_compact_readback_matches_full_and_rejects_missing_or_mutated_original_words() {
  let input=ExperimentalTripleActiveContactInput::new_seeded(&[vec![1.,0.],vec![0.,1.],vec![0.,0.]],&[1.,2.,-1.],1e-14,&[1.,1.,0.]).unwrap();
  let mut output=input.words.clone();output[3]=1;output[input.layout.status]=0;
  output[input.layout.state]=1f32.to_bits();output[input.layout.state+3]=2f32.to_bits();
  let full=bytemuck::cast_slice::<u32,u8>(&output);
  let compact=input.compact_readback_ranges().iter().flat_map(|range|full[range.start as usize..range.end as usize].iter().copied()).collect::<Vec<_>>();
  assert!(compact.len()<full.len());assert_eq!(input.candidate_from_compact_readback(&compact).unwrap(),input.candidate(full).unwrap());
  let a=input.parallel_factors(full).unwrap();let b=input.parallel_factors_from_compact_readback(&compact).unwrap();assert!(a.same_basis(&b));assert_eq!(a.qr,b.qr);
  for index in [0,4*4,4*(input.layout.immutable-1),compact.len()-4] {
   let mut bad=compact.clone();bad[index]^=1;assert!(input.candidate_from_compact_readback(&bad).is_err());
  }
  assert!(input.candidate_from_compact_readback(&compact[..compact.len()-4]).is_err());
  let mut extra=compact.clone();extra.extend([0;4]);assert!(input.candidate_from_compact_readback(&extra).is_err());
 }
 #[test]
 fn triple_active_decoder_rejects_partial_mutated_and_nonfinite_results() {
  let input=ExperimentalTripleActiveContactInput::new(&[vec![1.]],&[1.],1e-14).unwrap();
  assert!(input.candidate(input.bytes()).is_err());
  let mut valid=input.words.clone();valid[3]=2;valid[input.layout.status]=0;
  valid[input.layout.state]=1f32.to_bits();valid[input.layout.reactions]=1f32.to_bits();
  assert_eq!(input.candidate(bytemuck::cast_slice(&valid)).unwrap(),(vec![1.],vec![1.]));
  for index in [0,1,2,4,input.layout.immutable-1] {let mut bad=valid.clone();bad[index]^=1;assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());}
  for status in [1,2,8,13,14,u32::MAX] {let mut bad=valid.clone();bad[input.layout.status]=status;assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());}
  for iterations in [0,input.words[2]+1] {let mut bad=valid.clone();bad[3]=iterations;assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());}
  for index in [input.layout.state,input.layout.state+1,input.layout.state+2,input.layout.reactions] {let mut bad=valid.clone();bad[index]=f32::NAN.to_bits();assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());}
  let mut bad=valid.clone();bad[input.layout.reactions]=(-1f32).to_bits();assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());
  assert!(input.candidate(&input.bytes()[..input.bytes().len()-4]).is_err());
 }
}
