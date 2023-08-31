use rlst::dense::{rlst_rand_mat, RawAccess};
use rust_simd::hadamard::*;
use rust_simd::helpers::*;
use std::sync::{Arc, Mutex};

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
        let tmp = rlst_rand_mat![f64, (size_real, 1)];

        sibling_set.push(Arc::new(Mutex::new(tmp.data().to_vec())))
    }

    let kernel_data = kernel_like_data::<f64>();
    hadamard_product_simd(&sibling_set, &kernel_data);
}
