
use std::simd::*;

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
use std::arc::x86_64::*;

use rayon::prelude::*;

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
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

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
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

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
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

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
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

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
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

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
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

