use std::time::Instant;
use rust_simd::*;


fn main () {

    let (x, y, mut z) = data_f64(10000);

    let s = Instant::now();
    unsafe {
        dotp_simd_f64(&x, &y, &mut z);
    }
    println!("SIMD: {:?}", s.elapsed().as_millis());

    println!("{:?}", &z[0..10]);
}