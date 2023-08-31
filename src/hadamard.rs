use std::{
    arch::x86_64::*,
    sync::{Arc, Mutex, RwLock}
};

pub fn hadamard_kernel(kernel: &[f64], signal: &[f64], result: &mut [f64]) {
    // assert!(kernel.len() == 8);
    // assert!(signal.len() == 8);
    // assert!(result.len() == 8);

    unsafe {
        // Load the first half (4 elements) of kernel and signal into YMM registers
        let kernel_vec1 = _mm256_loadu_pd(&kernel[0]);
        let signal_vec1 = _mm256_loadu_pd(&signal[0]);

        // Compute the Hadamard product for the first half using FMA
        let res_vec1 = _mm256_fmadd_pd(kernel_vec1, signal_vec1, _mm256_setzero_pd());

        // Store the result back into the result array
        _mm256_storeu_pd(&mut result[0], res_vec1);

        // Repeat the same for the second half of the slices
        let kernel_vec2 = _mm256_loadu_pd(&kernel[4]);
        let signal_vec2 = _mm256_loadu_pd(&signal[4]);

        let res_vec2 = _mm256_fmadd_pd(kernel_vec2, signal_vec2, _mm256_setzero_pd());

        _mm256_storeu_pd(&mut result[4], res_vec2);
    }
}


pub fn hadamard_product_naive(sibling_set: &Vec<Arc<Mutex<Vec<f64>>>>, kernel_data: &RwLock<Vec<f64>>) {
    let expansion_order: usize = 9;
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size = p * q * r;
    let size_real = p * q * (r / 2 + 1);


    let mut res = vec![0f64; size_real*16*8];


    for i in 0..16 {


        let m2l_matrix_offset = i*size_real;


        // Loading this into cache is the most expensive operation.
        let m2l_matrix = &kernel_data.read().unwrap()[m2l_matrix_offset..m2l_matrix_offset+size_real];
        // println!("HERE {:?}", m2l_matrix.len());


        for k in 0..8 {
            let signal = sibling_set[k].lock().unwrap();
            let res_offset = k * size_real * 16 + i * size_real;

            let chunk_size = 4;
            let chunks = size_real / chunk_size;



            for j in 0..chunks {
                let simd_index = j * chunk_size;
                unsafe {
                    let signal_chunk = _mm256_loadu_pd(&signal[simd_index]);
                    let kernel_chunk = _mm256_loadu_pd(&m2l_matrix[simd_index]);
                    let res_chunk = _mm256_loadu_pd(&res[simd_index]);

                    let product = _mm256_mul_pd(signal_chunk, kernel_chunk);

                    let tmp = _mm256_add_pd(product, res_chunk);

                    // let tmp = _mm256_fmadd_pd(signal_chunk, kernel_chunk, res_chunk);
                    _mm256_storeu_pd(&mut res[simd_index], tmp);

                }
            }


            let start_remainder = chunks * chunk_size;
            for j in start_remainder..size_real {
                res[res_offset + j] += signal[j] * m2l_matrix[j];
            }

            // for j in 0..size_real {
            //     res[k*size_real*16+i*size_real+j] += signal[j]*m2l_matrix[j];
            // }
        }
    }

    // println!("Hadamard product {:?}", s.elapsed());

}

pub fn hadamard_product_simd<'a>(sibling_set: &Vec<Arc<Mutex<Vec<f64>>>>, kernel_data: &Vec<f64>) {
    let expansion_order: usize = 9;
    let n = 2 * expansion_order - 1;
    let &(m, n, o) = &(n, n, n);

    let p = m + 1;
    let q = n + 1;
    let r = o + 1;
    let size = p * q * r;
    let size_real = p * q * (r / 2 + 1);

    // Receiving a vec of sibling coefficients, need to go through by frequency so need to re
    // allocate so that they're arranged by frequency

    // let s = Instant::now();
    // let signal_data = transpose::<f64>(sibling_set);
    // println!("Signal Transpose {:?}", s.elapsed().as_millis());

    // Result buffer
    let mut result = vec![0f64; 16*size_real*8];

    // Iterate over frequencies

    // // let s = Instant::now();
    // for k in 0..size_real {
    //     // Get out a set of 8 convolutions at the kth frequency
    //     // Multiply with kth frequency components of sibling set
    //     let signal = &signal_data[(k*8)..(k*8)+8];
    //     let kernel = &kernel_data[k*16..(k*16)+16];

    //     // for i in 0..2 {
    //     //     // let res = &mut result[(k*128)+(i * 64)..(k*128)+(i * 64) + 64];

    //     //     // res
    //     //     //     .chunks_exact_mut(8)
    //     //     //     .zip(kernel.chunks_exact(8))
    //     //     //     .for_each(|(r, k)| {

    //     //     //         // Chunk of size 8 64 bits won't fit into avx2 register, need compute half of
    //     //     //         // chunk at a time.
    //     //     //         hadamard_kernel(k, signal,  r);

    //     //     //     })
    //     // }
    // }
    // println!("Hadamard product {:?}", s.elapsed());
}
