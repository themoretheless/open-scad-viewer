//! Resident f32/u32 tensors on an explicit CUDA stream.
//!
//! Custom kernels use NVRTC at initialization (or caller-supplied compatible
//! PTX). Matrix multiplication uses cuBLAS with an explicit precision policy.
//! No operation falls back to the CPU. CUDA execution must be qualified on
//! NVIDIA hardware; successful compilation on another platform is not GPU proof.
#![doc = include_str!("../README.md")]
mod attention;
mod dispatch;
mod error;
mod indexing;
mod libraries;
mod low_attention;
mod low_indexing;
mod low_ops;
mod low_precision;
mod low_scatter;
mod low_statistics;
mod matmul;
mod policy;
mod program;
mod reduction;
mod runtime;
mod statistics;

pub use error::CudaError;
pub use low_precision::CudaLowTensor;
pub use policy::CudaMatmulPolicy;
pub use runtime::{CUDA_KERNEL_SOURCE, CudaCapabilities, CudaRuntime, CudaTensor};
pub use tensor_core;

pub use program::{
    CudaPrepareOptions, CudaPreparedProgram, CudaProgramBuilder, CudaProgramStats, CudaValue,
};
