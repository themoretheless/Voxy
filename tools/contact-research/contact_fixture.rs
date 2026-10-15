//! Original diagnostic operator reader, shared by qualification tools/tests only.
use std::io::{Read,Seek};
pub struct OriginalContactFixture {
    pub requests:Vec<physics::hair::HairResponseSystem>,pub bounds:Vec<f64>,pub tolerance:f64,
    pub rows:usize,pub width:usize,
}
pub fn read(path:&std::path::Path)->OriginalContactFixture {
    let file=std::fs::File::open(path).unwrap();let length=file.metadata().unwrap().len();let mut input=std::io::BufReader::new(file);
    fn integer(input:&mut impl Read)->usize{let mut b=[0;4];input.read_exact(&mut b).unwrap();u32::from_le_bytes(b) as usize}
    fn scalar(input:&mut impl Read)->f64{let mut b=[0;8];input.read_exact(&mut b).unwrap();let v=f64::from_le_bytes(b);assert!(v.is_finite());v}
    let mut magic=[0;4];input.read_exact(&mut magic).unwrap();assert_eq!(&magic,b"VQC1");
    let rows=integer(&mut input);let width=integer(&mut input);let systems=integer(&mut input);integer(&mut input);let tolerance=scalar(&mut input);
    assert!(rows>0&&rows<=1024&&width>0&&width<=65536&&systems>0&&systems<=512&&tolerance>0.);
    assert_eq!(length,28+16*rows as u64+16*systems as u64+16*rows as u64*width as u64+80*width as u64);
    let bounds=(0..rows).map(|_|scalar(&mut input)).collect::<Vec<_>>();
    input.seek(std::io::SeekFrom::Current((8*rows+8*rows*width) as i64)).unwrap();
    let mut requests=Vec::new();let mut total=0;
    for _ in 0..systems {
        let n=integer(&mut input);let band=integer(&mut input);let lo=integer(&mut input);let hi=integer(&mut input);
        assert!(n>0&&n<=width&&band==9&&lo<=hi&&hi<=n);total+=n;
        requests.push(physics::hair::HairResponseSystem{system:physics::hair::HairLinearSystem{band_width:band,matrix:(0..n*band).map(|_|scalar(&mut input)).collect(),rhs:(0..n).map(|_|scalar(&mut input)).collect(),active:lo..hi},loads:(0..rows).map(|_|(0..n).map(|_|scalar(&mut input)).collect()).collect()});
    }
    assert_eq!(total,width);assert_eq!(input.stream_position().unwrap(),length);
    OriginalContactFixture{requests,bounds,tolerance,rows,width}
}
