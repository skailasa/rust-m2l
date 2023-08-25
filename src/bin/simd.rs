use std::time::{Instant};
use rust_simd::*;


fn main () {

    let (x, y, mut z) = data_f32(1000);

    let s = Instant::now();
    dotp_simd_f32(&x, &y, &mut z);
    println!("SIMD: {:?}", s.elapsed().as_nanos());
}