use std::time::{self, Instant};
use rust_simd::*;


fn main () {

    let (x, y, mut z) = data_f32(1000);

    let s = Instant::now();
    dotp_no_simd_bounds_f32(&x, &y, &mut z);
    println!("SIMD: {:?}", s.elapsed().as_nanos());
}