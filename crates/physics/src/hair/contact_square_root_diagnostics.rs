//! Opt-in exact binary capture of an original failed joint response admission.
use super::HairResponseSystem;
use std::io::Write;

/// Unsolved input: no candidate response or reaction is implied by this file.
pub(super) fn export_input(requests:&[HairResponseSystem],columns:&[Vec<f64>],
    bounds:&[f64],effective:&[f64],tolerance:f64,refinement:usize) {
    let Some(path)=std::env::var_os("VOXY_HAIR_QR_INPUT_EXPORT") else {return;};
    let write=||->std::io::Result<()> {
        let mut out=std::io::BufWriter::new(std::fs::File::create(&path)?);
        out.write_all(b"VQC1")?;
        let width=requests.iter().map(|r|r.system.rhs.len()).sum::<usize>();
        for n in [bounds.len(),width,requests.len(),refinement] {out.write_all(&(n as u32).to_le_bytes())?;}
        out.write_all(&tolerance.to_le_bytes())?;
        let values=|out:&mut std::io::BufWriter<std::fs::File>,row:&[f64]|->std::io::Result<()> {
            for x in row {out.write_all(&x.to_le_bytes())?;} Ok(())
        };
        values(&mut out,bounds)?;values(&mut out,effective)?;
        for column in columns {values(&mut out,column)?;}
        for request in requests {
            let s=&request.system;
            for n in [s.rhs.len(),s.band_width,s.active.start,s.active.end] {out.write_all(&(n as u32).to_le_bytes())?;}
            values(&mut out,&s.matrix)?;values(&mut out,&s.rhs)?;
            for load in &request.loads {values(&mut out,load)?;}
        }
        out.flush()
    };
    match write() {
        Ok(())=>eprintln!("HAIR QR INPUT EXPORT {:?} systems={} rows={} refinement={refinement}",path,requests.len(),bounds.len()),
        Err(error)=>eprintln!("HAIR QR INPUT EXPORT ERROR {error}"),
    }
}

pub(super) fn export(
    requests: &[HairResponseSystem], columns: &[Vec<f64>], bounds: &[f64],
    coordinates: &[f64], reactions: &[f64], responses: &[Vec<f64>],
    tolerance: f64, failed_row: usize,
) {
    let Some(path)=std::env::var_os("VOXY_HAIR_QR_FAILURE_EXPORT") else {return;};
    let write=|| -> std::io::Result<()> {
        let mut out=std::io::BufWriter::new(std::fs::File::create(&path)?);
        out.write_all(b"VQI1")?;
        for value in [bounds.len(),coordinates.len(),requests.len(),failed_row] {
            out.write_all(&(value as u32).to_le_bytes())?;
        }
        out.write_all(&tolerance.to_le_bytes())?;
        let values=|out:&mut std::io::BufWriter<std::fs::File>, row:&[f64]| -> std::io::Result<()> {
            for value in row {out.write_all(&value.to_le_bytes())?;}
            Ok(())
        };
        values(&mut out,bounds)?; values(&mut out,reactions)?;
        for column in columns {values(&mut out,column)?;}
        values(&mut out,coordinates)?;
        for (request,response) in requests.iter().zip(responses) {
            let system=&request.system;
            for value in [system.rhs.len(),system.band_width,system.active.start,system.active.end] {
                out.write_all(&(value as u32).to_le_bytes())?;
            }
            values(&mut out,&system.matrix)?; values(&mut out,&system.rhs)?;
            for load in &request.loads {values(&mut out,load)?;}
            values(&mut out,response)?;
        }
        out.flush()
    };
    match write() {
        Ok(())=>eprintln!("HAIR QR FAILURE EXPORT {:?} systems={} rows={} coordinates={} failed_row={failed_row}",path,requests.len(),bounds.len(),coordinates.len()),
        Err(error)=>eprintln!("HAIR QR FAILURE EXPORT ERROR {error}"),
    }
}
