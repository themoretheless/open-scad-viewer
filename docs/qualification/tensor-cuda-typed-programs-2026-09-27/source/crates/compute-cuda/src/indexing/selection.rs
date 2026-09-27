use super::{
    CudaScalar,
    dispatch::{CompareDispatch, LoadedCompare, SelectDispatch, SelectInputs},
    output,
};
use crate::{
    CudaError, CudaRuntime, CudaTensor,
    runtime::{layout_metadata, rank},
};
use gpu_compute::cuda::{
    CudaFunction, PushKernelArg,
    cudarc::driver::{DeviceRepr, ValidAsZeroBits},
};
use tensor_core::{Compacted, CompareOp, ScanOptions, Shape, compact_shape};

impl CudaRuntime {
    pub(super) fn compare_typed<T: CudaScalar>(
        &self,
        op: CompareOp,
        a: &CudaTensor<T>,
        b: &CudaTensor<T>,
    ) -> Result<CudaTensor<u32>, CudaError> {
        self.compare_loaded(op, a, b, &self.indexing.compare[T::KIND], None)
    }

    pub(crate) fn compare_loaded<T: DeviceRepr>(
        &self,
        op: CompareOp,
        a: &CudaTensor<T>,
        b: &CudaTensor<T>,
        kernel: &CudaFunction,
        dtype: Option<u32>,
    ) -> Result<CudaTensor<u32>, CudaError> {
        self.check(a)?;
        self.check(b)?;
        let pass = CompareDispatch::new(op, &a.layout, &b.layout)?;
        let mut out = self.zeros_typed(pass.shape.clone())?;
        if pass.shape.is_empty() {
            return Ok(out);
        }
        let metadata = self.metadata(&pass.metadata)?;
        pass.launch(
            self,
            LoadedCompare {
                function: kernel,
                dtype,
            },
            (a.storage.as_ref(), b.storage.as_ref()),
            output(&mut out),
            &metadata,
        )?;
        Ok(out)
    }

    pub(super) fn select_typed<T: CudaScalar>(
        &self,
        mask: &CudaTensor<u32>,
        yes: &CudaTensor<T>,
        no: &CudaTensor<T>,
    ) -> Result<CudaTensor<T>, CudaError> {
        self.select_loaded(mask, yes, no, &self.indexing.select[T::KIND])
    }

    pub(crate) fn select_loaded<T: DeviceRepr + ValidAsZeroBits>(
        &self,
        mask: &CudaTensor<u32>,
        yes: &CudaTensor<T>,
        no: &CudaTensor<T>,
        kernel: &CudaFunction,
    ) -> Result<CudaTensor<T>, CudaError> {
        self.check(mask)?;
        self.check(yes)?;
        self.check(no)?;
        let pass = SelectDispatch::new::<T>(&mask.layout, &yes.layout, &no.layout)?;
        let mut out = self.zeros_typed(pass.shape.clone())?;
        if pass.shape.is_empty() {
            return Ok(out);
        }
        let metadata = self.metadata(&pass.metadata)?;
        pass.launch(
            self,
            kernel,
            SelectInputs {
                mask: mask.storage.as_ref(),
                yes: yes.storage.as_ref(),
                no: no.storage.as_ref(),
            },
            output(&mut out),
            &metadata,
        )?;
        Ok(out)
    }

    pub(super) fn compact_typed<T: CudaScalar>(
        &self,
        input: &CudaTensor<T>,
        mask: &CudaTensor<u32>,
    ) -> Result<Compacted<CudaTensor<T>, CudaTensor<u32>>, CudaError> {
        self.compact_loaded(input, mask, &self.indexing.compact[T::KIND])
    }

    pub(crate) fn compact_loaded<T: DeviceRepr + ValidAsZeroBits>(
        &self,
        input: &CudaTensor<T>,
        mask: &CudaTensor<u32>,
        kernel: &CudaFunction,
    ) -> Result<Compacted<CudaTensor<T>, CudaTensor<u32>>, CudaError> {
        self.check(input)?;
        self.check(mask)?;
        let capacity = compact_shape(input.shape(), mask.shape())?;
        let mask_layout = mask.layout.broadcast_to(input.shape().clone())?;
        let mut values = self.zeros_typed(capacity.clone())?;
        let mut count = self.zeros_typed(Shape::new(vec![])?)?;
        if input.shape().is_empty() {
            return Ok(Compacted { values, count });
        }
        let mut flags = self.zeros_typed::<u32>(capacity)?;
        let metadata = self.metadata(&layout_metadata(&mask_layout))?;
        let (n, r, offset) = (
            input.shape().numel() as u64,
            rank(input.shape())?,
            mask_layout.offset() as u64,
        );
        unsafe {
            self.device
                .stream
                .launch_builder(&self.indexing.normalize_mask)
                .arg(mask.storage.as_ref())
                .arg(output(&mut flags))
                .arg(&metadata)
                .arg(&n)
                .arg(&r)
                .arg(&offset)
                .launch(self.config(n as usize))?;
        }
        let prefix = self.scan_typed(
            &flags,
            0,
            ScanOptions {
                inclusive: false,
                reverse: false,
            },
        )?;
        let metadata = self.metadata(&layout_metadata(&input.layout))?;
        let offset = input.layout.offset() as u64;
        // Flags are 0/1 and capacity fits u32, so prefix destinations are unique
        // and in bounds. Freshly zeroed output retains zero in every tail slot.
        unsafe {
            self.device
                .stream
                .launch_builder(kernel)
                .arg(input.storage.as_ref())
                .arg(flags.storage.as_ref())
                .arg(prefix.storage.as_ref())
                .arg(output(&mut values))
                .arg(output(&mut count))
                .arg(&metadata)
                .arg(&n)
                .arg(&r)
                .arg(&offset)
                .launch(self.config(n as usize))?;
        }
        Ok(Compacted { values, count })
    }
}
