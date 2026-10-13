#![allow(dead_code)]
mod math;use math::{V,add,dot,finite};
#[derive(Clone,Copy)]struct Entry{rod:usize,point:usize,gradient:V,mobility:f64}
struct Constraint{entries:[Entry;4],bound:f64,diagonal:f64,multiplier:f64,response:Vec<()>}
fn add_constraint(
    constraints: &mut Vec<Constraint>,
    mut entries: [Entry; 4],
    bound: f64,
) -> Result<Option<usize>, &'static str> {
    // Pinned-root entries contribute no increment. Canonical station order
    // removes aliases of a shared endpoint from adjacent capsule pairs.
    for entry in &mut entries {if entry.point==0 {entry.gradient=[0.;3];}}
    entries.sort_by_key(|entry|(entry.rod,entry.point));
    let zero=Entry {rod:0,point:0,gradient:[0.;3],mobility:0.};
    let mut packed=[zero;4];let mut count=0;
    for entry in entries {
        if entry.gradient.iter().all(|&v|v==0.) {continue;}
        if count>0 && packed[count-1].rod==entry.rod && packed[count-1].point==entry.point {
            packed[count-1].gradient=add(packed[count-1].gradient,entry.gradient);
        } else {packed[count]=entry;count+=1;}
    }
    let entries=packed;
    if entries.iter().any(|entry| !finite(entry.gradient)
        || !entry.mobility.is_finite() || entry.mobility < 0.) {
        return Err("invalid shared contact velocity constraint");
    }
    let mut diagonal = entries
        .iter()
        .map(|entry| entry.mobility * dot(entry.gradient, entry.gradient))
        .sum::<f64>();
    // Avoid premature overflow/underflow in g*g when sqrt(mobility)*g
    // still has a representable weighted norm. Preserve ordinary arithmetic.
    if diagonal==0. || !diagonal.is_finite() {
        let norm=entries.iter().flat_map(|e|e.gradient.map(|g|e.mobility.sqrt()*g))
            .fold(0f64,|norm,value|norm.hypot(value));
        diagonal=norm*norm;
    }
    if !diagonal.is_finite() || !bound.is_finite() {
        return Err("invalid shared contact velocity constraint");
    }
    if diagonal == 0. && entries.iter().any(|entry|entry.mobility>0. && entry.gradient.iter().any(|&v|v!=0.)) {
        return Err("contact constraint diagonal underflow");
    }
    if diagonal > 0. {
        // Only identical physical Jacobians are aliases. A small angular
        // difference can produce a residual larger than solve tolerance when
        // multiplied by the free state; never discard that original row.
        if let Some((index,existing))=constraints.iter_mut().enumerate().find(|(_,constraint)| {
            constraint.entries.iter().zip(entries).all(|(a,b)|a.rod==b.rod && a.point==b.point
                && a.mobility==b.mobility && a.gradient==b.gradient)
        }) {
            existing.bound=existing.bound.max(bound);
            return Ok(Some(index));
        }
        constraints.push(Constraint {
            entries,
            bound,
            diagonal,
            multiplier: 0.,
            response: Vec::new(),
        });
        return Ok(Some(constraints.len()-1));
    }
    Ok(None)
}
fn main(){for count in [1000,1600,2400]{
 let zero=Entry{rod:0,point:0,gradient:[0.;3],mobility:0.};
 let started=std::time::Instant::now();let repeats=30;
 for _ in 0..repeats {let mut rows=Vec::new();
  for id in 0..count {let entry=Entry{rod:id%469,point:id/469+1,gradient:[1.,0.,0.],mobility:1.};
   assert_eq!(add_constraint(&mut rows,[entry,zero,zero,zero],0.).unwrap(),Some(id));}
  std::hint::black_box(rows);
 }
 println!("rows={count} repeats={repeats} assembly_ms={}",started.elapsed().as_secs_f64()*1000./repeats as f64);
}}
