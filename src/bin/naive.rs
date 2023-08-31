use rust_simd::dotp::*;
use rust_simd::helpers::*;
use std::time::Instant;

fn main() {
    let s = Instant::now();
    let (x, y, mut z) = data_f32(10000);
    dotp_naive_f32(&x, &y, &mut z);
    println!("Naive: {:?}", s.elapsed().as_millis());

    println!("{:?}", &z[0..10]);
}
