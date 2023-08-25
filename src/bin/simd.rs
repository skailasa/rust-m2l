use rust_simd::*;


fn main () {

    let (x, y, mut z) = data_f32(1000);
    dotp_simd_f32(&x, &y, &mut z);

}