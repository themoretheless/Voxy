//! Export real expanded native coefficients and their existing GPU input words.
//! The companion verifier uses exact rational arithmetic, not rounded f64 sums.
use std::io::{Read, Write};
fn integer(input: &mut impl Read) -> usize {
    let mut bytes = [0; 4]; input.read_exact(&mut bytes).unwrap(); u32::from_le_bytes(bytes) as usize
}
fn scalar(input: &mut impl Read) -> f64 {
    let mut bytes = [0; 8]; input.read_exact(&mut bytes).unwrap(); f64::from_le_bytes(bytes)
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "fixture.vqc output.bin");
    let bytes = std::fs::read(&args[1]).unwrap();
    let mut input = std::io::Cursor::new(bytes);
    let mut magic = [0; 4]; input.read_exact(&mut magic).unwrap(); assert_eq!(&magic, b"VQC1");
    let rows = integer(&mut input); let width = integer(&mut input);
    let systems = integer(&mut input); let _refinement = integer(&mut input);
    assert!(rows > 0 && rows <= 1024 && width <= 65536 && systems <= 512);
    let tolerance = scalar(&mut input);
    let bounds = (0..rows).map(|_| scalar(&mut input)).collect::<Vec<_>>();
    for _ in 0..rows + rows * width { scalar(&mut input); }
    let mut columns = vec![Vec::new(); rows];
    for _ in 0..systems {
        let n = integer(&mut input); let band = integer(&mut input);
        let lo = integer(&mut input); let hi = integer(&mut input);
        assert!(n <= 65536 && band == 9 && lo <= hi && hi <= n);
        let request = physics::hair::HairResponseSystem {
            system: physics::hair::HairLinearSystem {
                band_width: band,
                matrix: (0..n*band).map(|_| scalar(&mut input)).collect(),
                rhs: (0..n).map(|_| scalar(&mut input)).collect(), active: lo..hi,
            },
            loads: (0..rows).map(|_| (0..n).map(|_| scalar(&mut input)).collect()).collect(),
        };
        for (column, local) in columns.iter_mut().zip(request.whiten_loads_expanded().unwrap()) {
            column.extend(local.into_iter().map(|value| value.parts()));
        }
    }
    assert_eq!(input.position() as usize, input.get_ref().len());
    assert!(columns.iter().all(|column| column.len() == width));
    let packed = voxy_render::ExperimentalTripleContactInput::new_expanded(&columns, &bounds, tolerance).unwrap();
    let mut out = std::io::BufWriter::new(std::fs::OpenOptions::new().write(true).create_new(true).open(&args[2]).unwrap());
    out.write_all(b"VPA1").unwrap();
    out.write_all(&(rows as u32).to_le_bytes()).unwrap(); out.write_all(&(width as u32).to_le_bytes()).unwrap();
    out.write_all(&packed.column_scale.to_le_bytes()).unwrap();
    for (parts, words) in columns.iter().flatten().zip(&packed.columns) {
        for value in parts { out.write_all(&value.to_le_bytes()).unwrap(); }
        for word in words { out.write_all(&word.to_le_bytes()).unwrap(); }
    }
    out.flush().unwrap();
    println!("rows={rows} width={width} systems={systems} records={} scale_bits={}", rows*width, packed.column_scale.to_bits());
}
