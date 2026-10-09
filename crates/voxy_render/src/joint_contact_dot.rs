//! Codec for a rectangular contact operator, never a compliance Gram.
//! Decoding checks transport only; physics must admit ORIGINAL loads/bounds.
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
impl JointContactDotInput {
    /// For JOINT_CONTACT_NORMALIZE_SHADER only. Readback is [norm, unit vector].
    /// Uses one 64-lane workgroup; no whole-solver speed claim is implied.
    pub fn normalization(vector:&[f64])->Result<Self,JointDotError> {
        if vector.is_empty() || vector.iter().any(|v|!v.is_finite()) {return Err(JointDotError::Shape);}
        let scale=vector.iter().map(|v|v.abs()).fold(0.,f64::max);
        if scale==0. {return Err(JointDotError::NumericRange);}
        let rows=vector.len().checked_add(1).ok_or(JointDotError::Capacity)?;
        let output=vector.len().checked_mul(2).and_then(|n|n.checked_add(4)).ok_or(JointDotError::Capacity)?;
        let total=rows.checked_mul(3).and_then(|n|output.checked_add(n)).filter(|&n|n<=u32::MAX as usize).ok_or(JointDotError::Capacity)?;
        let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|JointDotError::Capacity)?;
        words.extend([rows as u32,vector.len() as u32,1,0]);
        let mut packing_error=0f64;
        for &v in vector {
            let normalized=v/scale;
            if v!=0. && normalized==0. {return Err(JointDotError::NumericRange);}
            let pair=pack(normalized)?;
            packing_error=packing_error.max((normalized-(f32::from_bits(pair[0]) as f64+f32::from_bits(pair[1]) as f64)).abs());
            words.extend(pair);
        }
        words.resize(output+rows*2,0);words.resize(total,u32::MAX);
        let mut scales=vec![1.;rows];scales[0]=scale;
        Ok(Self {words,scales,vector_scale:1.,output,rows,packing_error})
    }
    /// Encode v - sum_j(q_j * alpha_j), one GPU invocation per coordinate.
    /// Basis columns are immutable. This is a QR vector-update stage only;
    /// callers must still validate the original physical joint solution.
    pub fn projection(basis:&[Vec<f64>],coefficients:&[f64],vector:&[f64])->Result<Self,JointDotError> {
        if basis.len()!=coefficients.len() || vector.is_empty()
            || basis.iter().any(|q|q.len()!=vector.len()) {return Err(JointDotError::Shape);}
        let width=basis.len().checked_add(1).ok_or(JointDotError::Capacity)?;
        let mut rows=Vec::new();rows.try_reserve_exact(vector.len()).map_err(|_|JointDotError::Capacity)?;
        for (i,&value) in vector.iter().enumerate() {
            let mut row=Vec::new();row.try_reserve_exact(width).map_err(|_|JointDotError::Capacity)?;
            row.push(value);row.extend(basis.iter().map(|q|q[i]));rows.push(row);
        }
        let mut weights=Vec::new();weights.try_reserve_exact(width).map_err(|_|JointDotError::Capacity)?;
        weights.push(1.);weights.extend(coefficients.iter().map(|a|-a));
        Self::new(&rows,&weights)
    }
    pub fn new(columns:&[Vec<f64>],coordinates:&[f64])->Result<Self,JointDotError> {
        let rows=columns.len();let width=coordinates.len();
        if rows==0 || width==0 || columns.iter().any(|c|c.len()!=width)
            || columns.iter().flatten().chain(coordinates).any(|v|!v.is_finite()) {
            return Err(JointDotError::Shape);
        }
        let output=rows.checked_mul(width).and_then(|n|n.checked_mul(2))
            .and_then(|n|width.checked_mul(2).and_then(|v|n.checked_add(v)))
            .and_then(|n|n.checked_add(4)).ok_or(JointDotError::Capacity)?;
        let total=rows.checked_mul(3).and_then(|n|output.checked_add(n))
            .filter(|&n|n<=u32::MAX as usize).ok_or(JointDotError::Capacity)?;
        let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|JointDotError::Capacity)?;
        words.extend([u32::try_from(rows).map_err(|_|JointDotError::Capacity)?,
            u32::try_from(width).map_err(|_|JointDotError::Capacity)?,0,0]);
        let vector_scale=coordinates.iter().map(|v|v.abs()).fold(0.,f64::max).max(f64::MIN_POSITIVE);
        let mut scales=Vec::with_capacity(rows);let mut packing_error=0f64;
        for column in columns {
            let scale=column.iter().map(|v|v.abs()).fold(0.,f64::max).max(f64::MIN_POSITIVE);
            scales.push(scale);
            for value in column {
                let normalized=value/scale;
                if *value!=0. && normalized==0. {return Err(JointDotError::NumericRange);}
                let pair=pack(normalized)?;
                packing_error=packing_error.max((normalized-(f32::from_bits(pair[0]) as f64+f32::from_bits(pair[1]) as f64)).abs());
                words.extend(pair);
            }
        }
        for value in coordinates {
            let normalized=value/vector_scale;
            if *value!=0. && normalized==0. {return Err(JointDotError::NumericRange);}
            let pair=pack(normalized)?;
            packing_error=packing_error.max((normalized-(f32::from_bits(pair[0]) as f64+f32::from_bits(pair[1]) as f64)).abs());
            words.extend(pair);
        }
        words.resize(output+rows*2,0);words.resize(total,u32::MAX);
        Ok(Self {words,scales,vector_scale,output,rows,packing_error})
    }
    pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
    pub fn dispatch(&self)->(u32,u32,u32) {(if self.words[2]==1 {1} else {(self.rows as u32).div_ceil(64)},1,1)}
    pub fn packing_error(&self)->f64 {self.packing_error}
    pub fn decode(&self,bytes:&[u8])->Result<Vec<f64>,JointDotError> {
        if bytes.len()!=self.words.len()*4 {return Err(JointDotError::Output);}
        let words:Vec<_>=bytes.chunks_exact(4).map(|b|u32::from_le_bytes(b.try_into().unwrap())).collect();
        if words[..self.output]!=self.words[..self.output] {return Err(JointDotError::Output);}
        let mut values=Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            if words[self.output+self.rows*2+row]!=0 {return Err(JointDotError::Output);}
            let hi=f32::from_bits(words[self.output+row*2]) as f64;
            let lo=f32::from_bits(words[self.output+row*2+1]) as f64;
            let value=rescale(hi+lo,self.scales[row],self.vector_scale);
            if !hi.is_finite()||!lo.is_finite()||!value.is_finite() {return Err(JointDotError::Output);}
            values.push(value);
        }
        Ok(values)
    }
}
pub const JOINT_CONTACT_DOT_SHADER:&str=include_str!("joint_contact_dot.wgsl");
pub const JOINT_CONTACT_NORMALIZE_SHADER:&str=include_str!("joint_contact_normalize.wgsl");
pub const JOINT_CONTACT_NORMALIZE_SERIAL_SHADER:&str=include_str!("joint_contact_normalize_serial.wgsl");
pub const JOINT_CONTACT_QR_SHADER:&str=include_str!("joint_contact_qr.wgsl");
pub const JOINT_CONTACT_EQUALITY_SHADER:&str=include_str!("joint_contact_equality.wgsl");
#[derive(Debug)]
pub struct ResidentContactEqualityInput {qr:ResidentContactQrInput,words:Vec<u32>,scale:f64,rhs:usize,output:usize}
#[derive(Debug)]
pub struct ResidentContactEqualityOutput {pub coordinates:Vec<f64>,pub reactions:Vec<f64>}
impl ResidentContactEqualityInput {
    pub fn new(columns:&[Vec<f64>],bounds:&[f64])->Result<Self,JointDotError> {
        Self::new_with_support_compaction(columns,bounds,true)
    }
    /// Qualification switch for paired measurements of the same operator.
    pub fn new_with_support_compaction(columns:&[Vec<f64>],bounds:&[f64],compact:bool)->Result<Self,JointDotError> {
        let qr=ResidentContactQrInput::new_with_support_compaction(columns,compact)?;
        if bounds.len()!=qr.count || bounds.iter().any(|v|!v.is_finite()) {return Err(JointDotError::Shape);}
        let mut normalized=Vec::with_capacity(qr.count);
        for (&b,&s) in bounds.iter().zip(&qr.scales) {
            let v=b/s;
            if !v.is_finite() || !s.recip().is_finite() || (b!=0. && v==0.) {return Err(JointDotError::NumericRange);}
            normalized.push(v);
        }
        let scale=normalized.iter().map(|v|v.abs()).fold(0.,f64::max).max(f64::MIN_POSITIVE);
        let rhs=qr.words.len();
        let additional=qr.count.checked_mul(6).and_then(|n|qr.width.checked_mul(2).and_then(|m|n.checked_add(m))).and_then(|n|n.checked_add(1)).ok_or(JointDotError::Capacity)?;
        let total=rhs.checked_add(additional).filter(|&n|n<=u32::MAX as usize).ok_or(JointDotError::Capacity)?;
        let mut words=qr.words.clone();words.try_reserve_exact(additional).map_err(|_|JointDotError::Capacity)?;
        for v in normalized {let n=v/scale;if v!=0. && n==0. {return Err(JointDotError::NumericRange);}words.extend(pack(n)?);}
        let output=rhs+qr.count*6;words.resize(total,0);words[total-1]=u32::MAX;
        Ok(Self {qr,words,scale,rhs,output})
    }
    pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
    pub fn columns(&self)->usize {self.qr.count}
    /// Change only equality RHS/scratch for this immutable resident QR.
    /// Validation is transactional; rejected bounds leave the input untouched.
    pub fn update_bounds(&mut self,bounds:&[f64])->Result<(),JointDotError> {
        if bounds.len()!=self.qr.count || bounds.iter().any(|v|!v.is_finite()) {return Err(JointDotError::Shape);}
        let mut normalized=Vec::with_capacity(bounds.len());
        for (&b,&s) in bounds.iter().zip(&self.qr.scales) {
            let v=b/s;
            if !v.is_finite() || !s.recip().is_finite() || (b!=0. && v==0.) {return Err(JointDotError::NumericRange);}
            normalized.push(v);
        }
        let scale=normalized.iter().map(|v|v.abs()).fold(0.,f64::max).max(f64::MIN_POSITIVE);
        let mut packed=Vec::with_capacity(bounds.len()*2);
        for v in normalized {let n=v/scale;if v!=0. && n==0. {return Err(JointDotError::NumericRange);}packed.extend(pack(n)?);}
        self.words[self.rhs..].fill(0);
        self.words[self.rhs..self.rhs+packed.len()].copy_from_slice(&packed);
        *self.words.last_mut().unwrap()=u32::MAX;self.scale=scale;
        Ok(())
    }
    /// Upload this range only, retaining original columns, completed Q and R.
    pub fn equality_update(&self)->(u64,&[u8]) {
        ((self.rhs*4) as u64,bytemuck::cast_slice(&self.words[self.rhs..]))
    }
    pub fn decode(&self,bytes:&[u8])->Result<ResidentContactEqualityOutput,JointDotError> {
        if bytes.len()!=self.words.len()*4 {return Err(JointDotError::Output);}
        // Equality admission needs the QR validity gates, not a second host
        // copy of every expanded basis vector and triangular column.
        self.qr.checked_words(&bytes[..self.rhs*4])?;
        let words:Vec<_>=bytes.chunks_exact(4).map(|b|u32::from_le_bytes(b.try_into().unwrap())).collect();
        if words[self.rhs..self.rhs+self.qr.count*2]!=self.words[self.rhs..self.rhs+self.qr.count*2] || words[words.len()-1]!=0 {return Err(JointDotError::Output);}
        let pair=|offset:usize|->Result<f64,JointDotError> {
            let hi=f32::from_bits(words[offset]) as f64;let lo=f32::from_bits(words[offset+1]) as f64;
            if !hi.is_finite()||!lo.is_finite() {return Err(JointDotError::Output);}Ok(hi+lo)
        };
        let compact=(0..self.qr.width).map(|i| {
            let v=rescale(pair(self.output+i*2)?,self.scale,1.);
            if !v.is_finite() {return Err(JointDotError::Output);}Ok(v)
        }).collect::<Result<Vec<_>,_>>()?;
        let mut coordinates=vec![0.;self.qr.original_width];
        for (&id,value) in self.qr.ids.iter().zip(compact) {coordinates[id]=value;}
        let reactions=(0..self.qr.count).map(|i| {
            let v=rescale(pair(self.rhs+self.qr.count*4+i*2)?,self.scale,self.qr.scales[i].recip());
            if !v.is_finite() {return Err(JointDotError::Output);}Ok(v)
        }).collect::<Result<Vec<_>,_>>()?;
        Ok(ResidentContactEqualityOutput {coordinates,reactions})
    }
}
/// Experimental full-column QR workspace. A single owner holds original packed
/// columns, Q and R until all ordered column dispatches finish.
#[derive(Debug)]
pub struct ResidentContactQrInput {words:Vec<u32>,scales:Vec<f64>,width:usize,original_width:usize,ids:Vec<usize>,count:usize,input_end:usize}
#[derive(Debug)]
pub struct ResidentContactQrOutput {pub basis:Vec<Vec<f64>>,pub triangular_columns:Vec<Vec<f64>>}
impl ResidentContactQrInput {
    pub fn new(columns:&[Vec<f64>])->Result<Self,JointDotError> {
        Self::new_with_support_compaction(columns,true)
    }
    pub fn new_with_support_compaction(columns:&[Vec<f64>],compact:bool)->Result<Self,JointDotError> {
        let count=columns.len();let original_width=columns.first().map_or(0,Vec::len);let width=original_width;
        if count==0 || width==0 || count>width || columns.iter().any(|c|c.len()!=width || c.iter().any(|v|!v.is_finite())) {return Err(JointDotError::Shape);}
        // Exact structural support only: never use a magnitude threshold.
        let ids:Vec<_>=(0..width).filter(|&i|!compact || columns.iter().any(|c|c[i]!=0.)).collect();
        let width=ids.len();
        if count>width {return Err(JointDotError::NumericRange);}
        let matrix=count.checked_mul(width).and_then(|n|n.checked_mul(2)).ok_or(JointDotError::Capacity)?;
        let total=count.checked_mul(count).and_then(|n|n.checked_mul(2)).and_then(|n|matrix.checked_mul(2).and_then(|m|m.checked_add(n))).and_then(|n|n.checked_add(4)).filter(|&n|n<=u32::MAX as usize).ok_or(JointDotError::Capacity)?;
        let mut words=Vec::new();words.try_reserve_exact(total).map_err(|_|JointDotError::Capacity)?;
        words.extend([width as u32,count as u32,0,0]);let mut scales=Vec::with_capacity(count);
        for c in columns {
            let scale=c.iter().map(|v|v.abs()).fold(0.,f64::max);
            if scale==0. {return Err(JointDotError::NumericRange);}
            scales.push(scale);
            for &i in &ids {let v=c[i];let normalized=v/scale;if v!=0. && normalized==0. {return Err(JointDotError::NumericRange);}words.extend(pack(normalized)?);}
        }
        let input_end=words.len();words.resize(total,0);
        Ok(Self {words,scales,width,original_width,ids,count,input_end})
    }
    pub fn bytes(&self)->&[u8] {bytemuck::cast_slice(&self.words)}
    pub fn columns(&self)->usize {self.count}
    fn checked_words(&self,bytes:&[u8])->Result<Vec<u32>,JointDotError> {
        if bytes.len()!=self.words.len()*4 {return Err(JointDotError::Output);}
        let words:Vec<_>=bytes.chunks_exact(4).map(|b|u32::from_le_bytes(b.try_into().unwrap())).collect();
        if words[..2]!=self.words[..2] || words[2]!=self.count as u32 || words[3]!=0 || words[4..self.input_end]!=self.words[4..self.input_end] {return Err(JointDotError::Output);}
        let pair=|offset:usize|->Result<f64,JointDotError> {
            let hi=f32::from_bits(words[offset]) as f64;let lo=f32::from_bits(words[offset+1]) as f64;
            if !hi.is_finite() || !lo.is_finite() {return Err(JointDotError::Output);}Ok(hi+lo)
        };
        let r=self.input_end+self.count*self.width*2;
        for j in 0..self.count {
            for i in 0..self.width {pair(self.input_end+(j*self.width+i)*2)?;}
            for i in 0..=j {
                let v=rescale(pair(r+(j*self.count+i)*2)?,self.scales[j],1.);
                if !v.is_finite() || (i==j && v<=0.) {return Err(JointDotError::Output);}
            }
        }
        Ok(words)
    }
    pub fn decode(&self,bytes:&[u8])->Result<ResidentContactQrOutput,JointDotError> {
        let words=self.checked_words(bytes)?;
        let pair=|offset:usize|f32::from_bits(words[offset]) as f64+f32::from_bits(words[offset+1]) as f64;
        let mut basis=Vec::with_capacity(self.count);let mut triangular_columns=Vec::with_capacity(self.count);
        let r=self.input_end+self.count*self.width*2;
        for j in 0..self.count {
            let mut q=vec![0.;self.original_width];
            for (i,&original) in self.ids.iter().enumerate() {q[original]=pair(self.input_end+(j*self.width+i)*2);}
            let column=(0..=j).map(|i| {
                rescale(pair(r+(j*self.count+i)*2),self.scales[j],1.)
            }).collect::<Vec<_>>();
            basis.push(q);triangular_columns.push(column);
        }
        Ok(ResidentContactQrOutput {basis,triangular_columns})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resident_qr_shader_and_publication_gates() {
        let module=naga::front::wgsl::parse_str(JOINT_CONTACT_QR_SHADER).unwrap();
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(&module).unwrap();
        let input=ResidentContactQrInput::new(&[vec![1.,0.],vec![0.,1.]]).unwrap();
        assert_eq!(input.columns(),2);assert!(input.decode(input.bytes()).is_err());
        assert!(ResidentContactQrInput::new(&[vec![0.,0.]]).is_err());
        assert!(ResidentContactQrInput::new(&[vec![1.],vec![1.]]).is_err());
    }
    #[test]
    fn resident_qr_compacts_only_exact_zero_axes() {
        let input=ResidentContactQrInput::new(&[vec![0.,1.,-0.,1e-30,1e-30],vec![0.,0.,0.,1.,0.]]).unwrap();
        assert_eq!(input.ids,vec![1,3,4]);assert_eq!(input.width,3);assert_eq!(input.original_width,5);
        assert!(ResidentContactQrInput::new(&[vec![0.,1.,f64::NAN]]).is_err());
        let input=ResidentContactEqualityInput::new(&[vec![0.,2.,0.]],&[6.]).unwrap();
        let mut words=input.words.clone();words[2]=1;
        words[input.qr.input_end]=1f32.to_bits();
        words[input.qr.input_end+2]=1f32.to_bits();
        words[input.rhs+2]=1f32.to_bits();words[input.rhs+4]=1f32.to_bits();
        words[input.output]=1f32.to_bits();*words.last_mut().unwrap()=0;
        let output=input.decode(bytemuck::cast_slice(&words)).unwrap();
        assert_eq!(output.coordinates,vec![0.,3.,0.]);assert_eq!(output.reactions,vec![1.5]);
        // Validation-only equality decoding must retain every QR publication
        // gate even though it no longer allocates expanded host Q/R copies.
        for (offset,value) in [(input.qr.input_end,f32::NAN.to_bits()),
            (input.qr.input_end+2,f32::INFINITY.to_bits()),
            (input.qr.input_end+2,(-1f32).to_bits())] {
            let mut corrupt=words.clone();corrupt[offset]=value;
            assert_eq!(input.decode(bytemuck::cast_slice(&corrupt)).unwrap_err(),JointDotError::Output);
        }
    }
    #[test]
    fn equality_rhs_update_preserves_operator_and_rejects_bad_bounds() {
        let mut input=ResidentContactEqualityInput::new(&[vec![0.,2.,0.]],&[6.]).unwrap();
        let prefix=input.words[..input.rhs].to_vec();
        input.update_bounds(&[-12.]).unwrap();
        assert_eq!(&input.words[..input.rhs],prefix);
        let fresh=ResidentContactEqualityInput::new(&[vec![0.,2.,0.]],&[-12.]).unwrap();
        assert_eq!(input.words,fresh.words);assert_eq!(input.scale,fresh.scale);
        let saved=input.words.clone();let scale=input.scale;
        for bounds in [vec![],vec![f64::NAN],vec![f64::INFINITY]] {
            assert!(input.update_bounds(&bounds).is_err());
            assert_eq!(input.words,saved);assert_eq!(input.scale,scale);
        }
        let (offset,bytes)=input.equality_update();
        assert_eq!(offset,input.rhs as u64*4);
        assert_eq!(bytes,&input.bytes()[offset as usize..]);
        let mut tiny=ResidentContactEqualityInput::new(&[vec![1e-300]],&[0.]).unwrap();
        let saved=tiny.words.clone();let scale=tiny.scale;
        assert_eq!(tiny.update_bounds(&[f64::MAX]),Err(JointDotError::NumericRange));
        assert_eq!(tiny.words,saved);assert_eq!(tiny.scale,scale);
    }
    #[test]
    fn resident_equality_shader_and_transport_gates() {
        let module=naga::front::wgsl::parse_str(JOINT_CONTACT_EQUALITY_SHADER).unwrap();
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(&module).unwrap();
        let input=ResidentContactEqualityInput::new(&[vec![1.,0.],vec![0.,1.]],&[2.,3.]).unwrap();
        assert!(input.decode(input.bytes()).is_err());
        assert!(ResidentContactEqualityInput::new(&[vec![1.]],&[]).is_err());
        assert!(ResidentContactEqualityInput::new(&[vec![1.]],&[f64::INFINITY]).is_err());
    }
    #[test]
    fn normalization_shader_and_zero_range_gates() {
        for shader in [JOINT_CONTACT_NORMALIZE_SHADER,JOINT_CONTACT_NORMALIZE_SERIAL_SHADER] {
            let module=naga::front::wgsl::parse_str(shader).unwrap();
            naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(&module).unwrap();
        }
        assert!(JointContactDotInput::normalization(&[0.,0.]).is_err());
        assert!(JointContactDotInput::normalization(&[f64::NAN]).is_err());
        assert!(JointContactDotInput::normalization(&[f64::MAX,f64::from_bits(1)]).is_err());
        let input=JointContactDotInput::normalization(&[3.,4.]).unwrap();
        assert_eq!(input.rows,3);assert_eq!(input.scales,vec![4.,1.,1.]);
        assert!(input.decode(input.bytes()).is_err());
    }
    #[test]
    fn qr_projection_layout_preserves_original_coordinate_order() {
        let input=JointContactDotInput::projection(&[vec![1.,0.,0.],vec![0.,1.,0.]],&[2.,-3.],&[4.,5.,6.]).unwrap();
        assert_eq!(input.rows,3);
        assert_eq!(input.words[1],3);
        assert_eq!(input.dispatch(),(1,1,1));
        let unpack=|offset:usize| f32::from_bits(input.words[offset]) as f64+f32::from_bits(input.words[offset+1]) as f64;
        let weights=4+3*3*2;
        for (i,expected) in [2.,8.,6.].into_iter().enumerate() {
            let dot=(0..3).map(|j|unpack(4+(i*3+j)*2)*unpack(weights+j*2)).sum::<f64>();
            assert!((dot*input.scales[i]*input.vector_scale-expected).abs()<1e-13);
        }
        // With no basis, publication still runs through the GPU codec.
        let identity=JointContactDotInput::projection(&[],&[],&[1.,-2.]).unwrap();
        assert_eq!(identity.words[1],1);
        assert!(JointContactDotInput::projection(&[vec![1.]],&[],&[1.]).is_err());
        assert!(JointContactDotInput::projection(&[vec![1.]],&[f64::NAN],&[1.]).is_err());
        assert!(JointContactDotInput::projection(&[vec![1.]],&[1.],&[1.,2.]).is_err());
    }
    #[test]
    fn rescaling_recovers_finite_values_after_intermediate_range_loss() {
        assert_eq!(rescale(2.,f64::MAX,0.25),f64::MAX*0.5);
        assert_eq!(rescale(-2.,f64::MAX,0.25),-f64::MAX*0.5);
        assert_eq!(rescale(f64::MIN_POSITIVE,f64::MIN_POSITIVE,2f64.powi(1023)),2f64.powi(-1021));
        assert_eq!(rescale(1.,f64::MIN_POSITIVE,f64::MIN_POSITIVE),0.);
        assert!(rescale(2.,f64::MAX,1.).is_infinite());
        assert_eq!(rescale(1.,f64::from_bits(1),1.),f64::from_bits(1));
        assert_eq!(rescale(-0.,f64::MAX,1.).to_bits(),(-0f64).to_bits());
        let input=JointContactDotInput::new(&[vec![f64::MAX,f64::MAX]],&[0.25,0.25]).unwrap();
        let mut words=input.words.clone();
        words[input.output]=2f32.to_bits();words[input.output+1]=0;
        words[input.output+2]=0;
        assert_eq!(input.decode(bytemuck::cast_slice(&words)).unwrap(),vec![f64::MAX*0.5]);
    }
    #[test]
    fn numeric_range_and_every_row_publication_are_checked() {
        // Normalization must never silently erase a nonzero physical load.
        for tiny in [f64::from_bits(1), 1e-200] {
            assert_eq!(JointContactDotInput::new(&[vec![f64::MAX,tiny]],&[1.,1.]).unwrap_err(),JointDotError::NumericRange);
            assert_eq!(JointContactDotInput::new(&[vec![1.,1.]],&[f64::MAX,tiny]).unwrap_err(),JointDotError::NumericRange);
        }
        let input=JointContactDotInput::new(&[vec![1.],vec![-1.]],&[1.]).unwrap();
        let mut words=input.words.clone();
        words[input.output..input.output+4].copy_from_slice(&[1f32.to_bits(),0,(-1f32).to_bits(),0]);
        let status=input.output+4;
        words[status..].fill(0);
        assert_eq!(input.decode(bytemuck::cast_slice(&words)).unwrap(),vec![1.,-1.]);
        for row in 0..2 {
            words[status+row]=1;
            assert_eq!(input.decode(bytemuck::cast_slice(&words)),Err(JointDotError::Output));
            words[status+row]=0;
            for value in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY] {
                let offset=input.output+2*row;
                let saved=words[offset];words[offset]=value.to_bits();
                assert_eq!(input.decode(bytemuck::cast_slice(&words)),Err(JointDotError::Output));
                words[offset]=saved;
            }
        }
        assert_eq!(input.decode(&input.bytes()[..input.bytes().len()-4]),Err(JointDotError::Output));
    }
    #[test]
    fn shader_validates_and_codec_rejects_missing_or_changed_outputs() {
        let module=naga::front::wgsl::parse_str(JOINT_CONTACT_DOT_SHADER).unwrap();
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(),naga::valid::Capabilities::all()).validate(&module).unwrap();
        let input=JointContactDotInput::new(&[vec![1.,2.],vec![0.,0.]],&[3.,4.]).unwrap();
        assert!(input.decode(input.bytes()).is_err());
        let mut words=input.words.clone();
        words[input.output..input.output+4].copy_from_slice(&[1f32.to_bits(),0,0,0]);
        words[input.output+4..].fill(0);
        assert_eq!(input.decode(bytemuck::cast_slice(&words)).unwrap(),vec![8.,0.]);
        words[2]=1;assert!(input.decode(bytemuck::cast_slice(&words)).is_err());
        assert!(JointContactDotInput::new(&[vec![1.]],&[1.,2.]).is_err());
        assert!(JointContactDotInput::new(&[vec![f64::NAN]],&[1.]).is_err());
    }
}
