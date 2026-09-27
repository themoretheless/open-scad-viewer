//! Typed indexing shares layout validation and traversal across storage formats.
mod gather;
mod scan;
mod scatter;
mod selection;

use crate::{
    CudaError, CudaRuntime, CudaTensor,
    runtime::{layout_metadata, rank},
};
use gpu_compute::cuda::{
    CudaFunction, CudaModule, CudaSlice, PushKernelArg,
    cudarc::driver::{DeviceRepr, ValidAsZeroBits},
};
use std::sync::Arc;
use tensor_core::{
    Compacted, CompareOp, Gathered, ScanOptions, Shape, TensorBackend, TensorError,
    TensorIndexBackend,
};

pub(crate) trait CudaScalar: DeviceRepr + ValidAsZeroBits + Copy + Default {
    const KIND: usize;
    const ONE: Self;
    fn materialize(
        runtime: &CudaRuntime,
        input: &CudaTensor<Self>,
    ) -> Result<CudaTensor<Self>, CudaError>;
}
impl CudaScalar for f32 {
    const KIND: usize = 0;
    const ONE: Self = 1.;
    fn materialize(
        runtime: &CudaRuntime,
        input: &CudaTensor<Self>,
    ) -> Result<CudaTensor<Self>, CudaError> {
        runtime.materialize(input)
    }
}
impl CudaScalar for u32 {
    const KIND: usize = 1;
    const ONE: Self = 1;
    fn materialize(
        runtime: &CudaRuntime,
        input: &CudaTensor<Self>,
    ) -> Result<CudaTensor<Self>, CudaError> {
        runtime.materialize_u32(input)
    }
}

pub(crate) struct IndexKernels {
    copy: [CudaFunction; 2],
    compare: [CudaFunction; 2],
    select: [CudaFunction; 2],
    scan: [CudaFunction; 2],
    add_scan: [CudaFunction; 2],
    gather: [CudaFunction; 2],
    compact: [CudaFunction; 2],
    invalid_indices: CudaFunction,
    normalize_mask: CudaFunction,
    scatter_owners: CudaFunction,
    scatter: [CudaFunction; 2],
}
impl IndexKernels {
    pub(crate) fn load(module: &Arc<CudaModule>) -> Result<Self, CudaError> {
        let pair = |f: &str, u: &str| -> Result<_, CudaError> {
            Ok([module.load_function(f)?, module.load_function(u)?])
        };
        Ok(Self {
            copy: pair("copy_f32", "copy_u32")?,
            compare: pair("compare_f32", "compare_u32")?,
            select: pair("where_f32", "where_u32")?,
            scan: pair("scan_f32", "scan_u32")?,
            add_scan: pair("add_scan_f32", "add_scan_u32")?,
            gather: pair("gather_f32", "gather_u32")?,
            compact: pair("compact_f32", "compact_u32")?,
            invalid_indices: module.load_function("invalid_indices")?,
            normalize_mask: module.load_function("normalize_mask")?,
            scatter_owners: module.load_function("scatter_owners")?,
            scatter: pair("scatter_f32", "scatter_u32")?,
        })
    }
}

pub(crate) fn output<T>(tensor: &mut CudaTensor<T>) -> &mut CudaSlice<T> {
    // Every caller just allocated this output and has not exposed a view.
    Arc::get_mut(&mut tensor.storage).unwrap()
}

impl CudaRuntime {
    /// Always allocates distinct contiguous storage, including for dense input.
    pub(crate) fn copy_typed<T: CudaScalar>(
        &self,
        tensor: &CudaTensor<T>,
    ) -> Result<CudaTensor<T>, CudaError> {
        self.check(tensor)?;
        let mut out = self.zeros_typed(tensor.shape().clone())?;
        if tensor.shape().is_empty() {
            return Ok(out);
        }
        let metadata = self.metadata(&layout_metadata(&tensor.layout))?;
        let (n, r, offset) = (
            tensor.shape().numel() as u64,
            rank(tensor.shape())?,
            tensor.layout.offset() as u64,
        );
        // Validated source layout, distinct contiguous output, matching typed ABI.
        unsafe {
            self.device
                .stream
                .launch_builder(&self.indexing.copy[T::KIND])
                .arg(tensor.storage.as_ref())
                .arg(output(&mut out))
                .arg(&metadata)
                .arg(&n)
                .arg(&r)
                .arg(&offset)
                .launch(self.config(n as usize))?;
        }
        Ok(out)
    }
    /// Updates unique contiguous u32 storage. Live views reject mutation.
    pub fn write_u32(&self, tensor: &mut CudaTensor<u32>, values: &[u32]) -> Result<(), CudaError> {
        self.check(tensor)?;
        if values.len() != tensor.shape().numel() {
            return Err(TensorError::ElementCountMismatch {
                expected: tensor.shape().numel(),
                actual: values.len(),
            }
            .into());
        }
        if !tensor.layout.is_contiguous() || tensor.layout.offset() != 0 {
            return Err(CudaError::SharedOutput);
        }
        let storage = Arc::get_mut(&mut tensor.storage).ok_or(CudaError::SharedOutput)?;
        if !values.is_empty() {
            self.device.stream.memcpy_htod(values, storage)?;
        }
        Ok(())
    }
}

impl TensorIndexBackend for CudaRuntime {
    type UIntTensor = CudaTensor<u32>;

    fn upload_u32(&self, shape: Shape, values: &[u32]) -> Result<Self::UIntTensor, CudaError> {
        if shape.numel() != values.len() {
            return Err(TensorError::ElementCountMismatch {
                expected: shape.numel(),
                actual: values.len(),
            }
            .into());
        }
        let mut tensor = self.zeros_typed(shape)?;
        self.write_u32(&mut tensor, values)?;
        Ok(tensor)
    }
    fn read_u32(&self, tensor: &Self::UIntTensor) -> Result<Vec<u32>, CudaError> {
        self.check(tensor)?;
        if tensor.shape().is_empty() {
            return Ok(Vec::new());
        }
        let tensor = self.materialize_u32(tensor)?;
        let start = tensor.layout.offset();
        Ok(self
            .device
            .stream
            .clone_dtoh(&tensor.storage.slice(start..start + tensor.shape().numel()))?)
    }
    fn materialize_u32(&self, tensor: &Self::UIntTensor) -> Result<Self::UIntTensor, CudaError> {
        self.check(tensor)?;
        if tensor.layout.is_contiguous() {
            return Ok(tensor.clone());
        }
        self.copy_typed(tensor)
    }
    fn reshape_u32(
        &self,
        tensor: &Self::UIntTensor,
        shape: Shape,
    ) -> Result<Self::UIntTensor, CudaError> {
        self.check(tensor)?;
        if shape.numel() != tensor.shape().numel() {
            return Err(TensorError::ElementCountMismatch {
                expected: tensor.shape().numel(),
                actual: shape.numel(),
            }
            .into());
        }
        let tensor = self.materialize_u32(tensor)?;
        self.view(&tensor, tensor.layout.reshape(shape)?)
    }
    fn permute_u32(
        &self,
        tensor: &Self::UIntTensor,
        axes: &[usize],
    ) -> Result<Self::UIntTensor, CudaError> {
        self.view(tensor, tensor.layout.permute(axes)?)
    }
    fn broadcast_u32(
        &self,
        tensor: &Self::UIntTensor,
        shape: Shape,
    ) -> Result<Self::UIntTensor, CudaError> {
        self.view(tensor, tensor.layout.broadcast_to(shape)?)
    }
    fn compare(
        &self,
        op: CompareOp,
        left: &CudaTensor,
        right: &CudaTensor,
    ) -> Result<Self::UIntTensor, CudaError> {
        self.compare_typed(op, left, right)
    }
    fn compare_u32(
        &self,
        op: CompareOp,
        left: &Self::UIntTensor,
        right: &Self::UIntTensor,
    ) -> Result<Self::UIntTensor, CudaError> {
        self.compare_typed(op, left, right)
    }
    fn select_f32(
        &self,
        mask: &Self::UIntTensor,
        yes: &CudaTensor,
        no: &CudaTensor,
    ) -> Result<CudaTensor, CudaError> {
        self.select_typed(mask, yes, no)
    }
    fn select_u32(
        &self,
        mask: &Self::UIntTensor,
        yes: &Self::UIntTensor,
        no: &Self::UIntTensor,
    ) -> Result<Self::UIntTensor, CudaError> {
        self.select_typed(mask, yes, no)
    }
    fn scan_f32(
        &self,
        input: &CudaTensor,
        axis: usize,
        options: ScanOptions,
    ) -> Result<CudaTensor, CudaError> {
        self.scan_typed(input, axis, options)
    }
    fn scan_u32(
        &self,
        input: &Self::UIntTensor,
        axis: usize,
        options: ScanOptions,
    ) -> Result<Self::UIntTensor, CudaError> {
        self.scan_typed(input, axis, options)
    }
    fn gather_f32(
        &self,
        input: &CudaTensor,
        indices: &Self::UIntTensor,
        axis: usize,
    ) -> Result<Gathered<CudaTensor, Self::UIntTensor>, CudaError> {
        self.gather_typed(input, indices, axis)
    }
    fn gather_u32(
        &self,
        input: &Self::UIntTensor,
        indices: &Self::UIntTensor,
        axis: usize,
    ) -> Result<Gathered<Self::UIntTensor, Self::UIntTensor>, CudaError> {
        self.gather_typed(input, indices, axis)
    }
    fn compact_f32(
        &self,
        input: &CudaTensor,
        mask: &Self::UIntTensor,
    ) -> Result<Compacted<CudaTensor, Self::UIntTensor>, CudaError> {
        self.compact_typed(input, mask)
    }
    fn compact_u32(
        &self,
        input: &Self::UIntTensor,
        mask: &Self::UIntTensor,
    ) -> Result<Compacted<Self::UIntTensor, Self::UIntTensor>, CudaError> {
        self.compact_typed(input, mask)
    }
}
