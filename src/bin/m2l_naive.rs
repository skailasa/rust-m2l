
use bempp_tree::implementations::helpers::points_fixture;
use bempp_tree::types::single_node::SingleNodeTree;
use bempp_traits::tree::Tree;

use rlst::dense::RawAccess;

use rust_simd::m2l::*;

fn main() {
    let npoints = 1000000;
    let ncrit = 150;
    let depth = 5;
    let expansion_order = 9;

    let points = points_fixture(npoints, None, None);

    let global_idxs: Vec<usize> = (0..npoints).collect();

    let tree = SingleNodeTree::new(points.data(), false, Some(ncrit), Some(depth), &global_idxs);
    m2l_naive(expansion_order, &tree);
}
