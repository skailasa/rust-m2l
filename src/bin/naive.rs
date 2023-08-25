use std::time::{Instant};
use rust_simd::*;


fn main () {

    let s = Instant::now();
    let (x, y, mut z) = data_f32(1000);
    dotp_naive_f32(&x, &y, &mut z);
    println!("Naive: {:?}", s.elapsed().as_nanos());
}