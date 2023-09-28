use rust_simd::dotp::*;
use rust_simd::helpers::*;
use std::time::Instant;

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
fn main() {
    let (x, y, mut z) = data_f64(10000);
    let mut tst = z.clone();

    let s = Instant::now();
    dotp_simd_f64_par(&x, &y, &mut z);
    println!("SIMD PAR: {:?}", s.elapsed().as_millis());

    // Test code
    dotp_naive_f64(&x, &y, &mut tst);
    tst.iter()
        .zip(z.iter())
        .for_each(|(a, b)| assert!(*a == *b));
}

#[cfg(target_arch = "aarch64")]
fn main() {
    println!("Running on aarch64!");
}