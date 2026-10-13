from pathlib import Path
s=Path('crates/physics/src/hair/contact_square_root_qr.rs').read_text()
start=s[s.index('pub(super) fn minimum_norm'):s.index('// One immutable coordinate map')]
helper=s[s.index('fn accurate_column_combination('):s.index('#[cfg(test)]\nmod tests')]
old='''                    for (value, &axis) in v.iter_mut().zip(basis) {
                        *value = (-projection).mul_add(axis, *value);
                    }
                    if self.sparse_basis && v.iter().any(|value|!value.is_finite()) {return None;}
'''
a=start.index('                    if self.sparse_basis {\n                        // Validate while writing:')
b=start.index('\n                }\n            }\n            let scale',a)
baseline=start[:a]+old.rstrip()+start[b:]
run='''pub fn run(c:&[Vec<f64>],b:&[f64],tol:f64)->Vec<Option<Vec<u64>>> { let mut q=EqualityQr::new(c);q.sparse_basis=true; (1..=c.len().min(20)).map(|n| q.solve(&(0..n).collect::<Vec<_>>(),&b[..n],tol).map(|(x,y)|x.into_iter().chain(y).map(f64::to_bits).collect())).collect() }'''
main=r'''fn main(){let bytes=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();assert_eq!(&bytes[..4],b"VQC1");let n=u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;let m=u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;let mut k=20;let mut f=||{let x=f64::from_le_bytes(bytes[k..k+8].try_into().unwrap());k+=8;x};let tol=f();for _ in 0..n{f();}let b:Vec<_>=(0..n).map(|_|f()).collect();let c:Vec<Vec<_>>=(0..n).map(|_|(0..m).map(|_|f()).collect()).collect();let old=baseline::run(&c,&b,tol);assert_eq!(old,candidate::run(&c,&b,tol));println!("bitwise matched cases={} successful={} rows={n} width={m}",old.len(),old.iter().filter(|x|x.is_some()).count());for repeat in 0..7{let mut times=[0.;2];for mode in [repeat%2,1-repeat%2]{let t=std::time::Instant::now();for _ in 0..20{let result=if mode==0{baseline::run(&c,&b,tol)}else{candidate::run(&c,&b,tol)};assert_eq!(result,old);std::hint::black_box(result);}times[mode]=t.elapsed().as_secs_f64()*1000.;}println!("paired repeat={repeat} baseline_ms={} candidate_ms={}",times[0],times[1]);}}'''
Path('/tmp/voxy-qr-paired.rs').write_text('#![allow(dead_code)]\nmod baseline {\n'+baseline+helper+run+'\n}\nmod candidate {\n'+start+helper+run+'\n}\n'+main)
