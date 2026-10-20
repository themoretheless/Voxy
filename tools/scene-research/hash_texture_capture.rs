//! Standalone: link the workspace's existing blake3 rlib; no pixel conversion.
use std::{env,fs};
fn main() {
    let mut args=env::args().skip(1);let format=args.next().expect("format");let path=args.next().expect("payload");assert!(args.next().is_none());
    let bytes=fs::read(path).expect("read payload");let hash=blake3::hash(&bytes);
    let statistics=match format.as_str() {
        "rgba32float" => {
            assert!(!bytes.is_empty() && bytes.len()%16==0);
            let mut first=None;let mut constant=true;let mut low=f32::INFINITY;let mut high=f32::NEG_INFINITY;
            for (i,word) in bytes.chunks_exact(4).enumerate() {
                let bits=u32::from_le_bytes(word.try_into().unwrap());let value=f32::from_bits(bits);assert!(value.is_finite(),"nonfinite captured pixel");
                if i%4==3 { if let Some(a)=first {constant &= a==bits;}else {first=Some(bits);} low=low.min(value);high=high.max(value); }
            }
            format!("{{\"kind\":\"float32\",\"count\":{},\"constant_bits\":{},\"first_bits\":{},\"min_bits\":{},\"max_bits\":{}}}",bytes.len()/16,constant,first.unwrap(),low.to_bits(),high.to_bits())
        }
        "rgba8unorm" => {
            assert!(!bytes.is_empty() && bytes.len()%4==0);let first=bytes[3];let mut constant=true;let mut low=255;let mut high=0;
            for alpha in bytes.iter().skip(3).step_by(4) {constant &= *alpha==first;low=low.min(*alpha);high=high.max(*alpha);}
            format!("{{\"kind\":\"uint8\",\"count\":{},\"constant_bits\":{},\"first_bits\":{},\"min_bits\":{},\"max_bits\":{}}}",bytes.len()/4,constant,first,low,high)
        }
        "bc7-rgba-unorm"|"bc7-rgba-unorm-srgb" => {assert!(!bytes.is_empty() && bytes.len()%16==0);"null".into()}
        _ => panic!("unsupported source format"),
    };
    println!("{{\"bytes\":{},\"blake3\":\"{}\",\"alpha\":{}}}",bytes.len(),hash,statistics);
}
