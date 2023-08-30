use rust_simd::*;
use std::time::Instant;

fn main() {
    let (x, y, mut z) = data_f64(10000);
    let mut tst = z.clone();

    let s = Instant::now();
    // dotp_naive_f32(&x, &y, &mut z);
    dotp_simd_f64(&x[..], &y[..], &mut z[..]);

    println!("SIMD: {:?}", s.elapsed().as_millis());

    // Test code
    // println!("{:?}", z);
    // dotp_naive_f32(&x, &y, &mut tst);
    // tst.iter().zip(z.iter()).for_each(|(a, b)| assert!(*a == *b));
}
