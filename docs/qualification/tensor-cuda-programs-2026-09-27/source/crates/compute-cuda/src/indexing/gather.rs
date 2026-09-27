use super::{CudaScalar, output};
use crate::{
    CudaError, CudaRuntime, CudaTensor,
    runtime::{layout_metadata, rank},
};
use gpu_compute::cuda::{
    CudaFunction, PushKernelArg,
    cudarc::driver::{DeviceRepr, ValidAsZeroBits},
};
use tensor_core::{Gathered, Shape, gather_shape, validate_index_count};

impl CudaRuntime {
    pub(super) fn invalid_index_count(
        &self,
        indices: &CudaTensor<u32>,
        length: usize,
    ) -> Result<CudaTensor<u32>, CudaError> {
        self.check(indices)?;
        validate_index_count(indices.shape())?;
        let mut result = self.zeros_typed(Shape::new(vec![])?)?;
        let count = indices.shape().numel() as u64;
        if count == 0 {
            return Ok(result);
        }
        let index_rank = rank(indices.shape())?;
        let offset = indices.layout.offset() as u64;
        let length = length as u64;
        let metadata = self.metadata(&layout_metadata(&indices.layout))?;
        // Count original logical indices even if their gathered/scattered
        // output is empty. The shared contract bounds the scalar count to u32.
        unsafe {
            self.device
                .stream
                .launch_builder(&self.indexing.invalid_indices)
                .arg(indices.storage.as_ref())
                .arg(output(&mut result))
                .arg(&metadata)
                .arg(&count)
                .arg(&index_rank)
                .arg(&offset)
                .arg(&length)
                .launch(self.config(count as usize))?;
        }
        Ok(result)
    }

    pub(super) fn gather_typed<T: CudaScalar>(
        &self,
        input: &CudaTensor<T>,
        indices: &CudaTensor<u32>,
        axis: usize,
    ) -> Result<Gathered<CudaTensor<T>, CudaTensor<u32>>, CudaError> {
        self.gather_loaded(input, indices, axis, &self.indexing.gather[T::KIND])
    }

    pub(crate) fn gather_loaded<T: DeviceRepr + ValidAsZeroBits>(
        &self,
        input: &CudaTensor<T>,
        indices: &CudaTensor<u32>,
        axis: usize,
        kernel: &CudaFunction,
    ) -> Result<Gathered<CudaTensor<T>, CudaTensor<u32>>, CudaError> {
        self.check(input)?;
        self.check(indices)?;
        let shape = gather_shape(input.shape(), indices.shape(), axis)?;
        let mut values = self.zeros_typed(shape.clone())?;
        let invalid_count = self.invalid_index_count(indices, input.shape().dims()[axis])?;
        let index_count = indices.shape().numel() as u64;
        if index_count == 0 {
            return Ok(Gathered {
                values,
                invalid_count,
            });
        }
        let index_rank = rank(indices.shape())?;
        let index_offset = indices.layout.offset() as u64;
        let length = input.shape().dims()[axis] as u64;
        if !shape.is_empty() {
            let mut metadata = layout_metadata(&input.layout);
            metadata.extend(layout_metadata(&indices.layout));
            let metadata = self.metadata(&metadata)?;
            let count = shape.numel() as u64;
            let input_rank = rank(input.shape())?;
            let input_offset = input.layout.offset() as u64;
            // All dimensions other than the replaced axis occur in this
            // nonempty output, so this product fits its validated element count.
            let inner = input.shape().dims()[axis + 1..].iter().product::<usize>() as u64;
            // The kernel compares every selected u32 index to the original
            // axis length before computing/reading an input storage address.
            unsafe {
                self.device
                    .stream
                    .launch_builder(kernel)
                    .arg(input.storage.as_ref())
                    .arg(indices.storage.as_ref())
                    .arg(output(&mut values))
                    .arg(&metadata)
                    .arg(&count)
                    .arg(&index_count)
                    .arg(&length)
                    .arg(&inner)
                    .arg(&input_rank)
                    .arg(&index_rank)
                    .arg(&input_offset)
                    .arg(&index_offset)
                    .launch(self.config(count as usize))?;
            }
        }
        Ok(Gathered {
            values,
            invalid_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tensor_core::Layout;

    #[test]
    fn gather_flat_addressing_preserves_prefix_indices_and_suffix_axes() {
        let input = Layout::contiguous(Shape::new(vec![2, 3, 5]).unwrap())
            .unwrap()
            .permute(&[1, 0, 2])
            .unwrap();
        let indices = [1usize, 0, 1, 1];
        let result_shape =
            gather_shape(input.shape(), &Shape::new(vec![2, 2]).unwrap(), 1).unwrap();
        assert_eq!(result_shape.dims(), &[3, 2, 2, 5]);
        for i in 0..result_shape.numel() {
            let prefix = i / (indices.len() * 5);
            let chosen = indices[(i / 5) % indices.len()];
            let source_linear = (prefix * 2 + chosen) * 5 + i % 5;
            let expected_address = chosen * 15 + prefix * 5 + i % 5;
            assert_eq!(
                input.element_offset(source_linear).unwrap(),
                expected_address
            );
        }
    }
}
