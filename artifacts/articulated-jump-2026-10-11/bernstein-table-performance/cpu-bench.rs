#![allow(dead_code)]
mod hair {
 #[path="/Users/themoretheless/Documents/ChatGPT/Voxy/crates/physics/src/hair/contact_polynomial.rs"] mod polynomial;
 use polynomial::*;
 fn reference(input:Poly,degree:usize)->Poly {
  let choose=|n:usize,k:usize|(0..k).fold(1u32,|v,i|v*(n-i) as u32/(i+1) as u32);
  let mut out=zero();
  for i in 0..=degree {for j in 0..=i {
   let ratio=choose(i,j) as f64/choose(degree,j) as f64;
   out[i]=out[i].add(input[j].mul(Bound {lo:ratio.next_down(),hi:ratio.next_up()}));
  }} out
 }
 pub fn run(){
  let old=std::env::args().nth(1).as_deref()==Some("reference");
  let input:Poly=std::array::from_fn(|i|Bound {lo:0.123+i as f64*0.25,hi:0.124+i as f64*0.25});
  for degree in 0..=6 {let a=reference(input,degree);let b=bernstein(input,degree);
   for (a,b) in a.iter().zip(b){assert_eq!((a.lo.to_bits(),a.hi.to_bits()),(b.lo.to_bits(),b.hi.to_bits()));}
  }
  for _ in 0..2000000 {let x=std::hint::black_box(input);let d=std::hint::black_box(6);
   std::hint::black_box(if old {reference(x,d)} else {bernstein(x,d)});
  }
 }
}
fn main(){hair::run();}
