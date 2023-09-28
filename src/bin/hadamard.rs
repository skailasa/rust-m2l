use num::complex::Complex64;
use rlst::dense::{rlst_rand_mat, RawAccess};
use rust_simd::hadamard::*;
use rust_simd::helpers::*;
use std::sync::{Arc, Mutex, RwLock};

#[cfg(all(target_arch = "aarch64", feature = "neon"))]
use std::arch::aarch64::*;
use rust_simd::hadamard::aarch64::*;


#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
pub fn main() {
    let expansion_order: usize = 9;
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size = p * q * r;
    let size_real = p * q * (r / 2 + 1);

    let mut sibling_set = Vec::new();

    for i in 0..8 {
        let tmp = rlst_rand_mat![Complex64, (size_real, 1)];

        sibling_set.push(Arc::new(Mutex::new(tmp.data().to_vec())))
    }

    let kernel_data = RwLock::new(kernel_like_data(expansion_order));
    hadamard_product_simd(expansion_order, &sibling_set, &kernel_data);
}

#[cfg(target_arch = "aarch64")]
fn main() {

    let a = Complex64::new(1.0, 2.0);
    let b = Complex64::new(3.0, 4.0);

    unsafe {

        let complex_ref = &a;
        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
        let ptr = tuple_ptr as *const f64;        
        let a_ra = vld1q_f64(ptr);
        
        let complex_ref = &b;
        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
        let ptr = tuple_ptr as *const f64;        
        let b_ra = vld1q_f64(ptr);

        let result = hadamard_product_kernel_neon(a_ra, b_ra);
        println!("Running on aarch64! {:?}", result);
    }

    // let expansion_order: usize = 9;
    // let n = 2 * expansion_order - 1;
    // let &(m, n, o) = &(n, n, n);

    // let p = m + 1;
    // let q = n + 1;
    // let r = o + 1;
    // let size = p * q * r;
    // let size_real = p * q * (r / 2 + 1);

    // let mut sibling_set = Vec::new();

    // for i in 0..8 {
    //     let tmp = rlst_rand_mat![Complex64, (size_real, 1)];
    //     sibling_set.push(Arc::new(Mutex::new(tmp.data().to_vec())))
    // }

    // let kernel_data = RwLock::new(kernel_like_data(expansion_order));
    // hadamard_product_simd_neon(expansion_order, &sibling_set, &kernel_data);

}