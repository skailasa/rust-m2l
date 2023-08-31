
use std::{
    sync::{Arc, RwLock},
    time::Instant,
    collections::HashMap
};

use num::Float;
use rayon::prelude::*;
use itertools::*;

use bempp_tree::types::{
    morton::MortonKey,
    domain::Domain
};

use crate::{
    hadamard::hadamard_product_naive,
    helpers::{
        m2l_like_data,
        m2l_like_data_arc,
        fft_like_data_arc,
        kernel_like_data_transpose,
    }
};


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
    let kernel_data = RwLock::new(kernel_like_data_transpose::<f64>());
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

        let mut sibling_set = Vec::new();

        for c in children.iter() {
            sibling_set.push(
                Arc::clone(fft_data.get(&c).unwrap())
            )
        }

        hadamard_product_naive(&sibling_set, &kernel_data);
        // hadamard_product_simd(&sibling_set, &kernel_data);

        for (i, dat) in halo_data.iter().enumerate() {
            if let Some(dat) = dat {
                // The indices of potentials from sibling set to save from
                let save_idxs = &scatter_idxs.read().unwrap()[i];

                // In real life would also have to lookup appropriate convolution that is being saved
                for _ in save_idxs.iter() {
                    dat.lock()
                        .unwrap()
                        .iter_mut()
                        .for_each(|x| *x += f64::from(1.0));
                }
            }
        }
    });
    println!("M2L parent par {:?}", s.elapsed());
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

