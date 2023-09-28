//! Crate to experiment with SIMD M2L-type operations in Rust
#![feature(stdsimd)]
#![feature(array_chunks)]
#![feature(slice_as_chunks)]
#![feature(portable_simd)]
pub mod dotp;
pub mod hadamard;
pub mod helpers;
pub mod m2l;
