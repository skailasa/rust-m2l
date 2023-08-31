//! Crate to experiment with SIMD M2L-type operations in Rust
#![feature(stdsimd)]
#![feature(array_chunks)]
#![feature(slice_as_chunks)]
#![feature(portable_simd)]
pub mod hadamard;
pub mod helpers;
pub mod dotp;
pub mod m2l;

// // TODO:
// // 1. Change to complex inputs, and udate hadamard product functions to products of complex numbers
// // 2. add tests.
// // 3. make software more flexible, so can parameterise as I would in real FMM software, and remove float templates