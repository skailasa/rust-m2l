use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use num::{complex::*, Float, Zero, One};

use bempp_tree::{
    implementations::helpers::points_fixture,
    types::{morton::MortonKey, single_node::SingleNodeTree},
};

use bempp_traits::tree::Tree;

use rlst::dense::{rlst_rand_mat, RawAccess};

pub const BLOCK_SIZE: usize = 1024;

pub fn data_f64(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let size: usize = BLOCK_SIZE * n;

    let x: Vec<f64> = (0..size).enumerate().map(|(i, _)| i as f64).collect();
    let y: Vec<f64> = (0..size).enumerate().map(|(i, _)| i as f64).collect();
    let z = vec![0f64; size];

    (x, y, z)
}

pub fn data_f32(n: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let size: usize = BLOCK_SIZE * n;

    let x: Vec<f32> = (0..size).enumerate().map(|(i, _)| i as f32).collect();
    let y: Vec<f32> = (0..size).enumerate().map(|(i, _)| i as f32).collect();
    let z = vec![0f32; size];

    (x, y, z)
}

// Dummy data that mirrors that of the FFT of Green's fct evaluations
pub fn kernel_like_data(expansion_order: usize) -> Vec<Complex64> {
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size_real = p * q * (r / 2 + 1);

    let data = rlst_rand_mat![Complex64, (16, size_real)];

    data.data().to_vec()
}

// Dummy data that mirrors that of the FFT of Green's fct evaluations
pub fn kernel_like_data_transpose(expansion_order: usize) -> Vec<Complex64> {
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size_real = p * q * (r / 2 + 1);

    let data = rlst_rand_mat![Complex64, (size_real, 16)];

    data.data().to_vec()
}

pub fn transpose<T: Clone>(data: &Vec<Arc<Mutex<Vec<T>>>>) -> Vec<T> {
    let outer_len = data.len();
    if outer_len == 0 {
        return Vec::new();
    }

    let inner_len = data[0].lock().unwrap().len();
    let mut transposed = Vec::with_capacity(inner_len * outer_len);

    for i in 0..inner_len {
        for j in 0..outer_len {
            let value = data[j].lock().unwrap()[i].clone();
            transposed.push(value);
        }
    }

    transposed
}

// Generate random multipole coefficients attached to a set of keys for testing M2L data access
pub fn m2l_like_data(
    expansion_order: usize,
    tree: &SingleNodeTree,
) -> HashMap<MortonKey, Vec<f64>> {
    let mut data = HashMap::new();

    let ncoeffs = 6 * (expansion_order - 1).pow(2) + 2;

    for key in tree.get_all_leaves_set().iter() {
        let tmp = vec![0.; ncoeffs];
        data.insert(*key, tmp);
    }

    data
}

// Generate random multipole coefficients attached to a set of keys for testing M2L data access
pub fn m2l_like_data_arc(
    expansion_order: usize,
    tree: &SingleNodeTree,
) -> HashMap<MortonKey, Arc<Mutex<Vec<f64>>>> {
    let ncoeffs = 6 * (expansion_order - 1).pow(2) + 2;
    let mut data = HashMap::new();

    for key in tree.get_all_leaves_set().iter() {
        let tmp = Arc::new(Mutex::new(vec![0.; ncoeffs]));
        data.insert(*key, tmp);
    }

    data
}


// Generate random coefficients attached to a set of keys for testing M2L data access
pub fn fft_like_data_arc(
    expansion_order: usize,
    tree: &SingleNodeTree,
) -> HashMap<MortonKey, Arc<Mutex<Vec<Complex64>>>> {
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size_real = p * q * (r / 2 + 1);

    let mut data: HashMap<MortonKey, Arc<Mutex<Vec<Complex64>>>> = HashMap::new();

    for key in tree.get_all_leaves_set().iter() {
        let tmp = Arc::new(Mutex::new(vec![Complex64::one(); size_real]));
        data.insert(*key, tmp);
    }

    data
}

pub fn fft_like_data_transposed(expansion_order: usize, tree: &SingleNodeTree) -> Vec<Complex<f64>> {
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size_real = p * q * (r / 2 + 1);

    let data = vec![Complex64::one(); size_real*tree.get_all_leaves_set().len()];

    data
}

pub fn fft_like_data_arc_vec(
    expansion_order: usize,
    tree: &SingleNodeTree
) -> Vec<Arc<Mutex<Vec<Complex64>>>>{

    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size_real = p * q * (r / 2 + 1);

    let mut data = Vec::new();

    let nkeys = tree.get_all_leaves_set().len();

    for _ in  0..nkeys {
        let tmp = Arc::new(Mutex::new(vec![Complex64::zero(); size_real]));
        data.push(tmp);
    }

    data
}

// // Generate random multipole coefficients attached to sets of keys, but at their parent level.
// pub fn m2l_like_data_parent(
//     expansion_order: usize,
//     tree: &SingleNodeTree,
// ) -> HashMap<MortonKey, Arc<Mutex<Vec<f64>>>> {
//     let ncoeffs = 6 * (expansion_order - 1).pow(2) + 2;

//     let parents = tree.get_keys(tree.get_depth() - 1).unwrap();
//     let mut data = HashMap::new();

//     for key in parents.iter() {
//         let tmp = Arc::new(Mutex::new(vec![0.; ncoeffs * 8]));
//         data.insert(*key, tmp);
//     }

//     data
// }
