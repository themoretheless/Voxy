//! Exact dyadic cycle selection and enclosed local phase for stored f32 durations.
use crate::{AnimationError,Playback};
use std::cmp::Ordering;
#[derive(Clone,Copy,Debug)]
pub struct RootCyclePhase {
    cycle:u64,
    phase:f64,
    bounds:[f64;2],
}
impl RootCyclePhase {
    pub fn cycle(self)->u64 {self.cycle}
    pub fn phase(self)->f64 {self.phase}
    pub fn exact_phase_bounds(self)->[f64;2] {self.bounds}
}
fn dyadic(value:f64)->(u128,i32) {
    let bits=value.to_bits();let exponent=((bits>>52)&2047) as i32;
    let fraction=u128::from(bits&((1_u64<<52)-1));
    if exponent==0 {(fraction,-1074)} else {(fraction|(1_u128<<52),exponent-1075)}
}
fn compare(a:(u128,i32),b:(u128,i32))->Ordering {
    if a.0==0 || b.0==0 {return a.0.cmp(&b.0);}
    let ah=128-a.0.leading_zeros() as i32+a.1;
    let bh=128-b.0.leading_zeros() as i32+b.1;
    if ah!=bh {return ah.cmp(&bh);}
    let base=a.1.min(b.1);
    (a.0<<((a.1-base) as u32)).cmp(&(b.0<<((b.1-base) as u32)))
}
/// Selects the exact floor(time/duration) and encloses its exact remainder.
/// One integer-to-f64 rounding publishes the phase; loop phases stay half-open.
pub fn enclose_root_cycle_phase(time:f64,duration:f32,playback:Playback)
    ->Result<RootCyclePhase,AnimationError> {
    if !time.is_finite() || time<0. || !duration.is_finite() || duration<=0. {
        return Err(AnimationError::InvalidSampleTime);
    }
    let duration=f64::from(duration);
    if playback==Playback::Clamp || time<duration {
        let phase=time.min(duration);
        return Ok(RootCyclePhase {cycle:0,phase,bounds:[phase,phase]});
    }
    let estimate=(time/duration).floor();
    if !estimate.is_finite() || estimate>9007199254740994. {
        return Err(AnimationError::RootRigidBudget);
    }
    let t=dyadic(time);let d=dyadic(duration);
    let mut cycle=estimate as u64;
    let mut selected=false;
    for _ in 0..3 {
        let product=(d.0*u128::from(cycle),d.1);
        if compare(t,product)==Ordering::Less {
            cycle=cycle.checked_sub(1).ok_or(AnimationError::RootRigidBudget)?;
        } else if compare(t,(d.0*u128::from(cycle+1),d.1))!=Ordering::Less {
            cycle+=1;
        } else {selected=true;break;}
    }
    if !selected || cycle>9007199254740991 {return Err(AnimationError::RootRigidBudget);}
    let base=t.1.min(d.1);
    let ts=(t.1-base) as u32;let ds=(d.1-base) as u32;
    let product=d.0*u128::from(cycle);
    if ts>=128 || ds>=128 || t.0.leading_zeros()<ts || product.leading_zeros()<ds {
        return Err(AnimationError::RootRigidBudget);
    }
    let remainder=(t.0<<ts)-(product<<ds);
    // A nonzero cycle of a stored f32 duration keeps this exponent normal.
    if !(-1022..=1023).contains(&base) {return Err(AnimationError::RootRigidBudget);}
    let factor=f64::from_bits(((base+1023) as u64)<<52);
    let mut phase=(remainder as f64)*factor;
    if phase>=duration {phase=duration.next_down();}
    let ordering=compare((remainder,base),dyadic(phase));
    let bounds=match ordering {
        Ordering::Equal=>[phase,phase],
        Ordering::Less=>[phase.next_down().max(0.),phase],
        Ordering::Greater=>[phase,phase.next_up().min(duration)],
    };
    Ok(RootCyclePhase {cycle,phase,bounds})
}
/// Local endpoints for a partition containing no interior loop seam.
pub(crate) fn root_segment_phases(start:f64,end:f64,duration:f32,playback:Playback)
    ->Result<[f64;2],AnimationError> {
    if end<start {return Err(AnimationError::InvalidSampleTime);}
    let a=enclose_root_cycle_phase(start,duration,playback)?;
    let b=enclose_root_cycle_phase(end,duration,playback)?;
    let last=if b.cycle()==a.cycle() {b.phase()}
        else if b.cycle()==a.cycle()+1 && b.phase()==0. {f64::from(duration)}
        else {return Err(AnimationError::RootRigidBudget);};
    if last<a.phase() {return Err(AnimationError::RootRigidBudget);}
    Ok([a.phase(),last])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_floor_corrects_rounded_division_and_encloses_remainder() {
        let time=109951164416.09999;
        let duration=0.1_f32;
        assert_eq!((time/f64::from(duration)).floor() as u64,1099511627777);
        let proof=enclose_root_cycle_phase(time,duration,Playback::Loop).unwrap();
        assert_eq!(proof.cycle(),1099511627776);
        assert!(proof.phase()>=0. && proof.phase()<f64::from(duration));
        assert_eq!(proof.exact_phase_bounds(),[proof.phase();2]);
        println!("ROOT_CLOCK_PROOF {:?}",(time,f64::from(duration),proof.cycle(),proof.phase(),proof.exact_phase_bounds()));
    }
    #[test]
    fn segment_endpoints_use_exact_remainders_and_keep_seam_left_limit() {
        let duration=0.1_f32;
        let start=109951164416.09999;
        assert_eq!(root_segment_phases(start,start,duration,Playback::Loop).unwrap(),
            [0.0999908447265625;2]);
        let d=f64::from(duration);
        assert_eq!(root_segment_phases(d.next_down(),d,duration,Playback::Loop).unwrap(),
            [d.next_down(),d]);
        assert!(root_segment_phases(0.,d.next_up(),duration,Playback::Loop).is_err());
        assert_eq!(root_segment_phases(2.,3.,duration,Playback::Clamp).unwrap(),[d,d]);
    }
    #[test]
    fn seam_neighbors_clamp_and_invalid_clocks_are_explicit() {
        for time in [0.,1_f64.next_down(),1.,1_f64.next_up(),4.5] {
            let proof=enclose_root_cycle_phase(time,1.,Playback::Loop).unwrap();
            assert_eq!(proof.cycle(),time.floor() as u64);
            assert_eq!(proof.phase(),time-time.floor());
        }
        let proof=enclose_root_cycle_phase(4.5,1.,Playback::Clamp).unwrap();
        assert_eq!(proof.cycle(),0);assert_eq!(proof.phase(),1.);
        for time in [f64::NAN,-1.,f64::INFINITY,1e100] {
            assert!(enclose_root_cycle_phase(time,1.,Playback::Loop).is_err());
        }
    }
}
