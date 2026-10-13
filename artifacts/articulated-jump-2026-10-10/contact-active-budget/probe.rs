#![allow(dead_code)]
enum HairContactEqualityProposal { Solution(Vec<f64>,Vec<f64>), ReleaseDirection(Vec<f64>) }
struct HairContactCoordinateRequest<'a> { columns:&'a [Vec<f64>], bounds:&'a [f64], tolerance:f64, seeds:&'a [f64] }
struct HairContactEqualityRequest<'a> { operator_index:usize, columns:&'a [Vec<f64>], bounds:&'a [f64], tolerance:f64 }
#[path="baseline.rs"] mod qr;
fn main() {
 let n=std::env::args().nth(1).map(|s|s.parse::<usize>().unwrap()).unwrap_or(513);let mut columns=vec![vec![0.;n];n];for i in 0..n{columns[i][i]=1.;}
 let result=qr::unilateral(&columns,&vec![1.;n],1e-14);
 println!("rows={n} admitted={}",result.is_some());
 if let Some((x,y))=result {assert_eq!(x,vec![1.;n]);assert_eq!(y,vec![1.;n]);}
}
