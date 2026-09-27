use crate::{CudaError, CudaMatmulPolicy, CudaRuntime, CudaTensor};
use gpu_compute::cuda::{
    CudaSlice, CudaStream,
    cudarc::{
        cublas::{CudaBlas, result, sys},
        driver::{DevicePtr, DevicePtrMut, DeviceRepr},
    },
};
use std::sync::Arc;
use tensor_core::{MatmulPlan, MatmulPrecision, TensorBackend};

mod plan;
pub(crate) use plan::GemmPlan;

/// A checked descriptor and numerical mode retained on its preparation stream.
/// Preparation initializes cuBLAS; replay only binds the current allocations.
#[derive(Clone, Debug)]
pub(crate) struct PreparedGemm {
    plan: GemmPlan,
    input_type: sys::cudaDataType_t,
    compute_type: sys::cublasComputeType_t,
    stream: Arc<CudaStream>,
}
impl PreparedGemm {
    pub(crate) fn plan(&self) -> &GemmPlan {
        &self.plan
    }
}

impl CudaRuntime {
    pub fn matmul_policy(&self, precision: MatmulPrecision) -> Result<CudaMatmulPolicy, CudaError> {
        CudaMatmulPolicy::for_device(
            precision,
            self.capabilities.compute_capability,
            std::env::var("NVIDIA_TF32_OVERRIDE").ok().as_deref(),
        )
    }
    pub(crate) fn matmul_impl(
        &self,
        a: &CudaTensor,
        b: &CudaTensor,
        precision: MatmulPrecision,
    ) -> Result<CudaTensor, CudaError> {
        self.check(a)?;
        self.check(b)?;
        let shape = MatmulPlan::new(a.shape(), b.shape())?;
        // Preserve policy validation before empty shortcuts or materialization.
        self.matmul_policy(precision)?;
        if shape.matrix_output.is_empty() || *shape.left.dims().last().unwrap() == 0 {
            return self.zeros(shape.output);
        }
        let a = self.reshape(a, shape.left)?;
        let b = self.reshape(b, shape.right)?;
        let plan = GemmPlan::new(&a.layout, &b.layout)?;
        let prepared = self.prepare_gemm_f32(plan, precision)?;
        let mut output = self.zeros(prepared.plan().output().clone())?;
        self.launch_gemm_f32(
            &prepared,
            a.storage.as_ref(),
            b.storage.as_ref(),
            Arc::get_mut(&mut output.storage).unwrap(),
        )?;
        self.view(&output, output.layout.reshape(shape.output)?)
    }

    /// The executor materializes and rank-promotes operands in its own schedule.
    /// This step checks precision even for empty/K=0 plans. Nonempty work obtains
    /// the cuBLAS handle before any recorded replay can occur.
    pub(crate) fn prepare_gemm_f32(
        &self,
        plan: GemmPlan,
        precision: MatmulPrecision,
    ) -> Result<PreparedGemm, CudaError> {
        let policy = self.matmul_policy(precision)?;
        self.prepare_gemm(plan, sys::cudaDataType_t::CUDA_R_32F, policy.compute_type)
    }

    /// No explicit device allocation, metadata upload or cuBLAS initialization.
    /// The caller validates runtime ownership and output aliases before enqueue.
    /// A zero plan does not write: its nonempty output needs a scheduled fill.
    pub(crate) fn launch_gemm_f32(
        &self,
        prepared: &PreparedGemm,
        left: &CudaSlice<f32>,
        right: &CudaSlice<f32>,
        output: &mut CudaSlice<f32>,
    ) -> Result<(), CudaError> {
        self.launch_gemm(prepared, left, right, output)
    }

    /// Common eager wrapper for native two-byte and f32 storage. The cuBLAS
    /// launch implementation is shared with prepared f32 programs.
    pub(crate) fn gemm_dense_f32<T: DeviceRepr>(
        &self,
        a: &CudaTensor<T>,
        b: &CudaTensor<T>,
        input_type: sys::cudaDataType_t,
        compute_type: sys::cublasComputeType_t,
    ) -> Result<CudaTensor, CudaError> {
        self.check(a)?;
        self.check(b)?;
        validate_input_width::<T>(input_type)?;
        let plan = GemmPlan::new(&a.layout, &b.layout)?;
        let prepared = self.prepare_gemm(plan, input_type, compute_type)?;
        let mut output = self.zeros(prepared.plan().output().clone())?;
        self.launch_gemm(
            &prepared,
            a.storage.as_ref(),
            b.storage.as_ref(),
            Arc::get_mut(&mut output.storage).unwrap(),
        )?;
        Ok(output)
    }

    fn prepare_gemm(
        &self,
        plan: GemmPlan,
        input_type: sys::cudaDataType_t,
        compute_type: sys::cublasComputeType_t,
    ) -> Result<PreparedGemm, CudaError> {
        if !plan.is_zero() && self.blas.get().is_none() {
            crate::libraries::require_cublas()?;
            let blas = CudaBlas::new(self.device.stream.clone())
                .map_err(|e| CudaError::Blas(e.to_string()))?;
            let _ = self.blas.set(blas);
        }
        Ok(PreparedGemm {
            plan,
            input_type,
            compute_type,
            stream: self.device.stream.clone(),
        })
    }

    fn launch_gemm<T: DeviceRepr>(
        &self,
        prepared: &PreparedGemm,
        left: &CudaSlice<T>,
        right: &CudaSlice<T>,
        output: &mut CudaSlice<f32>,
    ) -> Result<(), CudaError> {
        validate_input_width::<T>(prepared.input_type)?;
        if prepared.stream != self.device.stream
            || left.context() != &self.device.context
            || right.context() != &self.device.context
            || output.context() != &self.device.context
        {
            return Err(CudaError::InvalidInput(
                "prepared GEMM stream and storage context must match",
            ));
        }
        prepared
            .plan
            .validate_storage(left.len(), right.len(), output.len())?;
        let Some(dimensions) = prepared.plan.dimensions() else {
            return Ok(());
        };
        let blas = self.blas.get().ok_or(CudaError::InvalidInput(
            "GEMM was not prepared on this runtime",
        ))?;
        self.device.context.bind_to_thread()?;
        let alpha = 1f32;
        let beta = 0f32;
        for index in 0..prepared.plan.calls() {
            let batch = prepared.plan.batch(index)?;
            let a = left.slice(batch.left);
            let b = right.slice(batch.right);
            let mut out = output.slice_mut(batch.output);
            let (a_ptr, _a_guard) = a.device_ptr(&self.device.stream);
            let (b_ptr, _b_guard) = b.device_ptr(&self.device.stream);
            let (out_ptr, _out_guard) = out.device_ptr_mut(&self.device.stream);
            // The plan proves every view bound before the first call. Row-major
            // C=A*B is C^T=B^T*A^T in cuBLAS's column-major convention. Retained
            // guards register accesses on the same stream as eager execution.
            unsafe {
                result::gemm_ex(
                    *blas.handle(),
                    sys::cublasOperation_t::CUBLAS_OP_N,
                    sys::cublasOperation_t::CUBLAS_OP_N,
                    dimensions.m,
                    dimensions.n,
                    dimensions.k,
                    (&alpha as *const f32).cast(),
                    b_ptr as *const _,
                    prepared.input_type,
                    dimensions.lda,
                    a_ptr as *const _,
                    prepared.input_type,
                    dimensions.ldb,
                    (&beta as *const f32).cast(),
                    out_ptr as *mut _,
                    sys::cudaDataType_t::CUDA_R_32F,
                    dimensions.ldc,
                    prepared.compute_type,
                    sys::cublasGemmAlgo_t::CUBLAS_GEMM_DEFAULT,
                )
                .map_err(|e| CudaError::Blas(e.to_string()))?;
            }
        }
        Ok(())
    }
}
fn validate_input_width<T>(input_type: sys::cudaDataType_t) -> Result<(), CudaError> {
    let width = match input_type {
        sys::cudaDataType_t::CUDA_R_32F => 4,
        sys::cudaDataType_t::CUDA_R_16F | sys::cudaDataType_t::CUDA_R_16BF => 2,
        _ => return Err(CudaError::InvalidInput("unsupported GEMM input type")),
    };
    if std::mem::size_of::<T>() != width {
        return Err(CudaError::InvalidInput(
            "GEMM storage width does not match its declared type",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gemm_storage_types_match_the_cublas_abi() {
        use sys::cudaDataType_t::*;
        assert!(validate_input_width::<f32>(CUDA_R_32F).is_ok());
        assert!(validate_input_width::<u16>(CUDA_R_16F).is_ok());
        assert!(validate_input_width::<u16>(CUDA_R_16BF).is_ok());
        assert!(validate_input_width::<f32>(CUDA_R_16BF).is_err());
        assert!(validate_input_width::<u16>(CUDA_R_32F).is_err());
        assert!(validate_input_width::<f32>(CUDA_R_32I).is_err());
    }
}
