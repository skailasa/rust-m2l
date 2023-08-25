#![feature(stdsimd)]
#[macro_use]
extern crate ispc;
ispc_module!(ispdotc);

use std::arch::x86_64::*;

use rand::Rng;

pub const BLOCK_SIZE: usize = 1024;

pub fn data_f64(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {


    let mut rng = rand::thread_rng();

    let size: usize = BLOCK_SIZE*n;

    let x: Vec<f64> = (0..size).map(|_| rng.gen()).collect();
    let y: Vec<f64> = (0..size).map(|_| rng.gen()).collect();
    let z = vec![0f64; size];

    (
        x, y, z
    )
}

pub fn data_f32(n: usize) -> (Vec<f32>, Vec<f32>, Vec<f32>) {


    let mut rng = rand::thread_rng();

    let size: usize = BLOCK_SIZE*n;

    let x: Vec<f32> = (0..size).map(|_| rng.gen()).collect();
    let y: Vec<f32> = (0..size).map(|_| rng.gen()).collect();
    let z = vec![0f32; size];

    (
        x, y, z
    )
}

pub fn dotp_naive_f64(x: &[f64], y: &[f64], z: &mut [f64]) {
    for ((a,b), c) in x.iter().zip(y.iter()).zip(z.iter_mut()) {
        *c += a * b;
    }
}

pub fn dotp_naive_f32(x: &[f32], y: &[f32], z: &mut [f32]) {
    for ((a,b), c) in x.iter().zip(y.iter()).zip(z.iter_mut()) {
        *c += a * b;
    }
}

#[inline(never)]
pub fn dotp_simd_f64(x: &[f64], y: &[f64], z: &mut [f64]) {

    for ((a, b), c) in x
        .chunks_exact(4)
        .zip(y.chunks_exact(4))
        .zip(z.chunks_exact_mut(4)) {

            unsafe {
                let x_a = _mm256_loadu_pd(a.as_ptr());
                let y_a = _mm256_loadu_pd(b.as_ptr());
                let r_a = _mm256_loadu_pd(c.as_ptr());

                _mm256_storeu_pd(c.as_mut_ptr(), _mm256_fmadd_pd(x_a, y_a, r_a));

            }
        }

}


#[inline(never)]
pub fn dotp_simd_f32(x: &[f32], y: &[f32], z: &mut [f32]) {

    for ((a, b), c) in x
        .chunks_exact(8)
        .zip(y.chunks_exact(8))
        .zip(z.chunks_exact_mut(8)) {

            unsafe {
                let x_a = _mm256_loadu_ps(a.as_ptr());
                let y_a = _mm256_loadu_ps(b.as_ptr());
                let r_a = _mm256_loadu_ps(c.as_ptr());

                _mm256_storeu_ps(c.as_mut_ptr(), _mm256_fmadd_ps(x_a, y_a, r_a));

            }
        }

}

#[inline(never)]
pub fn dotp_no_simd_bounds_f32(x: &[f32], y: &[f32], z: &mut [f32]) {
    for idx in 0..1024 / 8 {
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