use bempp_traits::tree::Tree;
use bempp_tree::implementations::helpers::points_fixture;
use bempp_tree::types::single_node::SingleNodeTree;

use rlst::dense::RawAccess;

use rust_simd::m2l::*;

#[cfg(all(target_arch = "aarch64", feature = "neon"))]
use rust_simd::m2l::aarch64::*;

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
use rust_simd::m2l::x86::*;

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
fn main() {
    let npoints = 1000000;
    let ncrit = 150;
    let depth = 5;
    let expansion_order = 9;

    let points = points_fixture(npoints, None, None);

    let global_idxs: Vec<usize> = (0..npoints).collect();

    let tree = SingleNodeTree::new(points.data(), false, Some(ncrit), Some(depth), &global_idxs);

    // m2l_parent_par_naive(expansion_order, &tree);
    // m2l_parent_par_simd_avx(expansion_order, &tree);
}

#[cfg(all(target_arch = "aarch64", feature = "neon"))]
fn main() {
    let npoints = 1000000;
    let ncrit = 150;
    let depth = 5;
    let expansion_order = 9;

    let points = points_fixture(npoints, None, None);

    let global_idxs: Vec<usize> = (0..npoints).collect();

    let tree = SingleNodeTree::new(points.data(), false, Some(ncrit), Some(depth), &global_idxs);
    m2l_parent_par_simd(expansion_order, &tree);
    m2l_parent_par_naive(expansion_order, &tree);
}
