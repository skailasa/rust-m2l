#[macro_use]
extern crate ispc;
ispc_module!(ispdotc);

use rust_simd::*;

fn main () {

    let (x, y, mut z) = data_f32(1000);
    // ispdotc::dotp(&x, &y, &mut z);

}