use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};

use itertools::*;
use num::{Float, Complex, complex::Complex64, One, Zero};
use rayon::prelude::*;

use bempp_tree::types::{domain::Domain, morton::MortonKey, single_node::SingleNodeTree};

use crate::{
    hadamard::hadamard_product_naive,
    helpers::{fft_like_data_arc, kernel_like_data_transpose, m2l_like_data, m2l_like_data_arc, fft_like_data_arc_vec, kernel_like_data, transpose, fft_like_data_transposed},
};


pub fn m2l_naive(expansion_order: usize, tree: &SingleNodeTree) {
    let mut data = m2l_like_data(expansion_order, tree);

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
            entry.and_modify(|e| e.iter_mut().for_each(|x| *x += 0.));
        }
    }
    println!("M2L naive {:?}", s.elapsed().as_millis());
}

pub fn m2l_naive_par(expansion_order: usize, tree: &SingleNodeTree) {
    let data = m2l_like_data_arc(expansion_order, tree);
    let fft_data = fft_like_data_arc(expansion_order, tree);
    let ifft_data = fft_like_data_arc(expansion_order, tree);

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

        // Scatter data to all sources in interaction list
        interaction_list.into_iter().for_each(|source| {
            // Get data and scatter
            let entry_arc: &Arc<std::sync::Mutex<Vec<num::Complex<f64>>>> = ifft_data.get(&source).unwrap();
            let mut entry = entry_arc.lock().unwrap();
            entry.iter_mut().for_each(|x| *x += Complex64::new(0.0, 0.0));
        });
    });
    println!("M2L naive par {:?}", s.elapsed());
}

pub fn m2l_parent_par_naive(expansion_order: usize, tree: &SingleNodeTree) {
    let data = m2l_like_data_arc(expansion_order, tree);
    let mut keys: Vec<MortonKey> = data.keys().cloned().collect();
    keys.sort();

    // Iterate over parents now
    let scatter_idxs = Arc::new(RwLock::new(scatter_displacements()));

    // let kernel_data = kernel_like_data::<f64>();
    let kernel_data = RwLock::new(kernel_like_data_transpose(expansion_order));

    let s = Instant::now();
    let fft_data = fft_like_data_arc(expansion_order, tree);
    let ifft_data = fft_like_data_arc(expansion_order, tree);

    // For non-uniform trees simply have to iterate over each key in a level, computing for keys below to ensure
    // existence.
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
                halo_data.push(Some(Arc::clone(ifft_data.get(&pnc).unwrap())))
            } else {
                halo_data.push(None)
            }
        }

        let mut sibling_set = Vec::new();

        for c in children.iter() {
            sibling_set.push(Arc::clone(fft_data.get(&c).unwrap()))
        }

        let hadamard_products = hadamard_product_naive(expansion_order, &sibling_set, &kernel_data);

        for (i, dat) in halo_data.iter().enumerate() {
            if let Some(dat) = dat {
                // The indices of potentials from sibling set to save from
                let save_idxs = &scatter_idxs.read().unwrap()[i];

                // In real life would also have to lookup appropriate convolution that is being saved
                for _ in save_idxs.iter() {
                    dat.lock()
                        .unwrap()
                        .iter_mut()
                        .for_each(|x| *x += Complex64{re: 1.0, im: 1.0} );
                }
            }
        }
    });
    println!("M2L parent par naive {:?}", s.elapsed());
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


#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
pub mod x86 {   
    use super::*;
    use std::arch::x86_64::*;
    use crate::hadamard::hadamard_product_simd;

    pub fn m2l_parent_par_simd(expansion_order: usize, tree: &SingleNodeTree) {
        let n = 2 * expansion_order - 1;
        let &(m, n, o) = &(n, n, n);

        let p = m + 1;
        let q = n + 1;
        let r = o + 1;
        let size_real = p * q * (r / 2 + 1);

        let data = m2l_like_data_arc(expansion_order, tree);
        let mut keys: Vec<MortonKey> = data.keys().cloned().collect();
        keys.sort();

        // Iterate over parents now
        let scatter_idxs = Arc::new(RwLock::new(scatter_displacements()));

        let kernel_data = RwLock::new(kernel_like_data_transpose(expansion_order));

        let s = Instant::now();
        let fft_data = fft_like_data_arc(expansion_order, tree);
        let ifft_data = fft_like_data_arc(expansion_order, tree);

        keys.chunks_exact(4096).for_each(|key_block| {
            key_block.par_chunks_exact(8).for_each(|children| {
                let mut halo_data = vec::new();

                let parent = children[0].parent();

                let sentinel = mortonkey::default();

                let parent_neigbors_children = parent
                    .all_neighbors()
                    .iter()
                    .flat_map(|p| {
                        if let some(p) = p {
                            p.children()
                        } else {
                            vec![sentinel; 8]
                        }
                    })
                    .collect_vec();

                // get all halo data
                for &pnc in parent_neigbors_children.iter() {
                    if pnc != sentinel {
                        halo_data.push(some(arc::clone(ifft_data.get(&pnc).unwrap())))
                    } else {
                        halo_data.push(none)
                    }
                }

                let mut sibling_set = vec::new();

                for c in children.iter() {
                    sibling_set.push(arc::clone(fft_data.get(&c).unwrap()))
                }
                // until this point runtime is negligible (order 0.5s in total, including initial data instantiation)

                // takes a little over 1 second
                let hadamard_products = hadamard_product_simd(expansion_order, &sibling_set, &kernel_data);

                // this takes more than 3 seconds for demo problem, same obv for naive code
                // is there any way to use simd for the saves, mimicking what dhairya manages to do?
                // have to somehow use halo data within the hadamard product kernel, and in the exact right place
                // that wouldn't even reduce the number of saves, just move this loop elsewhere???

                for (i, dat) in halo_data.iter().enumerate() {
                    if let some(dat) = dat {
                        // the indices of potentials from sibling set to save from
                        let save_idxs = &scatter_idxs.read().unwrap()[i];
                        let mut dat_mut_ref = dat.lock().unwrap();

                        // in real life would also have to lookup appropriate convolution that is being saved

                        // for _ in save_idxs.iter() {
                        //     dat_mut_ref
                        //         .iter_mut()
                        //         .for_each(|x| *x += complex{re: 1.0, im: 1.0} );
                        // }
                        for _ in 0..1 {
                            dat_mut_ref
                                .iter_mut()
                                .for_each(|x| *x += 1.0 );
                        }
                    }
                }
            });

        });

        // keys.par_chunks_exact(8).for_each(|children| {
        //     let mut halo_data = Vec::new();

        //     let parent = children[0].parent();

        //     let sentinel = MortonKey::default();

        //     let parent_neigbors_children = parent
        //         .all_neighbors()
        //         .iter()
        //         .flat_map(|p| {
        //             if let Some(p) = p {
        //                 p.children()
        //             } else {
        //                 vec![sentinel; 8]
        //             }
        //         })
        //         .collect_vec();

        //     // Get all halo data
        //     for &pnc in parent_neigbors_children.iter() {
        //         if pnc != sentinel {
        //             halo_data.push(Some(Arc::clone(ifft_data.get(&pnc).unwrap())))
        //         } else {
        //             halo_data.push(None)
        //         }
        //     }

        //     let mut sibling_set = Vec::new();

        //     for c in children.iter() {
        //         sibling_set.push(Arc::clone(fft_data.get(&c).unwrap()))
        //     }
        //     // Until this point runtime is negligible (order 0.5s in total, including initial data instantiation)

        //     // Takes a little over 1 second
        //     hadamard_product_simd(expansion_order, &sibling_set, &kernel_data);

        //     // This takes more than 3 seconds for demo problem, same obv for naive code
        //     // Is there any way to use SIMD for the saves, mimicing what Dhairya manages to do?
        //     // Have to somehow use halo data WITHIN the hadamard product kernel, and in the exact right place
        //     // That wouldn't even reduce the number of saves, just move this loop elsewhere???

        //     for (i, dat) in halo_data.iter().enumerate() {
        //         if let Some(dat) = dat {
        //             // The indices of potentials from sibling set to save from
        //             let save_idxs = &scatter_idxs.read().unwrap()[i];
        //             let mut dat_mut_ref = dat.lock().unwrap();

        //             // In real life would also have to lookup appropriate convolution that is being saved

        //             for _ in save_idxs.iter() {
        //                 dat_mut_ref
        //                     .iter_mut()
        //                     .for_each(|x| *x += Complex{re: 1.0, im: 1.0} );
        //             }
        //         }
        //     }
        // });
        println!("M2L parent par SIMD {:?}", s.elapsed());
    }


    fn store_with_avx(data: &[Complex<f64>], destination: &mut Complex<f64>) {
        unsafe {
            let complex_ref = &data[0];
            let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
            let ptr = tuple_ptr as *mut f64;
            let a_ra = _mm256_loadu_pd(ptr);

            let complex_ref = destination;
            let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
            let mut_ptr = tuple_ptr as *mut f64;
            _mm256_storeu_pd(mut_ptr, a_ra);
        }

    }

}

#[cfg(all(target_arch = "aarch64", feature = "neon"))]
pub mod aarch64 {
    use super::*;
    use std::arch::aarch64::*;
    use crate::hadamard::aarch64::hadamard_product_simd_neon;

    pub fn m2l_parent_par_simd(expansion_order: usize, tree: &SingleNodeTree) {

        let n = 2 * expansion_order - 1;
        let &(m, n, o) = &(n, n, n);

        let p = m + 1;
        let q = n + 1;
        let r = o + 1;
        let size_real = p * q * (r / 2 + 1);

        let data = m2l_like_data_arc(expansion_order, tree);
        let mut keys: Vec<MortonKey> = data.keys().cloned().collect();
        keys.sort();

        // Iterate over parents now
        let scatter_idxs = Arc::new(RwLock::new(scatter_displacements()));

        let kernel_data = RwLock::new(kernel_like_data_transpose(expansion_order));

        let s = Instant::now();
        let fft_data = fft_like_data_arc(expansion_order, tree);
        let ifft_data = fft_like_data_arc(expansion_order, tree);

        keys.chunks_exact(512).for_each(|key_block| {
            key_block.par_chunks_exact(8).for_each(|children| {
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

                // get all halo data
                for &pnc in parent_neigbors_children.iter() {
                    if pnc != sentinel {
                        halo_data.push(Some(Arc::clone(ifft_data.get(&pnc).unwrap())))
                    } else {
                        halo_data.push(None)
                    }
                }

                let mut sibling_set = Vec::new();

                for c in children.iter() {
                    sibling_set.push(Arc::clone(fft_data.get(&c).unwrap()))
                }
                // until this point runtime is negligible (order 0.5s in total, including initial data instantiation)

                // takes a little over 1 second
                let hadamard_products = hadamard_product_simd_neon(expansion_order, &sibling_set, &kernel_data);

                // this takes more than 3 seconds for demo problem, same obv for naive code
                // is there any way to use simd for the saves, mimicking what dhairya manages to do?
                // have to somehow use halo data within the hadamard product kernel, and in the exact right place
                // that wouldn't even reduce the number of saves, just move this loop elsewhere???
                for (i, dat) in halo_data.iter().enumerate() {
                    if let Some(dat) = dat {
                        // the indices of potentials from sibling set to save from
                        let save_idxs = &scatter_idxs.read().unwrap()[i];
                        let mut dat_mut_ref = dat.lock().unwrap();

                        // in real life would also have to lookup appropriate convolution that is being saved

                        for _ in save_idxs.iter() {
                            dat_mut_ref
                                .iter_mut()
                                .for_each(|x| *x += Complex64{re: 1.0, im: 1.0} );
                        }
                    }
                }
            });

        });

        println!("M2L parent par SIMD {:?}", s.elapsed());
    }

}
