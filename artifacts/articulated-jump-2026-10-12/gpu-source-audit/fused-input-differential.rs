#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JointDotError { Shape, Capacity, NumericRange, Output }
impl std::fmt::Display for JointDotError {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {write!(f,"joint contact dot: {self:?}")}
}
impl std::error::Error for JointDotError {}
#[derive(Debug)]
pub struct JointContactDotInput {
    words: Vec<u32>, scales: Vec<f64>, vector_scale: f64,
    output: usize, rows: usize, packing_error: f64,
}
fn pack(value:f64)->Result<[u32;2],JointDotError> {
    let hi=value as f32;let lo=(value-hi as f64) as f32;
    if !hi.is_finite() || !lo.is_finite() || (value!=0. && hi==0. && lo==0.) {
        return Err(JointDotError::NumericRange);
    }
    Ok([hi.to_bits(),lo.to_bits()])
}
// Keep the ordinary operation order, but recover a representable result when
// an intermediate product overflows or underflows. Scales are positive finite.
fn rescale(value:f64,a:f64,b:f64)->f64 {
    let ordinary=value*a*b;
    if value==0. || (ordinary.is_finite() && ordinary!=0.) {return ordinary;}
    fn parts(mut x:f64)->(f64,i32) {
        let mut adjustment=0;
        if x<f64::MIN_POSITIVE {x*=2f64.powi(52);adjustment=-52;}
        let bits=x.to_bits();
        let exponent=((bits>>52)&0x7ff) as i32-1023+adjustment;
        (f64::from_bits((bits&((1u64<<52)-1))|(1023u64<<52)),exponent)
    }
    let (v,ve)=parts(value.abs());let (am,ae)=parts(a);let (bm,be)=parts(b);
    let mut mantissa=v*am*bm;let mut exponent=ve+ae+be;
    while mantissa>=2. {mantissa*=0.5;exponent+=1;}
    let result=if exponent>1023 {f64::INFINITY}
        else if exponent>=-1022 {mantissa*f64::from_bits(((exponent+1023) as u64)<<52)}
        else if exponent>=-1075 {(mantissa*f64::MIN_POSITIVE)*2f64.powi(exponent+1022)}
        else {0.};
    result.copysign(value)
}
mod old {
use super::{JointDotError,pack,rescale};

/// Prototype input for one complete unilateral coordinate operator. Unsupported
/// shapes/ranges reject the complete request; no rows or coefficients are dropped.
#[derive(Debug)]
pub struct ExperimentalResidentContactInput {
    words:Vec<u32>, total_words:usize, immutable_end:usize, reactions:usize, state:usize, status:usize,
    rows:usize, width:usize, column_scale:f64, state_scale:f64,
}
/// A GPU candidate only. The original f64 physical owner must validate it.
pub const EXPERIMENTAL_RESIDENT_CONTACT_SHADER:&str="/Users/themoretheless/Documents/ChatGPT/Voxy/crates/voxy_render/src/joint_contact_active.wgsl";
impl ExperimentalResidentContactInput {
    /// Cold active-set construction, retaining all original coordinates.
    pub fn new(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64)->Result<Self,JointDotError> {
        let rows=columns.len();let width=columns.first().map_or(0,Vec::len);
        if rows==0||width==0||bounds.len()!=rows||!tolerance.is_finite()||tolerance<=0.
            ||columns.iter().any(|c|c.len()!=width||c.iter().any(|v|!v.is_finite()))
            ||bounds.iter().any(|v|!v.is_finite()) {return Err(JointDotError::Shape);}
        if rows>512 {return Err(JointDotError::Capacity);}
        let matrix=rows.checked_mul(width).and_then(|n|n.checked_mul(2)).ok_or(JointDotError::Capacity)?;
        let square=rows.checked_mul(rows).and_then(|n|n.checked_mul(2)).ok_or(JointDotError::Capacity)?;
        let immutable_end=4usize.checked_add(matrix).and_then(|n|n.checked_add(rows*2+2)).ok_or(JointDotError::Capacity)?;
        let z=immutable_end.checked_add(matrix).and_then(|n|n.checked_add(square)).ok_or(JointDotError::Capacity)?;
        let reactions=z.checked_add(rows*4).and_then(|n|n.checked_add(width.checked_mul(2)?)).ok_or(JointDotError::Capacity)?;
        let state=reactions.checked_add(rows*2).ok_or(JointDotError::Capacity)?;
        let status=state.checked_add(width.checked_mul(2).ok_or(JointDotError::Capacity)?).and_then(|n|n.checked_add(rows*2)).ok_or(JointDotError::Capacity)?;
        let total=status.checked_add(1).filter(|&n|n<=16*1024*1024&&n<=u32::MAX as usize).ok_or(JointDotError::Capacity)?;
        let column_scale=columns.iter().flatten().map(|v|v.abs()).fold(0.,f64::max);
        let bound_scale=bounds.iter().map(|v|v.abs()).fold(tolerance,f64::max);
        let state_scale=bound_scale/column_scale;
        if !column_scale.is_finite()||column_scale<=0.||!column_scale.recip().is_finite()||!state_scale.is_finite()||state_scale<=0. {return Err(JointDotError::NumericRange);}
        let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|JointDotError::Capacity)?;
        words.extend([width as u32,rows as u32,512u32.max(rows as u32+1),0]);
        for v in columns.iter().flatten() {
            let normalized=v/column_scale;
            if *v!=0.&&normalized==0. {return Err(JointDotError::NumericRange);}
            words.extend(pack(normalized)?);
        }
        for &v in bounds {
            let normalized=v/bound_scale;
            if v!=0.&&normalized==0. {return Err(JointDotError::NumericRange);}
            words.extend(pack(normalized)?);
        }
        words.extend(pack(tolerance/bound_scale)?);
        assert_eq!(words.len(),immutable_end);
        words.resize(total,0);words[status]=u32::MAX;
        Ok(Self {words,total_words:total,immutable_end,reactions,state,status,rows,width,column_scale,state_scale})
    }
    /// Experimental hint upload for a shader that explicitly supports header[3].
    /// Hints never replace original coefficients or physical admission. Invalid
    /// or unrepresentable hints retain the complete cold-start input.
    pub fn new_seeded(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,seeds:&[f64])->Result<Self,JointDotError> {
        let mut input=Self::new(columns,bounds,tolerance)?;
        if seeds.len()!=input.rows || seeds.iter().any(|v|!v.is_finite()||*v<0.)
            || !seeds.iter().any(|&v|v>0.) {return Ok(input);}
        let packed=seeds.iter().map(|&value| {
            let normalized=rescale(value,input.column_scale,1./input.state_scale);
            if value>0. && normalized==0. {return Err(JointDotError::NumericRange);}
            pack(normalized)
        }).collect::<Result<Vec<_>,_>>();
        let Ok(packed)=packed else {return Ok(input);};
        for (i,value) in packed.into_iter().enumerate() {
            input.words[input.reactions+2*i..input.reactions+2*i+2].copy_from_slice(&value);
        }
        input.words[3]=1;
        Ok(input)
    }
    /// Whether this input contains experimental initial reaction hints.
    pub fn has_seed_hint(&self)->bool {self.words[3]==1}
    /// Complete immutable upload plus private resident scratch storage.
    pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
    /// Decode only completed, finite output. This is not original KKT/physics admission.
    pub fn candidate(&self,bytes:&[u8])->Result<(Vec<f64>,Vec<f64>),JointDotError> {
        if bytes.len()!=self.total_words*4 {return Err(JointDotError::Output);}
        // Read the admitted ABI in place. Copying every private QR scratch word
        // into another Vec made CPU memory proportional to GPU workspace size.
        // The full immutable operator audit remains bit-exact and endian-explicit.
        let word=|offset:usize|u32::from_le_bytes(bytes[offset*4..offset*4+4].try_into().unwrap());
        self.decode_candidate(word)
    }
    fn decode_candidate(&self,word:impl Fn(usize)->u32)->Result<(Vec<f64>,Vec<f64>),JointDotError> {
        if (0..self.immutable_end).any(|i|word(i)!=self.words[i])||word(self.status)!=0 {return Err(JointDotError::Output);}
        let scalar=|offset| {let hi=f32::from_bits(word(offset)) as f64;let lo=f32::from_bits(word(offset+1)) as f64;
            if hi.is_finite()&&lo.is_finite() {Ok(hi+lo)} else {Err(JointDotError::Output)}};
        let state=(0..self.width).map(|i| {let v=rescale(scalar(self.state+2*i)?,self.state_scale,1.);
            v.is_finite().then_some(v).ok_or(JointDotError::Output)}).collect::<Result<Vec<_>,_>>()?;
        let reactions=(0..self.rows).map(|i| {let v=rescale(scalar(self.reactions+2*i)?,self.state_scale,1./self.column_scale);
            (v.is_finite()&&v>=0.).then_some(v).ok_or(JointDotError::Output)}).collect::<Result<Vec<_>,_>>()?;
        Ok((state,reactions))
    }
}

/// Independent complete operators dispatched concurrently in disjoint workgroups.
/// The header and every immutable original operator are audited on readback.
#[derive(Debug)]
pub struct ExperimentalResidentContactBatch {
    inputs:Vec<ExperimentalResidentContactInput>,words:Vec<u32>,offsets:Vec<usize>,header_end:usize,
}
impl ExperimentalResidentContactBatch {
    /// All-or-nothing packing; never partitions a physical operator or drops rows.
    pub fn new(mut inputs:Vec<ExperimentalResidentContactInput>)->Result<Self,JointDotError> {
        if inputs.is_empty()||inputs.len()>65535 {return Err(JointDotError::Shape);}
        let header_end=4usize.checked_add(inputs.len()).ok_or(JointDotError::Capacity)?;
        let total=inputs.iter().try_fold(header_end,|n,i|n.checked_add(i.words.len()))
            .filter(|&n|n<=16*1024*1024).ok_or(JointDotError::Capacity)?;
        let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|JointDotError::Capacity)?;
        words.extend([u32::MAX,inputs.len() as u32,header_end as u32,0]);
        let mut offsets=Vec::with_capacity(inputs.len());let mut offset=header_end;
        for input in &inputs {words.push(offset as u32);offsets.push(offset);offset+=input.words.len();}
        for input in &mut inputs {
            words.extend_from_slice(&input.words);
            // The batch owns the complete upload. Keep only the immutable
            // certificate per input, not a second copy of private QR scratch.
            input.words=input.words[..input.immutable_end].to_vec();
        }
        Ok(Self {inputs,words,offsets,header_end})
    }
    pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
    pub fn workgroups(&self)->[u32;3] {[self.inputs.len() as u32,1,1]}
    /// Ordered byte ranges for an audited readback without private QR scratch.
    /// Concatenate these ranges exactly; every original immutable word remains.
    pub fn candidate_readback_ranges(&self)->Vec<std::ops::Range<u64>> {
        let mut ranges=Vec::with_capacity(1+2*self.inputs.len());
        ranges.push(0..(self.header_end*4) as u64);
        for (input,&offset) in self.inputs.iter().zip(&self.offsets) {
            ranges.push((offset*4) as u64..((offset+input.immutable_end)*4) as u64);
            ranges.push(((offset+input.reactions)*4) as u64..((offset+input.total_words)*4) as u64);
        }
        ranges
    }
    /// Same publication gates as full readback; original physical admission follows.
    pub fn candidate_from_compact_readback(&self,bytes:&[u8])->Result<Vec<(Vec<f64>,Vec<f64>)>,JointDotError> {
        let expected=self.header_end*4+self.inputs.iter().map(|i|
            (i.immutable_end+i.total_words-i.reactions)*4).sum::<usize>();
        if bytes.len()!=expected || bytes[..self.header_end*4]!=self.bytes()[..self.header_end*4] {
            return Err(JointDotError::Output);
        }
        let mut cursor=self.header_end*4;
        self.inputs.iter().map(|input| {
            let length=(input.immutable_end+input.total_words-input.reactions)*4;
            let current=&bytes[cursor..cursor+length];cursor+=length;
            input.decode_candidate(|index| {
                let compact=if index<input.immutable_end {index}
                    else {input.immutable_end+index-input.reactions};
                u32::from_le_bytes(current[compact*4..compact*4+4].try_into().unwrap())
            })
        }).collect()
    }

    /// Publish a complete batch only after all operators pass ABI/output checks.
    /// Original f64 physical admission is still required by each owner.
    pub fn candidate(&self,bytes:&[u8])->Result<Vec<(Vec<f64>,Vec<f64>)>,JointDotError> {
        if bytes.len()!=self.words.len()*4 {return Err(JointDotError::Output);}
        if bytes[..self.header_end*4]!=self.bytes()[..self.header_end*4] {return Err(JointDotError::Output);}
        self.inputs.iter().zip(&self.offsets).map(|(input,&offset)|
            input.candidate(&bytes[offset*4..(offset+input.total_words)*4])).collect()
    }
}

}
mod new {
use super::{JointDotError,pack,rescale};

/// Prototype input for one complete unilateral coordinate operator. Unsupported
/// shapes/ranges reject the complete request; no rows or coefficients are dropped.
#[derive(Debug)]
pub struct ExperimentalResidentContactInput {
    words:Vec<u32>, total_words:usize, immutable_end:usize, reactions:usize, state:usize, status:usize,
    rows:usize, width:usize, column_scale:f64, state_scale:f64,
}
/// A GPU candidate only. The original f64 physical owner must validate it.
pub const EXPERIMENTAL_RESIDENT_CONTACT_SHADER:&str="/Users/themoretheless/Documents/ChatGPT/Voxy/crates/voxy_render/src/joint_contact_active.wgsl";
impl ExperimentalResidentContactInput {
    /// Cold active-set construction, retaining all original coordinates.
    pub fn new(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64)->Result<Self,JointDotError> {
        let rows=columns.len();let width=columns.first().map_or(0,Vec::len);
        if rows==0||width==0||bounds.len()!=rows||!tolerance.is_finite()||tolerance<=0. {
            return Err(JointDotError::Shape);
        }
        // Validate and measure in the same ordered pass. Preserve the original
        // error precedence: malformed coefficients reject before capacity.
        let mut column_scale=0f64;
        for column in columns {
            if column.len()!=width {return Err(JointDotError::Shape);}
            for &value in column {
                if !value.is_finite() {return Err(JointDotError::Shape);}
                column_scale=column_scale.max(value.abs());
            }
        }
        let mut bound_scale=tolerance;
        for &value in bounds {
            if !value.is_finite() {return Err(JointDotError::Shape);}
            bound_scale=bound_scale.max(value.abs());
        }
        if rows>512 {return Err(JointDotError::Capacity);}
        let matrix=rows.checked_mul(width).and_then(|n|n.checked_mul(2)).ok_or(JointDotError::Capacity)?;
        let square=rows.checked_mul(rows).and_then(|n|n.checked_mul(2)).ok_or(JointDotError::Capacity)?;
        let immutable_end=4usize.checked_add(matrix).and_then(|n|n.checked_add(rows*2+2)).ok_or(JointDotError::Capacity)?;
        let z=immutable_end.checked_add(matrix).and_then(|n|n.checked_add(square)).ok_or(JointDotError::Capacity)?;
        let reactions=z.checked_add(rows*4).and_then(|n|n.checked_add(width.checked_mul(2)?)).ok_or(JointDotError::Capacity)?;
        let state=reactions.checked_add(rows*2).ok_or(JointDotError::Capacity)?;
        let status=state.checked_add(width.checked_mul(2).ok_or(JointDotError::Capacity)?).and_then(|n|n.checked_add(rows*2)).ok_or(JointDotError::Capacity)?;
        let total=status.checked_add(1).filter(|&n|n<=16*1024*1024&&n<=u32::MAX as usize).ok_or(JointDotError::Capacity)?;
        let state_scale=bound_scale/column_scale;
        if !column_scale.is_finite()||column_scale<=0.||!column_scale.recip().is_finite()||!state_scale.is_finite()||state_scale<=0. {return Err(JointDotError::NumericRange);}
        let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|JointDotError::Capacity)?;
        words.extend([width as u32,rows as u32,512u32.max(rows as u32+1),0]);
        for v in columns.iter().flatten() {
            let normalized=v/column_scale;
            if *v!=0.&&normalized==0. {return Err(JointDotError::NumericRange);}
            words.extend(pack(normalized)?);
        }
        for &v in bounds {
            let normalized=v/bound_scale;
            if v!=0.&&normalized==0. {return Err(JointDotError::NumericRange);}
            words.extend(pack(normalized)?);
        }
        words.extend(pack(tolerance/bound_scale)?);
        assert_eq!(words.len(),immutable_end);
        words.resize(total,0);words[status]=u32::MAX;
        Ok(Self {words,total_words:total,immutable_end,reactions,state,status,rows,width,column_scale,state_scale})
    }
    /// Experimental hint upload for a shader that explicitly supports header[3].
    /// Hints never replace original coefficients or physical admission. Invalid
    /// or unrepresentable hints retain the complete cold-start input.
    pub fn new_seeded(columns:&[Vec<f64>],bounds:&[f64],tolerance:f64,seeds:&[f64])->Result<Self,JointDotError> {
        let mut input=Self::new(columns,bounds,tolerance)?;
        if seeds.len()!=input.rows || seeds.iter().any(|v|!v.is_finite()||*v<0.)
            || !seeds.iter().any(|&v|v>0.) {return Ok(input);}
        let packed=seeds.iter().map(|&value| {
            let normalized=rescale(value,input.column_scale,1./input.state_scale);
            if value>0. && normalized==0. {return Err(JointDotError::NumericRange);}
            pack(normalized)
        }).collect::<Result<Vec<_>,_>>();
        let Ok(packed)=packed else {return Ok(input);};
        for (i,value) in packed.into_iter().enumerate() {
            input.words[input.reactions+2*i..input.reactions+2*i+2].copy_from_slice(&value);
        }
        input.words[3]=1;
        Ok(input)
    }
    /// Whether this input contains experimental initial reaction hints.
    pub fn has_seed_hint(&self)->bool {self.words[3]==1}
    /// Complete immutable upload plus private resident scratch storage.
    pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
    /// Decode only completed, finite output. This is not original KKT/physics admission.
    pub fn candidate(&self,bytes:&[u8])->Result<(Vec<f64>,Vec<f64>),JointDotError> {
        if bytes.len()!=self.total_words*4 {return Err(JointDotError::Output);}
        // Read the admitted ABI in place. Copying every private QR scratch word
        // into another Vec made CPU memory proportional to GPU workspace size.
        // The full immutable operator audit remains bit-exact and endian-explicit.
        let word=|offset:usize|u32::from_le_bytes(bytes[offset*4..offset*4+4].try_into().unwrap());
        self.decode_candidate(word)
    }
    fn decode_candidate(&self,word:impl Fn(usize)->u32)->Result<(Vec<f64>,Vec<f64>),JointDotError> {
        if (0..self.immutable_end).any(|i|word(i)!=self.words[i])||word(self.status)!=0 {return Err(JointDotError::Output);}
        let scalar=|offset| {let hi=f32::from_bits(word(offset)) as f64;let lo=f32::from_bits(word(offset+1)) as f64;
            if hi.is_finite()&&lo.is_finite() {Ok(hi+lo)} else {Err(JointDotError::Output)}};
        let state=(0..self.width).map(|i| {let v=rescale(scalar(self.state+2*i)?,self.state_scale,1.);
            v.is_finite().then_some(v).ok_or(JointDotError::Output)}).collect::<Result<Vec<_>,_>>()?;
        let reactions=(0..self.rows).map(|i| {let v=rescale(scalar(self.reactions+2*i)?,self.state_scale,1./self.column_scale);
            (v.is_finite()&&v>=0.).then_some(v).ok_or(JointDotError::Output)}).collect::<Result<Vec<_>,_>>()?;
        Ok((state,reactions))
    }
}

/// Independent complete operators dispatched concurrently in disjoint workgroups.
/// The header and every immutable original operator are audited on readback.
#[derive(Debug)]
pub struct ExperimentalResidentContactBatch {
    inputs:Vec<ExperimentalResidentContactInput>,words:Vec<u32>,offsets:Vec<usize>,header_end:usize,
}
impl ExperimentalResidentContactBatch {
    /// All-or-nothing packing; never partitions a physical operator or drops rows.
    pub fn new(mut inputs:Vec<ExperimentalResidentContactInput>)->Result<Self,JointDotError> {
        if inputs.is_empty()||inputs.len()>65535 {return Err(JointDotError::Shape);}
        let header_end=4usize.checked_add(inputs.len()).ok_or(JointDotError::Capacity)?;
        let total=inputs.iter().try_fold(header_end,|n,i|n.checked_add(i.words.len()))
            .filter(|&n|n<=16*1024*1024).ok_or(JointDotError::Capacity)?;
        let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|JointDotError::Capacity)?;
        words.extend([u32::MAX,inputs.len() as u32,header_end as u32,0]);
        let mut offsets=Vec::with_capacity(inputs.len());let mut offset=header_end;
        for input in &inputs {words.push(offset as u32);offsets.push(offset);offset+=input.words.len();}
        for input in &mut inputs {
            words.extend_from_slice(&input.words);
            // The batch owns the complete upload. Keep only the immutable
            // certificate per input, not a second copy of private QR scratch.
            input.words=input.words[..input.immutable_end].to_vec();
        }
        Ok(Self {inputs,words,offsets,header_end})
    }
    pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
    pub fn workgroups(&self)->[u32;3] {[self.inputs.len() as u32,1,1]}
    /// Ordered byte ranges for an audited readback without private QR scratch.
    /// Concatenate these ranges exactly; every original immutable word remains.
    pub fn candidate_readback_ranges(&self)->Vec<std::ops::Range<u64>> {
        let mut ranges=Vec::with_capacity(1+2*self.inputs.len());
        ranges.push(0..(self.header_end*4) as u64);
        for (input,&offset) in self.inputs.iter().zip(&self.offsets) {
            ranges.push((offset*4) as u64..((offset+input.immutable_end)*4) as u64);
            ranges.push(((offset+input.reactions)*4) as u64..((offset+input.total_words)*4) as u64);
        }
        ranges
    }
    /// Same publication gates as full readback; original physical admission follows.
    pub fn candidate_from_compact_readback(&self,bytes:&[u8])->Result<Vec<(Vec<f64>,Vec<f64>)>,JointDotError> {
        let expected=self.header_end*4+self.inputs.iter().map(|i|
            (i.immutable_end+i.total_words-i.reactions)*4).sum::<usize>();
        if bytes.len()!=expected || bytes[..self.header_end*4]!=self.bytes()[..self.header_end*4] {
            return Err(JointDotError::Output);
        }
        let mut cursor=self.header_end*4;
        self.inputs.iter().map(|input| {
            let length=(input.immutable_end+input.total_words-input.reactions)*4;
            let current=&bytes[cursor..cursor+length];cursor+=length;
            input.decode_candidate(|index| {
                let compact=if index<input.immutable_end {index}
                    else {input.immutable_end+index-input.reactions};
                u32::from_le_bytes(current[compact*4..compact*4+4].try_into().unwrap())
            })
        }).collect()
    }

    /// Publish a complete batch only after all operators pass ABI/output checks.
    /// Original f64 physical admission is still required by each owner.
    pub fn candidate(&self,bytes:&[u8])->Result<Vec<(Vec<f64>,Vec<f64>)>,JointDotError> {
        if bytes.len()!=self.words.len()*4 {return Err(JointDotError::Output);}
        if bytes[..self.header_end*4]!=self.bytes()[..self.header_end*4] {return Err(JointDotError::Output);}
        self.inputs.iter().zip(&self.offsets).map(|(input,&offset)|
            input.candidate(&bytes[offset*4..(offset+input.total_words)*4])).collect()
    }
}

}
fn main() {
let values=[0.,-0.,1.,-1.,1e-100,1e100,f64::MIN_POSITIVE,f64::from_bits(1),f64::MAX,f64::NAN,f64::INFINITY];
let mut count=0;
for &a in &values { for &b in &values {for &bound in &values {
let columns=vec![vec![a,b],vec![b,a]];let bounds=vec![bound,1.];
for tol in [1e-14,0.,f64::NAN] {
let before=old::ExperimentalResidentContactInput::new(&columns,&bounds,tol);
let after=new::ExperimentalResidentContactInput::new(&columns,&bounds,tol);
match (before,after) {(Ok(a),Ok(b))=>assert_eq!(a.bytes(),b.bytes()),(Err(a),Err(b))=>assert_eq!(a,b),_=>panic!("acceptance changed")};count+=1;
}}}}
println!("bitwise upload/error equality cases={count}");}
