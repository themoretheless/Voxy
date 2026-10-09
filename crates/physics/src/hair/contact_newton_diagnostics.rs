//! Opt-in original Newton admission capture, before reaction/pose publication.
use super::*;
use std::io::Write;
pub(super) fn export(rows:&[Constraint],original:&PositionIncrement,candidate:&PositionIncrement,
    reactions:&[f64],tolerance:f64) {
    let Some(path)=std::env::var_os("VOXY_HAIR_NEWTON_FAILURE_EXPORT") else {return;};
    let failed=rows.iter().zip(reactions).position(|(c,&r)| {
        let gap=c.speed(candidate)-c.bound;
        !gap.is_finite() || if r>0. {gap.abs()>tolerance} else {gap< -tolerance}
    }).unwrap_or(usize::MAX);
    let write=||->std::io::Result<()> {
        let mut out=std::io::BufWriter::new(std::fs::File::create(&path)?);
        out.write_all(b"VJN1")?;
        for x in [original.linear.len(),rows.len(),failed] {out.write_all(&(x as u32).to_le_bytes())?;}
        out.write_all(&tolerance.to_le_bytes())?;
        for r in 0..original.linear.len() {
            for n in [original.linear[r].len(),original.angular[r].len()] {out.write_all(&(n as u32).to_le_bytes())?;}
            for points in [&original.linear[r],&original.angular[r],&candidate.linear[r],&candidate.angular[r]] {
                for p in points {for x in p {out.write_all(&x.to_le_bytes())?;}}
            }
        }
        for (c,&reaction) in rows.iter().zip(reactions) {
            out.write_all(&c.bound.to_le_bytes())?;out.write_all(&reaction.to_le_bytes())?;
            for e in c.entries {
                for x in [e.rod,e.point] {out.write_all(&(x as u32).to_le_bytes())?;}
                for x in e.gradient {out.write_all(&x.to_le_bytes())?;}
                out.write_all(&e.mobility.to_le_bytes())?;
            }
        }
        out.flush()
    };
    match write() {
        Ok(())=> {
            eprintln!("HAIR NEWTON FAILURE EXPORT {:?} rods={} rows={} failed_row={failed}",path,original.linear.len(),rows.len());
            if let Some(c)=rows.get(failed) {
                eprintln!("HAIR NEWTON FAILURE DETAIL row={failed} original={:e} candidate={:e} bound={:e} gap={:e} reaction={:e} tolerance={tolerance:e}",c.speed(original),c.speed(candidate),c.bound,c.speed(candidate)-c.bound,reactions[failed]);
            }
        }
        Err(error)=>eprintln!("HAIR NEWTON FAILURE EXPORT ERROR {error}"),
    }
}
