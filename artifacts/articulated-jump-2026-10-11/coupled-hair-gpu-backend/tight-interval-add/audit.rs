fn enclosure(a:f64,b:f64)->(f64,f64) {
    let sum=a+b;
    if !sum.is_finite() {return (f64::NEG_INFINITY,f64::INFINITY);}
    let virtual_b=sum-a;
    let virtual_a=sum-virtual_b;
    let error=(a-virtual_a)+(b-virtual_b);
    if !virtual_b.is_finite() || !virtual_a.is_finite() || !error.is_finite() {
        return (sum.next_down(),sum.next_up());
    }
    (if error<0. {sum.next_down()} else {sum},if error>0. {sum.next_up()} else {sum})
}
fn main() {
    let mut seed=0x138ae769b204fdc5u64;
    let next=|seed:&mut u64| { *seed^=*seed<<13;*seed^=*seed>>7;*seed^=*seed<<17; f64::from_bits(if (*seed>>52)&2047==2047 {*seed^(1<<52)} else {*seed}) };
    for i in 0..40000 {
        let a=next(&mut seed);
        let b=match i%4 {0=>next(&mut seed),1=>-a,2=>-a.next_up(),_=>f64::from_bits((next(&mut seed).to_bits()&((1u64<<52)-1))|(a.to_bits()&(!((1u64<<52)-1))))};
        if !a.is_finite() || !b.is_finite() {continue;}
        let (lo,hi)=enclosure(a,b);
        println!("{:016x} {:016x} {:016x} {:016x}",a.to_bits(),b.to_bits(),lo.to_bits(),hi.to_bits());
    }
}
