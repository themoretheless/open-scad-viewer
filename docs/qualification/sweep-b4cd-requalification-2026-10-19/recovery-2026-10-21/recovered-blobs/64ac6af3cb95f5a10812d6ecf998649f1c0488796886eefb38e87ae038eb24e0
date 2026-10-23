// Exact 128D descriptor matching for photogrammetry.
// One CUDA block scans one row or column; tie-breaking mirrors the CPU scan.

#include <cuda_runtime.h>

static __device__ __forceinline__ bool better(float a_d, unsigned a_i, float b_d, unsigned b_i) {
    return a_d < b_d || (a_d == b_d && a_i < b_i);
}

static __device__ __forceinline__ float descriptor_dist(
    const float* __restrict__ da,
    const float* __restrict__ db,
    unsigned row,
    unsigned col
) {
    float d = 0.0f;
    unsigned ra = row * 128u;
    unsigned rb = col * 128u;
    #pragma unroll 4
    for (unsigned k = 0; k < 128u; k++) {
        float v = da[ra + k] - db[rb + k];
        d += v * v;
    }
    return d;
}

extern "C" __global__ void match_descriptor_rows(
    unsigned rows,
    unsigned cols,
    const float* __restrict__ da,
    const float* __restrict__ db,
    unsigned* __restrict__ row_j,
    float* __restrict__ row_d1,
    float* __restrict__ row_d2
) {
    __shared__ float sh_d[256];
    __shared__ float sh_s[256];
    __shared__ unsigned sh_i[256];
    unsigned row = blockIdx.x;
    unsigned lid = threadIdx.x;
    float best_d = 3.402823466e38f;
    float second_d = 3.402823466e38f;
    unsigned best_j = 0xffffffffu;
    for (unsigned col = lid; col < cols; col += blockDim.x) {
        float d = descriptor_dist(da, db, row, col);
        if (d < best_d) {
            second_d = best_d;
            best_d = d;
            best_j = col;
        } else if (d < second_d) {
            second_d = d;
        }
    }
    sh_d[lid] = best_d;
    sh_s[lid] = second_d;
    sh_i[lid] = best_j;
    __syncthreads();
    for (unsigned stride = blockDim.x / 2u; stride > 0u; stride /= 2u) {
        if (lid < stride) {
            unsigned other = lid + stride;
            if (better(sh_d[other], sh_i[other], sh_d[lid], sh_i[lid])) {
                sh_s[lid] = fminf(fminf(sh_s[lid], sh_s[other]), sh_d[lid]);
                sh_d[lid] = sh_d[other];
                sh_i[lid] = sh_i[other];
            } else {
                sh_s[lid] = fminf(fminf(sh_s[lid], sh_s[other]), sh_d[other]);
            }
        }
        __syncthreads();
    }
    if (lid == 0u) {
        row_j[row] = sh_i[0];
        row_d1[row] = sh_d[0];
        row_d2[row] = sh_s[0];
    }
}

extern "C" __global__ void match_descriptor_cols(
    unsigned rows,
    unsigned cols,
    const float* __restrict__ da,
    const float* __restrict__ db,
    unsigned* __restrict__ col_i,
    float* __restrict__ col_d1
) {
    __shared__ float sh_d[256];
    __shared__ unsigned sh_i[256];
    unsigned col = blockIdx.x;
    unsigned lid = threadIdx.x;
    float best_d = 3.402823466e38f;
    unsigned best_i = 0xffffffffu;
    for (unsigned row = lid; row < rows; row += blockDim.x) {
        float d = descriptor_dist(da, db, row, col);
        if (d < best_d) {
            best_d = d;
            best_i = row;
        }
    }
    sh_d[lid] = best_d;
    sh_i[lid] = best_i;
    __syncthreads();
    for (unsigned stride = blockDim.x / 2u; stride > 0u; stride /= 2u) {
        if (lid < stride) {
            unsigned other = lid + stride;
            if (better(sh_d[other], sh_i[other], sh_d[lid], sh_i[lid])) {
                sh_d[lid] = sh_d[other];
                sh_i[lid] = sh_i[other];
            }
        }
        __syncthreads();
    }
    if (lid == 0u) {
        col_i[col] = sh_i[0];
        col_d1[col] = sh_d[0];
    }
}
