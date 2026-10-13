#![allow(dead_code)]
mod hair {
    #[path = "/Users/themoretheless/Documents/ChatGPT/Voxy/crates/physics/src/hair/contact_polynomial.rs"] mod polynomial;
    use polynomial::*;
    fn reference_split(mut values:Poly,degree:usize)->(Poly,Poly) {
        let mut left=zero();let mut right=zero();left[0]=values[0];right[degree]=values[degree];
        for level in 1..=degree {
            for i in 0..=degree-level {values[i]=values[i].add(values[i+1]).mul(Bound::exact(0.5));}
            left[level]=values[0];right[degree-level]=values[degree-level];
        }
        (left,right)
    }
    pub fn run() {
        let old=std::env::args().nth(1).as_deref()==Some("reference");
        let input:Poly=std::array::from_fn(|i|Bound {lo:0.123+i as f64*0.25,hi:0.124+i as f64*0.25});
        for _ in 0..4000000 {
            let x=std::hint::black_box(input);
            std::hint::black_box(if old {reference_split(x,6)} else {split(x,6)});
        }
    }
}
fn main(){hair::run();}
