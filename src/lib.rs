// TODO:

// 1. Change to complex inputs, and udate hadamard product functions to products of complex numbers
// 2. add tests.
// 3. make software more flexible, so can parameterise as I would in real FMM software, and remove float templates
//
#![feature(stdsimd)]
#![feature(array_chunks)]
#![feature(slice_as_chunks)]
#![feature(portable_simd)]

use std::arch::x86_64::*;
use std::collections::{HashMap, HashSet};
use std::simd::*;
use std::sync::{Arc, Mutex, RwLock};
use std::time::*;

use itertools::*;

use bempp_tree::implementations::helpers::points_fixture;
use bempp_tree::types::domain::Domain;
use bempp_tree::types::morton::MortonKey;

use bempp_traits::tree::{MortonKeyInterface, Tree};
use bempp_tree::types::single_node::SingleNodeTree;
use num::Float;
use rand::prelude::*;
use rand::SeedableRng;
use rayon::prelude::*;
use rayon::slice::{ParallelSlice, ParallelSliceMut};

use rlst::dense::{base_matrix::BaseMatrix, rlst_mat, Dynamic, Matrix, RawAccess, VectorContainer};
use rlst::dense::{rlst_pointer_mat, rlst_rand_mat, LayoutType};

pub const BLOCK_SIZE: usize = 1024;

pub type PointsType =
    Matrix<f64, BaseMatrix<f64, VectorContainer<f64>, Dynamic, Dynamic>, Dynamic, Dynamic>;

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

type KernelData =
    Matrix<f64, BaseMatrix<f64, VectorContainer<f64>, Dynamic, Dynamic>, Dynamic, Dynamic>;

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

    let data = rlst_rand_mat![f64, (16 * 8, size_real)];

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


pub fn hadamard_kernel(kernel: &[f64], signal: &[f64], result: &mut [f64]) {
    // assert!(kernel.len() == 8);
    // assert!(signal.len() == 8);
    // assert!(result.len() == 8);

    unsafe {
        // Load the first half (4 elements) of kernel and signal into YMM registers
        let kernel_vec1 = _mm256_loadu_pd(&kernel[0]);
        let signal_vec1 = _mm256_loadu_pd(&signal[0]);

        // Compute the Hadamard product for the first half using FMA
        let res_vec1 = _mm256_fmadd_pd(kernel_vec1, signal_vec1, _mm256_setzero_pd());

        // Store the result back into the result array
        _mm256_storeu_pd(&mut result[0], res_vec1);

        // Repeat the same for the second half of the slices
        let kernel_vec2 = _mm256_loadu_pd(&kernel[4]);
        let signal_vec2 = _mm256_loadu_pd(&signal[4]);

        let res_vec2 = _mm256_fmadd_pd(kernel_vec2, signal_vec2, _mm256_setzero_pd());

        _mm256_storeu_pd(&mut result[4], res_vec2);
    }
}

pub fn hadamard_product_naive(sibling_set: &Vec<Arc<Mutex<Vec<f64>>>>, kernel_data: &Vec<f64>) {
    let expansion_order: usize = 9;
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size = p * q * r;
    let size_real = p * q * (r / 2 + 1);


    let mut res = vec![0f64; size_real*16*8];

    for i in 0..16 {

        let m2l_matrix_offset = i*size_real;
        let m2l_matrix = &kernel_data[m2l_matrix_offset..m2l_matrix_offset+size_real];

        for k in 0..8 {
            let signal = sibling_set[k].lock().unwrap();
            let res_offset = k * size_real * 16 + i * size_real;

            let chunk_size = 4;
            let chunks = size_real / chunk_size;

            // Chunking can be done more intelligently, can keep a single chunk of m2l matrix in memory
            // at once, and calculate product with all signals.

            for j in 0..chunks {
                let simd_index = j * chunk_size;

                unsafe {
                    let signal_chunk = _mm256_loadu_pd(&signal[simd_index]);
                    let kernel_chunk = _mm256_loadu_pd(&m2l_matrix[simd_index]);
                    let res_chunk = _mm256_loadu_pd(&res[simd_index]);

                    let product = _mm256_mul_pd(signal_chunk, kernel_chunk);

                    let tmp = _mm256_add_pd(product, res_chunk);

                    // let tmp = _mm256_fmadd_pd(signal_chunk, kernel_chunk, res_chunk);
                    _mm256_storeu_pd(&mut res[simd_index], tmp);

                }
            }

            let start_remainder = chunks * chunk_size;
            for j in start_remainder..size_real {
                res[res_offset + j] += signal[j] * m2l_matrix[j];
            }

            // for j in 0..size_real {
            //     res[k*size_real*16+i*size_real+j] += signal[j]*m2l_matrix[j];
            // }
        }
    }

    // println!("Hadamard product {:?}", s.elapsed());

}

pub fn hadamard_product_simd<'a>(sibling_set: &Vec<Arc<Mutex<Vec<f64>>>>, kernel_data: &Vec<f64>) {
    let expansion_order: usize = 9;
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size = p * q * r;
    let size_real = p * q * (r / 2 + 1);

    // Receiving a vec of sibling coefficients, need to go through by frequency so need to re
    // allocate so that they're arranged by frequency

    // let s = Instant::now();
    let signal_data = transpose::<f64>(sibling_set);
    // println!("Signal Transpose {:?}", s.elapsed().as_millis());

    // Result buffer
    let mut result = vec![0f64; kernel_data.len()];

    // Iterate over frequencies

    // let s = Instant::now();
    for k in 0..size_real {
        // Get out a set of 8 convolutions at the kth frequency
        // Multiply with kth frequency components of sibling set
        let signal = &signal_data[(k*8)..(k*8)+8];

        for i in 0..2 {
            let res = &mut result[(k*128)+(i * 64)..(k*128)+(i * 64) + 64];
            let kernel = &kernel_data[(k*128)+(i * 64)..(k*128)+(i * 64) + 64];

            res
                .chunks_exact_mut(8)
                .zip(kernel.chunks_exact(8))
                .for_each(|(r, k)| {

                    // Chunk of size 8 64 bits won't fit into avx2 register, need compute half of
                    // chunk at a time.
                    hadamard_kernel(k, signal,  r);

                })
        }

    }
    // println!("Hadamard product {:?}", s.elapsed());
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

pub fn m2l_naive<T>()
where
    T: Float + std::ops::AddAssign<T>,
{
    let mut data = m2l_like_data::<T>();

    // Iterate through all keys, pull up their interaction lists and save some random data to them
    let keys: Vec<MortonKey> = data.keys().cloned().collect();

    let s = Instant::now();
    for key in keys.iter() {
        let interaction_list: Vec<MortonKey> = key
            .parent()
            .neighbors()
            .iter()
            .flat_map(|pn| pn.children())
            .filter(|pnc| !key.is_adjacent_same_level(pnc))
            .collect();

        // Scatter data to all sources in interaction list
        for source in interaction_list.iter() {
            let entry = data.entry(*source);
            entry.and_modify(|e| e.iter_mut().for_each(|x| *x += T::from(1.0).unwrap()));
        }
    }
    println!("M2L naive {:?}", s.elapsed().as_millis());
}

pub fn m2l_naive_par<T>()
where
    T: Float + std::ops::AddAssign<T> + std::marker::Send,
{
    let data = m2l_like_data_arc::<T>();

    // Iterate through all keys, pull up their interaction lists and save some random data to them
    let keys: Vec<MortonKey> = data.keys().cloned().collect();

    let s = Instant::now();
    keys.into_par_iter().for_each(|key| {
        let interaction_list: Vec<MortonKey> = key
            .parent()
            .neighbors()
            .iter()
            .flat_map(|pn| pn.children())
            .filter(|pnc| !key.is_adjacent_same_level(pnc))
            .collect();

        let entry_arc = data.get(&key).unwrap();
        let mut entry = entry_arc.lock().unwrap();
        // Scatter data to all sources in interaction list
        interaction_list.into_iter().for_each(|source| {
            // Get data and scatter

            entry.iter_mut().for_each(|x| *x += T::from(1.0).unwrap());
        });
    });
    println!("M2L naive par {:?}", s.elapsed().as_millis());
}

pub fn m2l_parent_par()
{
    let data = m2l_like_data_arc::<f64>();

    let fft_data = fft_like_data_arc::<f64>();

    let mut keys: Vec<MortonKey> = data.keys().cloned().collect();
    keys.sort();

    // Iterate over parents now

    let scatter_idxs = Arc::new(RwLock::new(scatter_displacements()));


    // let kernel_data = kernel_like_data::<f64>();
    let kernel_data = kernel_like_data_transpose::<f64>();
    let s = Instant::now();
    keys.par_chunks_exact(8).for_each(|children| {
        let mut halo_data = Vec::new();

        let parent = children[0].parent();

        let sentinel = MortonKey::default();

        let parent_neigbors_children = parent
            .all_neighbors()
            .iter()
            .flat_map(|p| {
                if let Some(p) = p {
                    p.children()
                } else {
                    vec![sentinel; 8]
                }
            })
            .collect_vec();

        // Get all halo data
        for &pnc in parent_neigbors_children.iter() {
            if pnc != sentinel {
                halo_data.push(Some(Arc::clone(data.get(&pnc).unwrap())))
            } else {
                halo_data.push(None)
            }
        }

        // Will need to allocate to store temporary values of convolutions
        let expansion_order: usize = 9;
        let ncoeffs = 6*(expansion_order-1).pow(2) + 2;
        let tmp = vec![0f64; ncoeffs*16*8];

        let mut sibling_set = Vec::new();

        for c in children.iter() {
            sibling_set.push(
                Arc::clone(fft_data.get(&c).unwrap())
            )
        }

        hadamard_product_naive(&sibling_set, &kernel_data);

        for (i, dat) in halo_data.iter().enumerate() {
            if let Some(dat) = dat {
                // The indices of potentials from sibling set to save from
                let save_idxs = &scatter_idxs.read().unwrap()[i];

                // In real life would also have to lookup appropriate convolution that is being saved
                for sv_idx in save_idxs.iter() {
                    dat.lock()
                        .unwrap()
                        .iter_mut()
                        .for_each(|x| *x += f64::from(1.0));
                }
            }
        }
    });
    println!("M2L parent par {:?}", s.elapsed().as_millis());
}

pub fn scatter_displacements() -> Vec<Vec<usize>> {
    let domain = Domain {
        diameter: [1.0, 1.0, 1.0],
        origin: [0., 0., 0.],
    };

    // Create a point in the centre of a uniform tree
    let key = MortonKey::from_point(&[0.5, 0.5, 0.5], &domain, 5);

    // Find its siblings
    let siblings = key.siblings();

    // Find its parent siblings (halo)
    let parent = key.parent();
    let halo = parent.neighbors();

    let halo_children = halo.iter().flat_map(|h| h.children()).collect_vec();

    let mut halo_children_idxs = HashMap::new();

    for (i, hc) in halo_children.iter().enumerate() {
        halo_children_idxs.insert(*hc, i);
    }

    let mut scatter_idxs = vec![Vec::new(); halo_children.len()];
    // Need to find indices of each sibling's interaction list inside the halo children.

    for (i, sibling) in siblings.iter().enumerate() {
        let interaction_list: Vec<MortonKey> = sibling
            .parent()
            .neighbors()
            .iter()
            .flat_map(|pn| pn.children())
            .filter(|pnc| !sibling.is_adjacent_same_level(pnc))
            .collect();

        for source in interaction_list.iter() {
            let idx = halo_children_idxs.get(source).unwrap();
            scatter_idxs[*idx].push(i)
        }
    }

    // Will also need to return something the same shape with the associated transfer vector
    scatter_idxs
}

// fn sort_indices<T: Ord + Clone>(data: &[T]) -> Vec<usize> {
//     let mut indices: Vec<_> = (0..data.len()).collect();
//     indices.sort_by_key(|&i| &data[i]);
//     indices
// }

// pub fn dotp_naive_f64(x: &[f64], y: &[f64], z: &mut [f64]) {
//     for ((a,b), c) in x.iter().zip(y.iter()).zip(z.iter_mut()) {
//         *c += a * b;
//     }
// }

pub fn dotp_naive_f64(x: &[f64], y: &[f64], z: &mut [f64]) {
    for ((a, b), c) in x
        .chunks_exact(4)
        .zip(y.chunks_exact(4))
        .zip(z.chunks_exact_mut(4))
    {
        // assert_eq!(a.len(), b.len());
        // assert_eq!(c.len(), b.len());

        // for i in 0..c.len() {
        //     c[i] += a[i] * b[i];
        // }

        c.iter_mut()
            .zip(a.iter())
            .zip(b.iter())
            .for_each(|((c, a), b)| *c = *a * b);
    }
}

pub fn dotp_naive_f32(x: &[f32], y: &[f32], z: &mut [f32]) {
    for ((a, b), c) in x
        .chunks_exact(8)
        .zip(y.chunks_exact(8))
        .zip(z.chunks_exact_mut(8))
    {
        // assert_eq!(a.len(), b.len());
        // assert_eq!(c.len(), b.len());

        // for i in 0..c.len() {
        //     c[i] += a[i] * b[i];
        // }

        c.iter_mut()
            .zip(a.iter())
            .zip(b.iter())
            .for_each(|((c, a), b)| *c = *a * b);
    }
}

#[inline(never)]
pub fn dotp_simd_f64(x: &[f64], y: &[f64], z: &mut [f64]) {
    let chunk_size = 4;
    for ((a, b), c) in x
        .chunks_exact(chunk_size)
        .zip(y.chunks_exact(chunk_size))
        .zip(z.chunks_exact_mut(chunk_size))
    {
        unsafe {
            let x_a = _mm256_loadu_pd(a.as_ptr());
            let y_a = _mm256_loadu_pd(b.as_ptr());
            let r_a = _mm256_loadu_pd(c.as_ptr());

            // let tmp = _mm256_mul_pd(x_a, y_a);
            // let tmp2 = _mm256_add_pd(tmp, r_a);
            _mm256_storeu_pd(c.as_mut_ptr(), _mm256_fmadd_pd(x_a, y_a, r_a));
            // _mm256_storeu_pd(c.as_mut_ptr(), tmp2);
        }
    }
}

#[inline(never)]
pub fn dotp_simd_f32(x: &[f32], y: &[f32], z: &mut [f32]) {
    for ((a, b), c) in x
        .chunks_exact(8)
        .zip(y.chunks_exact(8))
        .zip(z.chunks_exact_mut(8))
    {
        unsafe {
            let x_a = _mm256_loadu_ps(a.as_ptr());
            let y_a = _mm256_loadu_ps(b.as_ptr());
            let r_a = _mm256_loadu_ps(c.as_ptr());

            _mm256_storeu_ps(c.as_mut_ptr(), _mm256_fmadd_ps(x_a, y_a, r_a));
        }
    }
}

pub fn dotp_simd_f32_portable(x: &[f32], y: &[f32], z: &mut [f32]) {
    x.array_chunks::<8>()
        .map(|&a| f32x8::from_array(a))
        .zip(y.array_chunks::<8>().map(|&b| f32x8::from_array(b)))
        .zip(
            z.array_chunks_mut::<8>(), // .map(|&mut c| f32x8::from_array(c))
        )
        .for_each(|((a, b), mut c)| {
            let mut c_s = f32x8::splat(0.0);
            c_s = a.mul_add(b, c_s);

            unsafe {
                let idxs = Simd::from_array([0, 1, 2, 3, 4, 5, 6, 7]);
                c_s.scatter(c, idxs);
            }
        });
}

pub fn dotp_simd_f64_portable(x: &[f64], y: &[f64], z: &mut [f64]) {
    x.array_chunks::<4>()
        .map(|&a| f64x4::from_array(a))
        .zip(y.array_chunks::<4>().map(|&b| f64x4::from_array(b)))
        .zip(z.array_chunks_mut::<4>())
        .for_each(|((a, b), mut c)| {
            let mut c_s = f64x4::splat(0.0);
            c_s = a.mul_add(b, c_s);

            unsafe {
                let idxs = Simd::from_array([0, 1, 2, 3]);
                c_s.scatter(c, idxs);
            }
        });
}

#[inline(never)]
pub fn dotp_simd_f32_par(x: &[f32], y: &[f32], z: &mut [f32]) {
    let chunk_size = 8;
    x.par_chunks_exact(chunk_size)
        .zip(y.par_chunks_exact(chunk_size))
        .zip(z.par_chunks_exact_mut(chunk_size))
        .for_each(|((a, b), c)| unsafe {
            let x_a = _mm256_loadu_ps(a.as_ptr());
            let y_a = _mm256_loadu_ps(b.as_ptr());
            let r_a = _mm256_loadu_ps(c.as_ptr());

            _mm256_storeu_ps(c.as_mut_ptr(), _mm256_fmadd_ps(x_a, y_a, r_a));
        });
}

#[inline(never)]
pub fn dotp_simd_f64_par(x: &[f64], y: &[f64], z: &mut [f64]) {
    let chunk_size = 4;
    x.par_chunks_exact(chunk_size)
        .zip(y.par_chunks_exact(chunk_size))
        .zip(z.par_chunks_exact_mut(chunk_size))
        .for_each(|((a, b), c)| unsafe {
            let x_a = _mm256_loadu_pd(a.as_ptr());
            let y_a = _mm256_loadu_pd(b.as_ptr());
            let r_a = _mm256_loadu_pd(c.as_ptr());

            _mm256_storeu_pd(c.as_mut_ptr(), _mm256_fmadd_pd(x_a, y_a, r_a));
        });
}

#[inline(never)]
pub fn dotp_no_simd_bounds_f32(x: &[f32], y: &[f32], z: &mut [f32]) {
    for idx in 0..x.len() / 8 {
        unsafe {
            let (a, b, c) = (
                x.get_unchecked(idx * 8),
                y.get_unchecked(idx * 8),
                z.get_unchecked_mut(idx * 8),
            );
            let x_a = _mm256_loadu_ps(a);
            let y_a = _mm256_loadu_ps(b);
            let r_a = _mm256_loadu_ps(c);
            _mm256_storeu_ps(c, _mm256_fmadd_ps(x_a, y_a, r_a));
        }
    }
}
