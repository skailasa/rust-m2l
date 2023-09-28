use std::sync::{Arc, Mutex, RwLock};
use num::{complex::Complex64, Zero};

// Compute the Hadamard product of a sibling set of FFT coefficients (i.e. the multipole expansions)
// With all 16 unique Green kernels corresponding to the unique convolutions.
// This function doesn't do any special optimisations, just implementing the convolutions as a triple
// for loop
pub fn hadamard_product_naive(
    expansion_order: usize,
    sibling_set: &Vec<Arc<Mutex<Vec<Complex64>>>>,
    kernel_data: &RwLock<Vec<Complex64>>,
) -> Vec<Complex64> {
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size_real = p * q * (r / 2 + 1);

    let mut res = vec![Complex64::zero(); size_real * 16 * 8];

    for i in 0..16 {
        let m2l_matrix_offset = i * size_real;

        // Loading this into cache is the most expensive operation.
        let m2l_matrix =
            &kernel_data.read().unwrap()[m2l_matrix_offset..m2l_matrix_offset + size_real];

        for k in 0..8 {
            let signal = sibling_set[k].lock().unwrap();

            for j in 0..size_real {
                res[k * size_real * 16 + i * size_real + j] += signal[j] * m2l_matrix[j];
            }
        }
    }
    res
}

#[cfg(all(target_arch = "x86", feature = "avx"))]
pub mod x86 {
    use super::*;
    use std::arch::x86_64::*;

    // Compute the Hadamard product of a sibling set of FFT coefficients (i.e. the multipole expansions)
    // With all 16 unique Green kernels corresponding to the unique convolutions.
    // This function uses explicit SIMD to fetch and compute the component wise product of the complex
    // numbers corresponding to the FFT outputs.
    pub fn hadamard_product_simd(
        expansion_order: usize,
        sibling_set: &Vec<Arc<Mutex<Vec<Complex64>>>>,
        kernel_data: &RwLock<Vec<Complex64>>,
    ) -> Vec<Complex64> {
        let n = 2 * expansion_order - 1;
        let &(m, n, o) = &(n, n, n);

        let p = m + 1;
        let q = n + 1;
        let r = o + 1;
        let size_real = p * q * (r / 2 + 1);

        let mut res = vec![Complex64::zero(); size_real * 16 * 8];

        for i in 0..16 {
            let m2l_matrix_offset = i * size_real;

            // Loading this into cache is the most expensive operation.
            let m2l_matrix =
                &kernel_data.read().unwrap()[m2l_matrix_offset..m2l_matrix_offset + size_real];

            // Instead of storing in a temporary buffer to scatter later, there should be a way of directly
            // loading the ifft data structure into a SIMD register here and directly saving the convolutions
            // as they are computed and already held in SIMD registers. Then we will have a very similar memory
            // access pattern to PVFMM and should not have to do the scatter operation as an additional step.
            for k in 0..8 {
                let signal = sibling_set[k].lock().unwrap();
                let res_offset = k * size_real * 16 + i * size_real;

                let chunk_size = 2;
                let chunks = size_real / chunk_size;

                for j in 0..chunks {
                    let simd_index = j * chunk_size;
                    unsafe {
                        let complex_ref = &signal[simd_index];
                        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
                        let ptr = tuple_ptr as *const f64;
                        let signal_chunk = _mm256_loadu_pd(ptr);

                        let complex_ref = &m2l_matrix[simd_index];
                        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
                        let ptr = tuple_ptr as *const f64;
                        let kernel_chunk = _mm256_loadu_pd(ptr);

                        let complex_ref = &res[simd_index];
                        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
                        let ptr = tuple_ptr as *const f64;
                        let res_chunk = _mm256_loadu_pd(ptr);

                        // Find component wise product, add with what's already there
                        let product = hadamard_product_kernel_avx2(signal_chunk, kernel_chunk);
                        let tmp = _mm256_add_pd(product, res_chunk);

                        // Save
                        let complex_ref = &res[simd_index];
                        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
                        let ptr = tuple_ptr as *mut f64;

                        _mm256_storeu_pd(ptr, tmp);
                    }
                }

                // Handle remainder
                let start_remainder = chunks * chunk_size;
                for j in start_remainder..size_real {
                    res[res_offset + j] += signal[j] * m2l_matrix[j];
                }
            }
        }

        res
    }

// The SIMD kernel for computing the component wise product of two complex numbers loaded into
// SIMD registers a and b respectively. Optimised for AVX 2 256-bit wide registers
pub fn hadamard_product_kernel_avx2(a_ra: __m256d, b_ra: __m256d) -> __m256d {
    unsafe {
        // Extract real parts [a1, a1, a2, a2]
        let a_real = _mm256_shuffle_pd(a_ra, a_ra, 0b0);

        // Extract imaginary parts [b1, b1, b2, b2]
        let a_imag = _mm256_shuffle_pd(a_ra, a_ra, 0b1111);

        // Multiply real parts [a1c1, a1d1, a2c2, a2d2]
        let real_mul = _mm256_mul_pd(a_real, b_ra);

        // Multiply imag parts [b1c1, b1d1, b2c2, b2d2]
        let imag_mul = _mm256_mul_pd(a_imag, b_ra);

        // Find components that go in the real and imag part of solution
        // [a1c1, b1d1, a2c2, b2d2]
        let real_part = _mm256_blend_pd(real_mul, imag_mul, 0b1010);
        // [b1c1, a1d1, b2c2, a2d2]
        let imag_part = _mm256_blend_pd(real_mul, imag_mul, 0b0101);

        // [a1c1-b1d1, a1c1-b1d1, a2c2-b2d2, a2c2-b2d2]
        let real = _mm256_hsub_pd(real_part, real_part);
        let imag = _mm256_hadd_pd(imag_part, imag_part);

        let result = _mm256_blend_pd(real, imag, 0b1010);

        result
    }
    }
}



#[cfg(all(target_arch = "aarch64", feature = "neon"))]
pub mod aarch64 {
    use super::*;
    use std::arch::aarch64::*;


    pub fn hadamard_product_simd_neon(
        expansion_order: usize, 
        sibling_set: &Vec<Arc<Mutex<Vec<Complex64>>>>, 
        kernel_data: &RwLock<Vec<Complex64>>
    ) -> Vec<Complex64> {
        
        let n = 2 * expansion_order - 1;
        let &(m, n, o) = &(n, n, n);

        let p = m + 1;
        let q = n + 1;
        let r = o + 1;
        let size_real = p * q * (r / 2 + 1);

        let res = vec![Complex64::zero(); size_real * 16 * 8];

        for i in 0..16 {
            let m2l_matrix_offset = i * size_real;

            // Loading this into cache is the most expensive operation.
            let m2l_matrix =
                &kernel_data.read().unwrap()[m2l_matrix_offset..m2l_matrix_offset + size_real];

            // Instead of storing in a temporary buffer to scatter later, there should be a way of directly
            // loading the ifft data structure into a SIMD register here and directly saving the convolutions
            // as they are computed and already held in SIMD registers. Then we will have a very similar memory
            // access pattern to PVFMM and should not have to do the scatter operation as an additional step.
            for k in 0..8 {
                let signal = sibling_set[k].lock().unwrap();
                
                let chunks = size_real;

                for j in 0..chunks {
                    let simd_index = j;
                    unsafe {
                        let complex_ref = &signal[simd_index];
                        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
                        let ptr = tuple_ptr as *const f64;
                        let signal_chunk = vld1q_f64(ptr); 

                        let complex_ref = &m2l_matrix[simd_index];
                        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
                        let ptr = tuple_ptr as *const f64;
                        let kernel_chunk = vld1q_f64(ptr);

                        let complex_ref = &res[simd_index];
                        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
                        let ptr = tuple_ptr as *const f64;
                        let res_chunk = vld1q_f64(ptr);

                        // Find component wise product, add with what's already there
                        let product = hadamard_product_kernel_neon(signal_chunk, kernel_chunk);
                        let tmp = vaddq_f64(product, res_chunk);

                        // Save
                        let complex_ref = &res[simd_index];
                        let tuple_ptr: *const (f64, f64) = complex_ref as *const _ as *const (f64, f64);
                        let ptr = tuple_ptr as *mut f64;

                        vst1q_f64(ptr, tmp)
                    }
                }
            }
        }

        res

    }
    
    pub fn hadamard_product_kernel_neon(a_ra: float64x2_t, b_ra: float64x2_t) -> float64x2_t {
        unsafe {
            // Extract real parts [a1, a1]
            let a_real = vdupq_lane_f64(vget_low_f64(a_ra), 0);
            
            // Extract imaginary parts [b1, b1]
            let a_imag = vdupq_lane_f64(vget_high_f64(a_ra), 0);

            // Multiply real parts [a1c1, a1d1]
            let real_mul = vmulq_f64(a_real, b_ra);

            // Multiply imag parts [b1c1, b1d1]
            let imag_mul = vmulq_f64(a_imag, b_ra);

            // Construct results for real and imaginary parts
            let real =  vsub_f64(vget_low_f64(real_mul), vget_high_f64(imag_mul));
            let imag =  vadd_f64(vget_high_f64(real_mul), vget_low_f64(imag_mul));

            let result = vcombine_f64(real, imag);

            result
        }
    }

   
    
}
