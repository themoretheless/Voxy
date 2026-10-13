//! Exact dyadic verifier; independently checked against BigRational.
use super::*;
use num_bigint::BigInt;
#[derive(Clone,Debug)]
struct D {n:BigInt,e:i32}
impl D {
    fn normalized(n:BigInt,e:i32)->Self {
        if n==BigInt::from(0) {return Self {n,e:0};}
        let shift=n.trailing_zeros().unwrap() as i32;
        Self {n:n>>(shift as usize),e:e+shift}
    }
    fn from_r(value:&R)->Option<Self> {
        let bits=value.denom().bits();
        if value.denom().trailing_zeros()?!=bits.checked_sub(1)? {return None;}
        Some(Self::normalized(value.numer().clone(),-i32::try_from(bits-1).ok()?))
    }
    fn integer(n:i32)->Self {Self::normalized(n.into(),0)}
    fn add(&self,b:&Self)->Self {
        let e=self.e.min(b.e);
        Self::normalized((&self.n<<((self.e-e) as usize))+(&b.n<<((b.e-e) as usize)),e)
    }
    fn mul(&self,b:&Self)->Self {Self::normalized(&self.n*&b.n,self.e+b.e)}
    fn neg(&self)->Self {Self {n:-&self.n,e:self.e}}
    fn positive(&self)->bool {self.n>BigInt::from(0)}
    fn nonnegative(&self)->bool {self.n>=BigInt::from(0)}
    fn rational(&self)->R {
        if self.e>=0 {R::from_integer(&self.n<<(self.e as usize))}
        else {R::new(self.n.clone(),BigInt::from(1)<<((-self.e) as usize))}
    }
}
fn sum(a:&[D],b:&[D])->Vec<D> {
    (0..a.len().max(b.len())).map(|i|a.get(i).cloned().unwrap_or_else(||D::integer(0)).add(&b.get(i).cloned().unwrap_or_else(||D::integer(0)))).collect()
}
fn multiply(a:&[D],b:&[D])->Vec<D> {
    let mut result=vec![D::integer(0);a.len()+b.len()-1];
    for (i,x) in a.iter().enumerate() {for (j,y) in b.iter().enumerate() {result[i+j]=result[i+j].add(&x.mul(y));}}
    result
}
fn bernstein_nonnegative(power:&[D],degree:usize,strict:bool)->bool {
    // 840 is divisible by every binomial coefficient for degrees 0..=8.
    // Multiplying by this positive constant preserves the exact sign.
    if degree>8 || power.len()>degree+1 {return false;}
    (0..=degree).all(|i| {
        let value=(0..=i).fold(D::integer(0),|s,j| {
            let weight=choose(i,j)*(840/choose(degree,j));
            s.add(&power.get(j).cloned().unwrap_or_else(||D::integer(0)).mul(&D::integer(weight as i32)))
        });
        if strict {value.positive()} else {value.nonnegative()}
    })
}
struct PreparedMotion {
    radius:D,
    first:Vec<[D;3]>,
    slope:Vec<[D;3]>,
}
impl PreparedMotion {
    fn new(motion:&ExactMotion)->Option<Self> {
        let convert=|values:&[[[R;3];3];2]| {
            values.iter().flatten().map(|v|Some([D::from_r(&v[0])?,D::from_r(&v[1])?,D::from_r(&v[2])?])).collect::<Option<Vec<_>>>()
        };
        Some(Self {radius:D::from_r(&motion.radius)?,first:convert(&motion.first)?,slope:convert(&motion.slope)?})
    }
    fn verify(&self,lo:&R,hi:&R,axis:&[Vec<R>;3])->Option<bool> {
        if hi<=lo {return Some(false);}
        let axis=axis.iter().map(|a|a.iter().map(D::from_r).collect::<Option<Vec<_>>>()).collect::<Option<Vec<_>>>()?;
        let axis:[Vec<D>;3]=axis.try_into().ok()?;
        Some(self.interval(lo,hi)?.verify(&axis))
    }
    fn interval(&self,lo:&R,hi:&R)->Option<IntervalGeometry> {
        let width=D::from_r(&(hi-lo))?;let lo=D::from_r(lo)?;
        Some(IntervalGeometry {radius:self.radius.clone(),relative:self.first.iter().zip(&self.slope)
            .map(|(p,v)|std::array::from_fn(|k|vec![p[k].add(&lo.mul(&v[k])),width.mul(&v[k])])).collect()})
    }
}
// Prepared affine relative coordinates shared by axis construction and proof.
struct IntervalGeometry {radius:D,relative:Vec<[Vec<D>;3]>}
fn dyadic_cross(a:&[Vec<D>;3],b:&[Vec<D>;3])->[Vec<D>;3] {
    std::array::from_fn(|i| {let j=(i+1)%3;let k=(i+2)%3;
        sum(&multiply(&a[j],&b[k]),&multiply(&a[k],&b[j]).into_iter().map(|v|v.neg()).collect::<Vec<_>>())})
}
fn dyadic_dot(a:&[Vec<D>;3],b:&[Vec<D>;3])->Vec<D> {
    (0..3).fold(vec![D::integer(0)],|s,k|sum(&s,&multiply(&a[k],&b[k])))
}
fn dyadic_perpendicular(p:&[Vec<D>;3],e:&[Vec<D>;3])->[Vec<D>;3] {
    let norm=dyadic_dot(e,e);let projection=dyadic_dot(p,e);
    std::array::from_fn(|k|sum(&multiply(&p[k],&norm),&multiply(&e[k],&projection).into_iter().map(|v|v.neg()).collect::<Vec<_>>()))
}
impl IntervalGeometry {
    fn point(&self,e:usize,v:usize)->[Vec<D>;3] {self.relative[e*3+v].clone()}
    fn difference(&self,a:usize,b:usize)->[Vec<D>;3] {
        std::array::from_fn(|k|self.relative[a][k].iter().zip(&self.relative[b][k]).map(|(a,b)|a.add(&b.neg())).collect())
    }
    fn axis(&self,index:usize)->[Vec<D>;3] {
        let edges=[(0,1),(1,2),(2,0)];
        match index {
            0=>dyadic_cross(&self.difference(0,1),&self.difference(0,2)),
            1..=3=>{let (a,b)=edges[index-1];dyadic_cross(&self.difference(3,0),&self.difference(a,b))},
            4..=9=>{let i=index-4;let (a,b)=edges[i%3];dyadic_perpendicular(&self.point(i/3,a),&self.difference(a,b))},
            10..=12=>dyadic_perpendicular(&self.point(0,index-10).map(|v|v.into_iter().map(|x|x.neg()).collect()),&self.difference(3,0)),
            13..=18=>self.point((index-13)/3,(index-13)%3),
            _=>unreachable!("only the nineteen exact feature axes exist"),
        }
    }
    fn verify(&self,axis:&[Vec<D>;3])->bool {
        let degree=axis[0].len().saturating_sub(1);
        if axis[0].is_empty() || degree>3 || axis.iter().any(|a|a.len()!=degree+1) || !self.radius.positive() {return false;}
        let norm=dyadic_dot(axis,axis);
        if !bernstein_nonnegative(&norm,2*degree,true) {return false;}
        let negative_norm=norm.iter().map(|v|v.mul(&self.radius).mul(&self.radius).neg()).collect::<Vec<_>>();
        self.relative.iter().all(|p| {
            let projection=dyadic_dot(p,axis);
            bernstein_nonnegative(&projection,degree+1,false)
                && bernstein_nonnegative(&sum(&multiply(&projection,&projection),&negative_norm),2*degree+2,false)
        })
    }
}
fn rational_axis(axis:[Vec<D>;3])->[Vec<R>;3] {
    axis.map(|v|v.into_iter().map(|x|x.rational()).collect())
}
// Scalar dyadic geometry avoids rational polynomial construction at a point.
struct StaticPoint { radius:D, relative:Vec<[D;3]> }
fn scalar_dot(a:&[D;3],b:&[D;3])->D {
    (0..3).fold(D::integer(0),|s,k|s.add(&a[k].mul(&b[k])))
}
fn scalar_cross(a:&[D;3],b:&[D;3])->[D;3] {
    std::array::from_fn(|i| {let j=(i+1)%3;let k=(i+2)%3;a[j].mul(&b[k]).add(&a[k].mul(&b[j]).neg())})
}
fn scalar_perpendicular(p:&[D;3],e:&[D;3])->[D;3] {
    let norm=scalar_dot(e,e);let projection=scalar_dot(p,e);
    std::array::from_fn(|k|p[k].mul(&norm).add(&e[k].mul(&projection).neg()))
}
impl PreparedMotion {
    fn point_at(&self,time:&R)->Option<StaticPoint> {
        let t=D::from_r(time)?;
        Some(StaticPoint {radius:self.radius.clone(),relative:self.first.iter().zip(&self.slope)
            .map(|(p,v)|std::array::from_fn(|k|p[k].add(&t.mul(&v[k])))).collect()})
    }
}
impl StaticPoint {
    fn point(&self,e:usize,v:usize)->[D;3] {self.relative[e*3+v].clone()}
    fn edge(&self,a:usize,b:usize)->[D;3] {
        std::array::from_fn(|k|self.relative[a][k].add(&self.relative[b][k].neg()))
    }
    fn rod(&self)->[D;3] {
        std::array::from_fn(|k|self.relative[3][k].add(&self.relative[0][k].neg()))
    }
    fn axis(&self,index:usize)->[D;3] {
        let edges=[(0,1),(1,2),(2,0)];
        match index {
            0=>scalar_cross(&self.edge(0,1),&self.edge(0,2)),
            1..=3=>{let (a,b)=edges[index-1];scalar_cross(&self.rod(),&self.edge(a,b))},
            4..=9=>{let i=index-4;let (a,b)=edges[i%3];scalar_perpendicular(&self.point(i/3,a),&self.edge(a,b))},
            10..=12=>scalar_perpendicular(&self.point(0,index-10).map(|x|x.neg()),&self.rod()),
            13..=18=>self.point((index-13)/3,(index-13)%3),
            _=>unreachable!("only the nineteen exact feature axes exist"),
        }
    }
    fn verify(&self,axis:&[D;3])->bool {
        if !self.radius.positive() {return false;}
        let norm=scalar_dot(axis,axis);if !norm.positive() {return false;}
        let threshold=norm.mul(&self.radius).mul(&self.radius);
        self.relative.iter().all(|p| {let projection=scalar_dot(p,axis);
            projection.nonnegative() && projection.mul(&projection).add(&threshold.neg()).nonnegative()})
    }
    fn basic_supported(&self)->bool {
        (0..4).any(|i| {let axis=self.axis(i);self.verify(&axis)||self.verify(&axis.map(|x|x.neg()))})
    }
}
pub(super) fn verify(motion:&ExactMotion,lo:&R,hi:&R,axis:&[Vec<R>;3])->Option<bool> {
    PreparedMotion::new(motion)?.verify(lo,hi,axis)
}

#[test]
fn dyadic_operations_and_bernstein_signs_match_rational_oracle() {
    let mut seed=0x1281356714u64;
    let mut next=|| {seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);let bits=(seed&!(0x7ffu64<<52))|(((seed>>24)%2047)<<52);f64::from_bits(bits)};
    for _ in 0..1000 {
        let a=real(next());let b=real(next());let da=D::from_r(&a).unwrap();let db=D::from_r(&b).unwrap();
        assert_eq!(da.rational(),a);assert_eq!(da.add(&db).rational(),&a+&b);assert_eq!(da.mul(&db).rational(),&a*&b);
        let power=(0..9).map(|_|real(next())).collect::<Vec<_>>();let dp=power.iter().map(|v|D::from_r(v).unwrap()).collect::<Vec<_>>();
        for degree in 0..=8 {for strict in [false,true] {assert_eq!(bernstein_nonnegative(&dp[..=degree],degree,strict),nonnegative(&power[..=degree],degree,strict));}}
    }
    assert!(D::from_r(&R::new(1.into(),3.into())).is_none());
}
#[test]
#[ignore="requires captured exact axis fixture"]
fn captured_dyadic_verifier_matches_rational_and_rejects_larger_radius() {
    let data=std::fs::read_to_string(std::env::var("VOXY_HAIR_EXACT_AXIS_FIXTURE").unwrap()).unwrap();let mut lines=data.lines();assert_eq!(lines.next(),Some("VRC1"));
    let values=lines.next().unwrap().split_whitespace().map(|v|v.parse::<f64>().unwrap()).collect::<Vec<_>>();assert_eq!(values.len(),32);
    let p=|i:usize|[values[i],values[i+1],values[i+2]];
    let capsule=CapsuleMotion {start:[p(0),p(3)],end:[p(6),p(9)],radius:values[30]};let triangle=TriangleMotion {start:[p(12),p(15),p(18)],end:[p(21),p(24),p(27)]};
    let original=ExactMotion::new(capsule,triangle,values[31]);let larger=ExactMotion::new(CapsuleMotion {radius:capsule.radius+1e-9,..capsule},triangle,values[31]);
    let mut rejected=false;let mut count=0;let mut witnesses=Vec::new();
    for line in lines {
        let mut fields=line.split_whitespace().map(|v|v.parse::<R>().unwrap());let lo=fields.next().unwrap();let hi=fields.next().unwrap();
        let axis=std::array::from_fn(|_|(0..3).map(|_|fields.next().unwrap()).collect::<Vec<_>>());assert!(fields.next().is_none());
        assert_eq!(verify(&original,&lo,&hi,&axis),Some(true));
        assert_eq!(verify(&larger,&lo,&hi,&axis),Some(larger.verify_rational(&lo,&hi,&axis)));rejected|=!larger.verify_rational(&lo,&hi,&axis);
        witnesses.push((lo.clone(),hi.clone(),axis.clone()));
        let reversed=axis.map(|a|a.into_iter().map(|v|-v).collect());assert_eq!(verify(&original,&lo,&hi,&reversed),Some(false));count+=1;
    }
    assert_eq!(count,17);assert!(rejected);
    let began=std::time::Instant::now();
    if let Some(leaves)=feature_axis_certificate(&original,1024) {
        let mut end=zero_r();
        for (lo,hi,axis) in &leaves {assert_eq!(*lo,end);assert!(original.verify_rational(lo,hi,axis));end=hi.clone();}
        assert_eq!(end,R::from_integer(1.into()));
        assert!(feature_axis_certificate(&larger,1024).is_none());
        eprintln!("EXACT FEATURE AXES status=certified intervals={} elapsed_s={} scope=test_only_not_full_groom_or_FPS",leaves.len(),began.elapsed().as_secs_f64());
    } else {
        eprintln!("EXACT FEATURE AXES status=unknown elapsed_s={} production_retains_general_axis_discovery=true",began.elapsed().as_secs_f64());
    }

    if std::env::var_os("VOXY_HAIR_FEATURE_AXIS_BENCHMARK").is_some() {
        let mut times=[Vec::new(),Vec::new()];
        for round in 0..6 {for mode in [round%2,1-round%2] {
            let began=std::time::Instant::now();
            let leaves=if mode==0 {super::discovery::build(std::hint::black_box(capsule),std::hint::black_box(triangle),values[31],1024)}
                else {feature_axis_certificate(std::hint::black_box(&original),1024)};
            assert!(std::hint::black_box(leaves).is_some());
            let elapsed=began.elapsed().as_secs_f64()*1000.;if round>0 {times[mode].push(elapsed);}
        }}
        let raw=times.clone();for samples in &mut times {samples.sort_by(f64::total_cmp);}
        eprintln!("FEATURE AXIS PAIRED BENCHMARK general_ms={:?} feature_ms={:?} general_median_ms={} feature_median_ms={} ratio={} scope=one_complete_primitive_certificate_not_frame_or_FPS",
            raw[0],raw[1],times[0][2],times[1][2],times[0][2]/times[1][2]);
    }
    if std::env::var_os("VOXY_HAIR_DYADIC_BENCHMARK").is_some() {
        let mut times=[Vec::new(),Vec::new()];
        for round in 0..6 {
            for mode in [round%2,1-round%2] {
                let began=std::time::Instant::now();
                for _ in 0..4 {for (lo,hi,axis) in &witnesses {
                    let accepted=if mode==0 {original.verify_rational(std::hint::black_box(lo),std::hint::black_box(hi),std::hint::black_box(axis))}
                        else {verify(std::hint::black_box(&original),std::hint::black_box(lo),std::hint::black_box(hi),std::hint::black_box(axis))==Some(true)};
                    assert!(std::hint::black_box(accepted));
                }}
                let elapsed=began.elapsed().as_secs_f64()*1000.;
                if round>0 {times[mode].push(elapsed);}
            }
        }
        let raw=times.clone();for samples in &mut times {samples.sort_by(f64::total_cmp);}
        eprintln!("DYADIC PAIRED BENCHMARK rational_ms={:?} dyadic_ms={:?} rational_median_ms={} dyadic_median_ms={} ratio={} scope=68_interval_verifications_per_run_not_discovery_or_frame_or_FPS",
            raw[0],raw[1],times[0][2],times[1][2],times[0][2]/times[1][2]);
    }

    eprintln!("DYADIC EXACT VERIFIER intervals={count} rational_oracle_equal=true larger_radius_rejected=true scope=single_primitive_no_full_runtime_or_FPS_claim");
}

#[cfg(test)]
struct FeatureAxes<'a> {motion:&'a ExactMotion,lo:&'a R,width:R,next:usize}
#[cfg(test)]
impl FeatureAxes<'_> {
    fn point(&self,endpoint:usize,vertex:usize)->[Vec<R>;3] {
        std::array::from_fn(|k|vec![&self.motion.first[endpoint][vertex][k]+self.lo*&self.motion.slope[endpoint][vertex][k],&self.width*&self.motion.slope[endpoint][vertex][k]])
    }
    fn edge(&self,a:usize,b:usize)->[Vec<R>;3] {
        let a=self.point(0,a);let b=self.point(0,b);
        std::array::from_fn(|k|a[k].iter().zip(&b[k]).map(|(a,b)|a-b).collect())
    }
    fn rod(&self)->[Vec<R>;3] {
        let a=self.point(1,0);let b=self.point(0,0);
        std::array::from_fn(|k|a[k].iter().zip(&b[k]).map(|(a,b)|a-b).collect())
    }
}
#[cfg(test)]
fn polynomial_cross(a:&[Vec<R>;3],b:&[Vec<R>;3])->[Vec<R>;3] {
    std::array::from_fn(|i| {let j=(i+1)%3;let k=(i+2)%3;
        plus(&product(&a[j],&b[k]),&product(&a[k],&b[j]).into_iter().map(|v|-v).collect::<Vec<_>>())})
}
#[cfg(test)]
fn polynomial_dot(a:&[Vec<R>;3],b:&[Vec<R>;3])->Vec<R> {
    (0..3).fold(vec![zero_r()],|s,k|plus(&s,&product(&a[k],&b[k])))
}
#[cfg(test)]
fn perpendicular_axis(point:&[Vec<R>;3],edge:&[Vec<R>;3])->[Vec<R>;3] {
    let norm=polynomial_dot(edge,edge);let projection=polynomial_dot(point,edge);
    // Multiply the perpendicular projection by |edge|^2: exact cubic,
    // no closest-point division. Only the subsequent full proof admits it.
    std::array::from_fn(|k|plus(&product(&point[k],&norm),&product(&edge[k],&projection).into_iter().map(|v|-v).collect::<Vec<_>>()))
}
#[cfg(test)]
impl FeatureAxes<'_> {
    fn at(&self,index:usize)->Option<[Vec<R>;3]> {
        let edges=[(0,1),(1,2),(2,0)];
        Some(match index {
            0=>polynomial_cross(&self.edge(0,1),&self.edge(0,2)),
            1..=3=> {let (a,b)=edges[index-1];polynomial_cross(&self.rod(),&self.edge(a,b))},
            4..=9=> {let i=index-4;let (a,b)=edges[i%3];perpendicular_axis(&self.point(i/3,a),&self.edge(a,b))},
            10..=12=> {let point=self.point(0,index-10).map(|v|v.into_iter().map(|x|-x).collect());perpendicular_axis(&point,&self.rod())},
            13..=18=>self.point((index-13)/3,(index-13)%3),
            _=>return None,
        })
    }
}
#[cfg(test)]
impl Iterator for FeatureAxes<'_> {
    type Item=[Vec<R>;3];
    fn next(&mut self)->Option<Self::Item> {let index=self.next;self.next+=1;self.at(index)}
}
#[cfg(test)]
fn feature_axes<'a>(motion:&'a ExactMotion,lo:&'a R,hi:&R)->FeatureAxes<'a> {
    FeatureAxes {motion,lo,width:hi-lo,next:0}
}
#[cfg(test)]
fn point_motion(motion:&ExactMotion,time:&R)->ExactMotion {
    ExactMotion {radius:motion.radius.clone(),first:std::array::from_fn(|e|std::array::from_fn(|v|std::array::from_fn(|k|&motion.first[e][v][k]+time*&motion.slope[e][v][k]))),slope:std::array::from_fn(|_|std::array::from_fn(|_|std::array::from_fn(|_|zero_r())))}
}
// Static feature selection needs scalar exact inequalities, not a lifted
// degree-eight polynomial with zero higher coefficients. Interval admission
// still uses verify above.
#[cfg(test)]
fn verify_point(point:&ExactMotion,axis:&[Vec<R>;3])->Option<bool> {
    if axis.iter().any(|a|a.is_empty() || a.iter().skip(1).any(|v|v!=&zero_r()))
        || point.slope.iter().flatten().flatten().any(|v|v!=&zero_r()) {return None;}
    let axis=axis.iter().map(|a|D::from_r(&a[0])).collect::<Option<Vec<_>>>()?;
    let radius=D::from_r(&point.radius)?;
    if !radius.positive() {return Some(false);}
    let norm=axis.iter().fold(D::integer(0),|s,a|s.add(&a.mul(a)));
    if !norm.positive() {return Some(false);}
    let threshold=norm.mul(&radius).mul(&radius);
    for endpoint in &point.first {for vertex in endpoint {
        let mut projection=D::integer(0);
        for k in 0..3 {projection=projection.add(&D::from_r(&vertex[k])?.mul(&axis[k]));}
        if !projection.nonnegative() || !projection.mul(&projection).add(&threshold.neg()).nonnegative() {return Some(false);}
    }}
    Some(true)
}
fn basic_point_supported(motion:&PreparedMotion,time:&R)->bool {
    motion.point_at(time).is_some_and(|p|p.basic_supported())
}
pub(super) fn feature_certifies(motion:&ExactMotion,budget:usize)->bool {
    feature_walk::<false>(motion,budget).is_some()
}
#[cfg(test)]
fn feature_axis_certificate(motion:&ExactMotion,budget:usize)->Option<Vec<(R,R,[Vec<R>;3])>> {
    feature_walk::<true>(motion,budget)
}
fn feature_walk<const RETAIN:bool>(motion:&ExactMotion,budget:usize)->Option<Vec<(R,R,[Vec<R>;3])>> {
    feature_walk_impl::<RETAIN,true>(motion,budget)
}
fn feature_walk_impl<const RETAIN:bool,const REUSE:bool>(motion:&ExactMotion,budget:usize)->Option<Vec<(R,R,[Vec<R>;3])>> {
    // Immutable operands are converted once per complete motion query.
    let prepared=PreparedMotion::new(motion)?;
    let zero=zero_r();let one=R::from_integer(1.into());
    // Cache only exact point predicates at shared dyadic boundaries, scoped
    // to this immutable motion. Unknown endpoints remain lazy so a successful
    // whole-interval axis incurs no additional point queries.
    let mut stack=vec![(zero,one,0,None,None)];let mut visited=0;let mut leaves=Vec::new();
    while let Some((lo,hi,depth,mut lo_supported,mut hi_supported))=stack.pop() {
        visited+=1;if visited>budget || depth>48 {return None;}
        let moving=prepared.interval(&lo,&hi)?;
        let mut accepted=None;
        for index in 0..4 {
            let axis=moving.axis(index);
            if moving.verify(&axis) {accepted=Some(axis);break;}
            let negative=axis.map(|v|v.into_iter().map(|x|x.neg()).collect());
            if moving.verify(&negative) {accepted=Some(negative);break;}
        }
        if let Some(axis)=accepted {if RETAIN {leaves.push((lo,hi,rational_axis(axis)));}continue;}
        let mid=(&lo+&hi)/R::from_integer(2.into());
        let point=prepared.point_at(&mid)?;
        // Refine the cheap family when it already separates the midpoint.
        // Expand to cubic/vertex axes only where local feature geometry needs
        // them; every accepted interval still carries a complete exact proof.
        let basic_supported=point.basic_supported();
        let supported=|time:&R,cached:&mut Option<bool>| {
            if REUSE {if let Some(value)=*cached {return value;}}
            let value=basic_point_supported(&prepared,time);*cached=Some(value);value
        };
        if basic_supported && supported(&lo,&mut lo_supported) && supported(&hi,&mut hi_supported) {
            stack.push((mid.clone(),hi,depth+1,Some(basic_supported),hi_supported));
            stack.push((lo,mid,depth+1,lo_supported,Some(basic_supported)));continue;
        }
        let mut point_supported=false;
        for index in 4..19 {
            let axis=point.axis(index);
            let positive=point.verify(&axis);
            let negative=!positive && point.verify(&axis.map(|x|x.neg()));
            if !positive && !negative {continue;}
            point_supported=true;
            let axis=moving.axis(index);
            let axis=if positive {axis} else {axis.map(|v|v.into_iter().map(|x|x.neg()).collect())};
            if moving.verify(&axis) {accepted=Some(axis);break;}
        }
        // Midpoint filtering rejects axes only. Acceptance still requires all
        // exact whole-interval inequalities; an unsupported family falls back.
        if let Some(axis)=accepted {if RETAIN {leaves.push((lo,hi,rational_axis(axis)));}continue;}
        if !point_supported {return None;}
        stack.push((mid.clone(),hi,depth+1,Some(basic_supported),hi_supported));
        stack.push((lo,mid,depth+1,lo_supported,Some(basic_supported)));
    }
    Some(leaves)
}

#[test]
fn feature_axis_certificate_rejects_crossing_and_unknown_budget() {
    let triangle=TriangleMotion {start:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]],end:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]]};
    let capsule=CapsuleMotion {start:[[-0.1,0.,1.],[0.1,0.,1.]],end:[[-0.1,0.,2.],[0.1,0.,2.]],radius:0.1};
    let motion=ExactMotion::new(capsule,triangle,1e-10);
    let leaves=feature_axis_certificate(&motion,64).unwrap();
    for (lo,hi,axis) in &leaves {assert!(motion.verify_rational(lo,hi,axis));}
    assert!(feature_walk::<false>(&motion,64).unwrap().is_empty());
    assert!(!feature_certifies(&motion,0));
    let crossing=ExactMotion::new(CapsuleMotion {end:[[-0.1,0.,-1.],[0.1,0.,-1.]],..capsule},triangle,1e-10);
    assert!(!feature_certifies(&crossing,64));
    let degenerate=TriangleMotion {start:[[0.;3];3],end:[[0.;3];3]};
    let point=ExactMotion::new(capsule,degenerate,1e-10);
    let leaves=feature_axis_certificate(&point,64).expect("vertex/segment axes must retain a collapsed triangle as a point collider");
    for (lo,hi,axis) in &leaves {assert!(point.verify_rational(lo,hi,axis));}
}

#[test]
fn cubic_feature_axes_cover_coplanar_endpoint_and_vertex_contacts() {
    let triangle=TriangleMotion {start:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]],end:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]]};
    for points in [[[0.,-2.2,0.],[0.3,-2.4,0.]],[[-0.5,2.2,0.],[0.5,2.2,0.]]] {for moving in [false,true] {
        let triangle=if moving {TriangleMotion {end:[[-2.01,-1.99,0.01],[2.02,-2.01,-0.01],[0.01,2.02,0.02]],..triangle}} else {triangle};
        let capsule=CapsuleMotion {start:points,end:if moving {points.map(|p|[p[0]+0.05,p[1],p[2]+0.01])} else {points},radius:0.1};
        let motion=ExactMotion::new(capsule,triangle,1e-10);
        let zero=zero_r();let one=R::from_integer(1.into());
        for axis in feature_axes(&motion,&zero,&one).take(4) {
            assert_ne!(verify(&motion,&zero,&one,&axis),Some(true));
            assert_ne!(verify(&motion,&zero,&one,&axis.map(|v|v.into_iter().map(|x|-x).collect())),Some(true));
        }
        let leaves=feature_axis_certificate(&motion,64).unwrap();
        assert!(leaves.iter().any(|(_,_,axis)|axis[0].len()==4));
        for (lo,hi,axis) in &leaves {assert!(motion.verify_rational(lo,hi,axis));}
        let penetration=ExactMotion::new(CapsuleMotion {radius:0.3,..capsule},triangle,1e-10);
        assert!(!feature_certifies(&penetration,64));
    }}
}

#[test]
fn exact_static_feature_selection_matches_rational_interval_oracle() {
    let triangle=TriangleMotion {start:[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.]],end:[[-2.01,-1.99,0.01],[2.02,-2.01,-0.01],[0.01,2.02,0.02]]};
    let capsule=CapsuleMotion {start:[[0.,-2.2,0.],[0.3,-2.4,0.]],end:[[0.05,-2.2,0.01],[0.35,-2.4,0.01]],radius:0.1};
    let motion=ExactMotion::new(capsule,triangle,1e-10);
    let zero=zero_r();let one=R::from_integer(1.into());
    for time in [zero.clone(),R::new(1.into(),2.into()),one.clone()] {
        let point=point_motion(&motion,&time);
        for axis in feature_axes(&point,&zero,&one) {for axis in [axis.clone(),axis.map(|v|v.into_iter().map(|x|-x).collect())] {
            assert_eq!(verify_point(&point,&axis),Some(point.verify_rational(&zero,&one,&axis)));
        }}
    }
    let moving_axis=feature_axes(&motion,&zero,&one).next().unwrap();
    assert_eq!(verify_point(&motion,&moving_axis),None);
}

#[test]
#[ignore = "requires original VRC1 geometry; exact shared-boundary predicate comparison"]
fn captured_exact_shared_boundary_predicates_preserve_interval_proofs() {
    let data=std::fs::read_to_string(std::env::var("VOXY_HAIR_EXACT_AXIS_FIXTURE").unwrap()).unwrap();
    let values=data.lines().nth(1).unwrap().split_whitespace().map(|v|v.parse::<f64>().unwrap()).collect::<Vec<_>>();
    assert_eq!(values.len(),32);let p=|i:usize|[values[i],values[i+1],values[i+2]];
    let capsule=CapsuleMotion {start:[p(0),p(3)],end:[p(6),p(9)],radius:values[30]};
    let triangle=TriangleMotion {start:[p(12),p(15),p(18)],end:[p(21),p(24),p(27)]};
    for radius in [capsule.radius,capsule.radius+1e-9] {
        let motion=ExactMotion::new(CapsuleMotion {radius,..capsule},triangle,values[31]);
        for budget in [1,16,1024] {
            let original=feature_walk_impl::<true,false>(&motion,budget);
            let reused=feature_walk_impl::<true,true>(&motion,budget);
            assert_eq!(original,reused,"same budget must produce identical exact leaves or unknown");
            if let Some(leaves)=reused {for (lo,hi,axis) in leaves {assert!(motion.verify_rational(&lo,&hi,&axis));}}
        }
        let mut samples=[Vec::new(),Vec::new()];
        for round in 0..4 {for mode in [round%2,1-round%2] {
            let began=std::time::Instant::now();
            let outcome=if mode==0 {feature_walk_impl::<false,false>(&motion,1024)} else {feature_walk_impl::<false,true>(&motion,1024)};
            std::hint::black_box(outcome);
            if round>0 {samples[mode].push(began.elapsed().as_secs_f64()*1000.);}
        }}
        let raw=samples.clone();for times in &mut samples {times.sort_by(f64::total_cmp);}
        eprintln!("EXACT BOUNDARY REUSE radius={radius} reference_ms={:?} reused_ms={:?} reference_median_ms={} reused_median_ms={} ratio={} scope=feature_certificate_not_frame_or_FPS",raw[0],raw[1],samples[0][1],samples[1][1],samples[0][1]/samples[1][1]);
    }
}

#[test]
fn scalar_dyadic_features_match_rational_geometry_and_decisions() {
    let mut seed=176128u64;
    let mut next=|| {seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);((seed>>32)%129) as f64/16.-4.};
    let zero=zero_r();let one=R::from_integer(1.into());
    for case in 0..32 {
        let mut triangle=TriangleMotion {start:std::array::from_fn(|_|std::array::from_fn(|_|next())),end:std::array::from_fn(|_|std::array::from_fn(|_|next()))};
        if case%8==0 {triangle.start=[[0.;3];3];triangle.end=[[0.;3];3];}
        let capsule=CapsuleMotion {start:std::array::from_fn(|_|std::array::from_fn(|_|next())),end:std::array::from_fn(|_|std::array::from_fn(|_|next())),radius:0.125};
        let motion=ExactMotion::new(capsule,triangle,1e-10);
        let prepared=PreparedMotion::new(&motion).unwrap();
        for t in 0..4 {
            let lo=R::new(t.into(),4.into());let hi=R::new((t+1).into(),4.into());
            let interval=prepared.interval(&lo,&hi).unwrap();
            for (i,axis) in feature_axes(&motion,&lo,&hi).enumerate() {
                let native=interval.axis(i);
                assert_eq!(rational_axis(native.clone()),axis);
                assert_eq!(interval.verify(&native),motion.verify_rational(&lo,&hi,&axis));
                let negative=native.map(|v|v.into_iter().map(|x|x.neg()).collect());
                assert_eq!(interval.verify(&negative),motion.verify_rational(&lo,&hi,&rational_axis(negative)));
            }
        }
        for t in 0..=4 {
            let time=R::new(t.into(),4.into());let point=point_motion(&motion,&time);let scalar=prepared.point_at(&time).unwrap();
            for (i,axis) in feature_axes(&point,&zero,&one).enumerate() {
                let dyadic=scalar.axis(i);
                for k in 0..3 {assert_eq!(dyadic[k].rational(),axis[k][0]);assert!(axis[k].iter().skip(1).all(|x|x==&zero));}
                assert_eq!(scalar.verify(&dyadic),verify_point(&point,&axis).unwrap());
                assert_eq!(scalar.verify(&dyadic),point.verify_rational(&zero,&one,&axis));
                let negative=axis.map(|v|v.into_iter().map(|x|-x).collect());
                assert_eq!(scalar.verify(&dyadic.map(|x|x.neg())),point.verify_rational(&zero,&one,&negative));
            }
        }
    }
}
