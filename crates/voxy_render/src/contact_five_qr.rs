//! Experimental rectangular equality QR; physical inequality admission is external.
use crate::{ExperimentalExactFiveContactInput, joint_contact_dot::JointDotError};
const MAGIC:u32=0x31513556; // V5Q1, distinct from input and arithmetic probes.
const HEADER:usize=32;
const VALUE:usize=8;
#[derive(Clone,Debug)]
pub struct ExperimentalFiveQrValue {pub words:[u32;5],pub exponent:i32,pub error_bound:f32}
#[derive(Debug)]
pub struct ExperimentalFiveQrReadback {
    pub coordinates:Vec<ExperimentalFiveQrValue>,pub reactions:Vec<ExperimentalFiveQrValue>,
    pub basis:Vec<ExperimentalFiveQrValue>,pub triangular:Vec<ExperimentalFiveQrValue>,
}
#[derive(Debug)]
pub struct ExperimentalFiveQrInput {
    words:Vec<u32>,rows:usize,width:usize,input_end:usize,
    q:usize,r:usize,state:usize,reaction:usize,state_scale:f64,reaction_scale:f64,
}
impl ExperimentalFiveQrInput {
    pub fn new(columns:&[Vec<[f64;2]>],bounds:&[f64],tolerance:f64)->Result<Self,JointDotError> {
        let rows=columns.len();let width=columns.first().map_or(0,Vec::len);
        if rows==0 || rows>width || rows>1024 {return Err(JointDotError::Shape);}
        let packed=ExperimentalExactFiveContactInput::new(columns,bounds,tolerance)?;
        let count=rows.checked_mul(width).ok_or(JointDotError::Capacity)?;
        let q=HEADER.checked_add((count+rows+1).checked_mul(5).ok_or(JointDotError::Capacity)?).ok_or(JointDotError::Capacity)?;
        let r=q.checked_add(count.checked_mul(VALUE).ok_or(JointDotError::Capacity)?).ok_or(JointDotError::Capacity)?;
        let state=r.checked_add(rows.checked_mul(rows).and_then(|n|n.checked_mul(VALUE)).ok_or(JointDotError::Capacity)?).ok_or(JointDotError::Capacity)?;
        let reaction=state.checked_add(width.checked_mul(VALUE).ok_or(JointDotError::Capacity)?).ok_or(JointDotError::Capacity)?;
        let total=reaction.checked_add(rows.checked_mul(VALUE).ok_or(JointDotError::Capacity)?).ok_or(JointDotError::Capacity)?;
        if total>16*1024*1024/4 {return Err(JointDotError::Capacity);}
        let state_scale=packed.bound_scale()/packed.column_scale();
        let reaction_scale=state_scale/packed.column_scale();
        if !state_scale.is_finite() || state_scale<=0. || !reaction_scale.is_finite() || reaction_scale<=0. {return Err(JointDotError::NumericRange);}
        let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|JointDotError::Capacity)?;
        words.extend([MAGIC,1,rows as u32,width as u32,5,VALUE as u32,u32::MAX,0,HEADER as u32,(HEADER+count*5) as u32,(HEADER+(count+rows)*5) as u32,q as u32,r as u32,state as u32,reaction as u32,total as u32]);
        for value in [packed.column_scale(),packed.bound_scale()] {let bits=value.to_bits();words.extend([bits as u32,(bits>>32) as u32]);}
        words.resize(HEADER,0);
        words.extend(packed.packed_columns().iter().flatten().copied());
        words.extend(packed.packed_bounds().iter().flatten().copied());words.extend(packed.packed_tolerance());
        assert_eq!(words.len(),q);words.resize(total,0);
        Ok(Self{words,rows,width,input_end:q,q,r,state,reaction,state_scale,reaction_scale})
    }
    pub fn bytes(&self)->&[u8]{bytemuck::cast_slice(&self.words)}
    pub fn steps(&self)->usize{self.rows+1}
    pub fn state_scale(&self)->f64{self.state_scale}
    pub fn reaction_scale(&self)->f64{self.reaction_scale}
    pub fn shader_source()->String{format!("{}\n{}",include_str!("contact_five_product.wgsl"),include_str!("contact_five_qr.wgsl"))}
    pub fn candidate(&self,bytes:&[u8])->Result<ExperimentalFiveQrReadback,JointDotError> {
        if bytes.len()!=self.bytes().len(){return Err(JointDotError::Output);}
        let words=bytes.chunks_exact(4).map(|v|u32::from_le_bytes(v.try_into().unwrap())).collect::<Vec<_>>();
        if words[6]!=0 || words[7] as usize!=self.steps() || (0..self.input_end).any(|i|i!=6&&i!=7&&words[i]!=self.words[i]) {return Err(JointDotError::Output);}
        let read=|start:usize,count:usize|->Result<Vec<ExperimentalFiveQrValue>,JointDotError>{
            words[start..start+count*VALUE].chunks_exact(VALUE).map(|v|{
                let parts:[u32;5]=v[..5].try_into().unwrap();let error=f32::from_bits(v[6]);
                if v[7]!=1 || parts.iter().any(|&w|!f32::from_bits(w).is_finite()) || !error.is_finite() || error<0. {return Err(JointDotError::Output);}
                Ok(ExperimentalFiveQrValue{words:parts,exponent:v[5] as i32,error_bound:error})
            }).collect()
        };
        Ok(ExperimentalFiveQrReadback{coordinates:read(self.state,self.width)?,reactions:read(self.reaction,self.rows)?,basis:read(self.q,self.rows*self.width)?,triangular:read(self.r,self.rows*self.rows)?})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn five_qr_shader_validates(){let module=naga::front::wgsl::parse_str(&ExperimentalFiveQrInput::shader_source()).unwrap();naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::empty()).validate(&module).unwrap();}
    #[test]
    fn five_qr_input_rejects_stale_and_mutated_readbacks(){
        let input=ExperimentalFiveQrInput::new(&[vec![[1.,0.]]],&[1.],1e-14).unwrap();
        assert!(input.candidate(input.bytes()).is_err());
        let mut words=input.words.clone();words[6]=0;words[7]=2;
        for at in [input.q,input.r,input.state,input.reaction]{words[at]=1f32.to_bits();words[at+7]=1;}
        assert!(input.candidate(bytemuck::cast_slice(&words)).is_ok());
        for i in [0,1,4,5,8,HEADER,input.state+7]{let mut bad=words.clone();bad[i]^=1;assert!(input.candidate(bytemuck::cast_slice(&bad)).is_err());}
        assert!(ExperimentalFiveQrInput::new(&[vec![[1.,0.]],vec![[2.,0.]]],&[1.,2.],1e-14).is_err());
    }
}
