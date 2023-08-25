use std::time::Instant;
use rust_simd::*;


fn main () {

    let s = Instant::now();
    let (x, y, mut z) = data_f64(10000);
    dotp_naive_f64(&x, &y, &mut z);
    println!("Naive: {:?}", s.elapsed().as_millis());

    println!("{:?}", &z[0..10]);
}