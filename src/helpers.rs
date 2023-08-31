use std::{
    collections::HashMap,
    sync::{Arc, Mutex}
};

use num::{Float, complex::*};

use bempp_tree::{
    implementations::helpers::points_fixture,
    types::{
        morton::MortonKey,
        single_node::SingleNodeTree
    }
};

use bempp_traits::tree::Tree;

use rlst::dense::{RawAccess, rlst_rand_mat};


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


pub fn kernel_like_data<T>() -> Vec<f64>
where
    T: Float,
{
    let expansion_order: usize = 9;
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size = p * q * r;
    let size_real = p * q * (r / 2 + 1);

    let data = rlst_rand_mat![f64, (16, size_real)];

    data.data().to_vec()
}

pub fn kernel_like_data_transpose<T>() -> Vec<f64>
where
    T: Float,
{
    let expansion_order: usize = 9;
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size = p * q * r;
    let size_real = p * q * (r / 2 + 1);

    let data = rlst_rand_mat![f64, (size_real, 16)];

    data.data().to_vec()
}


fn transpose<T: Clone>(data: &Vec<Arc<Mutex<Vec<T>>>>) -> Vec<T> {
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


// Generate random coefficients attached to a set of keys for testing M2L data access
pub fn m2l_like_data<T>() -> HashMap<MortonKey, Vec<T>>
where
    T: Float,
{
    let expansion_order: usize = 9;
    let npoints = 1000000;
    let points = points_fixture(npoints, None, None);
    let mut data: HashMap<MortonKey, Vec<T>> = HashMap::new();

    let ncrit = 150;
    let depth = 5;
    let global_idxs: Vec<usize> = (0..npoints).collect();

    let tree = SingleNodeTree::new(points.data(), false, Some(ncrit), Some(depth), &global_idxs);

    // let keys: Vec<usize> = (0..nkeys).into_iter().collect();
    let ncoeffs = 6 * (expansion_order - 1).pow(2) + 2;

    for key in tree.get_all_leaves_set().iter() {
        let tmp = vec![T::zero(); ncoeffs];
        data.insert(*key, tmp);
    }

    data
}

// Generate random coefficients attached to a set of keys for testing M2L data access
pub fn m2l_like_data_arc<T>() -> HashMap<MortonKey, Arc<Mutex<Vec<T>>>>
where
    T: Float,
{
    let expansion_order: usize = 9;
    let npoints = 1000000;
    let points = points_fixture(npoints, None, None);
    let mut data: HashMap<MortonKey, Arc<Mutex<Vec<T>>>> = HashMap::new();

    let ncrit = 150;
    let depth = 5;
    let global_idxs: Vec<usize> = (0..npoints).collect();

    let tree = SingleNodeTree::new(points.data(), false, Some(ncrit), Some(depth), &global_idxs);

    // let keys: Vec<usize> = (0..nkeys).into_iter().collect();
    let ncoeffs = 6 * (expansion_order - 1).pow(2) + 2;

    for key in tree.get_all_leaves_set().iter() {
        let tmp = Arc::new(Mutex::new(vec![T::zero(); ncoeffs]));
        data.insert(*key, tmp);
    }

    data
}

// Generate random coefficients attached to a set of keys for testing M2L data access
pub fn fft_like_data_arc<T>() -> HashMap<MortonKey, Arc<Mutex<Vec<T>>>>
where
    T: Float,
{
    let expansion_order: usize = 9;

    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size = p * q * r;
    let size_real = p * q * (r / 2 + 1);

    let npoints = 1000000;
    let points = points_fixture(npoints, None, None);
    let mut data: HashMap<MortonKey, Arc<Mutex<Vec<T>>>> = HashMap::new();

    let ncrit = 150;
    let depth = 5;
    let global_idxs: Vec<usize> = (0..npoints).collect();

    let tree = SingleNodeTree::new(points.data(), false, Some(ncrit), Some(depth), &global_idxs);


    for key in tree.get_all_leaves_set().iter() {
        let tmp = Arc::new(Mutex::new(vec![T::zero(); size_real]));
        data.insert(*key, tmp);
    }

    data
}

// Generate random coefficients attached to sets of keys, but at their parent level.
pub fn m2l_like_data_parent<T>() -> HashMap<MortonKey, Arc<Mutex<Vec<T>>>>
where
    T: Float,
{
    let expansion_order: usize = 9;
    let npoints = 1000000;
    let points = points_fixture(npoints, None, None);
    let mut data: HashMap<MortonKey, Arc<Mutex<Vec<T>>>> = HashMap::new();

    let ncrit = 150;
    let depth = 5;
    let global_idxs: Vec<usize> = (0..npoints).collect();

    let tree = SingleNodeTree::new(points.data(), false, Some(ncrit), Some(depth), &global_idxs);

    let ncoeffs = 6 * (expansion_order - 1).pow(2) + 2;

    let parents = tree.get_keys(depth - 1).unwrap();

    for key in parents.iter() {
        let tmp = Arc::new(Mutex::new(vec![T::zero(); ncoeffs * 8]));
        data.insert(*key, tmp);
    }

    data
}
