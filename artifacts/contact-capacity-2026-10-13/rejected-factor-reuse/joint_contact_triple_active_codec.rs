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
 rows:usize,width:usize,columns:Vec<u32>,flags:Vec<u32>,qr:Vec<u32>,selected:usize,column_scale_bits:u64,
}
impl ExperimentalTripleActiveContactInput {
 pub fn shader_source()->String {format!("{}\n{}",EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,include_str!("joint_contact_triple_active.wgsl"))}
 /// Qualification-only 256-lane alternative using the identical checked payload.
 pub fn parallel_shader_source()->String {format!("{}\n{}",EXPERIMENTAL_TRIPLE_CONTACT_ARITHMETIC,include_str!("joint_contact_triple_parallel.wgsl"))}
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
  self.candidate(bytes)?;
  let output=bytes.chunks_exact(4).map(|b|u32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();
  let flags=self.words[self.layout.state-3*self.rows..self.layout.state].to_vec();
  let selected=flags.chunks_exact(3).filter(|v|v[0]==1f32.to_bits()).count();
  if output[3]!=1||selected==0 {return Err(JointDotError::Output);}
  let end=self.layout.immutable+3*self.rows*self.width+3*self.rows*self.rows;
  let qr=output[self.layout.immutable..end].to_vec();
  if qr.iter().any(|v|!f32::from_bits(*v).is_finite()) {return Err(JointDotError::Output);}
  Ok(ExperimentalTripleActiveFactors {rows:self.rows,width:self.width,columns:self.words[4..4+3*self.rows*self.width].to_vec(),flags,qr,selected,column_scale_bits:self.inverse_column_scale.to_bits()})
 }
 /// Apply exact matching basis scratch. Bounds may change; original admission remains mandatory.
 /// Use only with parallel_shader_source; a rejected candidate must be retried cold.
 pub fn reuse_parallel_factors(&mut self,factors:&ExperimentalTripleActiveFactors)->Result<(),JointDotError> {
  if self.rows!=factors.rows||self.width!=factors.width||self.inverse_column_scale.to_bits()!=factors.column_scale_bits
   ||self.words[4..4+3*self.rows*self.width]!=factors.columns
   ||self.words[self.layout.state-3*self.rows..self.layout.state]!=factors.flags {return Err(JointDotError::Shape);}
  let end=self.layout.immutable+factors.qr.len();
  self.words[self.layout.immutable..end].copy_from_slice(&factors.qr);
  self.words[3]=factors.selected as u32;
  Ok(())
 }
 pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
 /// Transport validation only. The original native owner must admit this candidate.
 pub fn candidate(&self,bytes:&[u8])->Result<(Vec<f64>,Vec<f64>),JointDotError> {
  if bytes.len()!=self.words.len()*4 {return Err(JointDotError::Output);}
  let word=|i:usize|u32::from_le_bytes(bytes[i*4..i*4+4].try_into().unwrap());
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
