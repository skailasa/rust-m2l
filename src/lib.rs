//! Crate to experiment with SIMD M2L-type operations in Rust
#![feature(stdsimd)]
#![feature(array_chunks)]
#![feature(slice_as_chunks)]
#![feature(portable_simd)]
pub mod dotp;
pub mod hadamard;
pub mod helpers;
pub mod m2l;

// // TODO:
// // 1. Change to complex inputs, and update hadamard product functions to products of complex numbers
// // 2. Add tests.
// // 3. Make software more flexible, so can parameterise as I would in real FMM software, and remove float templates
