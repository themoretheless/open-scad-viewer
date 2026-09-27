use super::{CudaScalar, output};
use crate::{
    CudaError, CudaRuntime, CudaTensor,
    runtime::{layout_metadata, rank},
};
use gpu_compute::cuda::{CudaFunction, PushKernelArg, cudarc::driver::DeviceRepr};
use tensor_core::{ScatterOp, Scattered, Shape, TensorScatterBackend, scatter_updates_shape};

impl CudaRuntime {
    fn scatter_typed<T: CudaScalar>(
        &self,
        op: ScatterOp,
        input: &CudaTensor<T>,
        indices: &CudaTensor<u32>,
        updates: &CudaTensor<T>,
        axis: usize,
    ) -> Result<Scattered<CudaTensor<T>, CudaTensor<u32>>, CudaError> {
        self.check(input)?;
        self.check(indices)?;
        self.check(updates)?;
        let expected =
            scatter_updates_shape(input.shape(), indices.shape(), updates.shape(), axis)?;
        // Validate the logical byte count after broadcasting as well as the
        // underlying storage before any operation is enqueued.
        let updates = self.view(updates, updates.layout.broadcast_to(expected)?)?;
        let mut values = self.copy_typed(input)?;
        let invalid_count = self.scatter_loaded(
            op,
            indices,
            &updates,
            &mut values,
            axis,
            &self.indexing.scatter[T::KIND],
            None,
        )?;
        Ok(Scattered {
            values,
            invalid_count,
        })
    }

    /// Shared dispatch after callers validate the broadcast update shape and
    /// allocate an independent contiguous base copy. Source and destination
    /// scalar widths may differ; the selected kernel controls their ABI.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn scatter_loaded<I: DeviceRepr, O: DeviceRepr>(
        &self,
        op: ScatterOp,
        indices: &CudaTensor<u32>,
        updates: &CudaTensor<I>,
        values: &mut CudaTensor<O>,
        axis: usize,
        kernel: &CudaFunction,
        dtype: Option<u32>,
    ) -> Result<CudaTensor<u32>, CudaError> {
        let length = values.shape().dims()[axis];
        let invalid_count = self.invalid_index_count(indices, length)?;
        let index_count = indices.shape().numel() as u64;
        if values.shape().is_empty() || index_count == 0 {
            return Ok(invalid_count);
        }
        let index_rank = rank(indices.shape())?;
        let index_offset = indices.layout.offset() as u64;
        let axis_length = length as u64;
        let owners = if op == ScatterOp::Replace {
            let mut owners = self.zeros_typed::<u32>(Shape::new(vec![length])?)?;
            let metadata = self.metadata(&layout_metadata(&indices.layout))?;
            // One owner per axis destination; j+1 is a nonzero position token.
            // The index-count bound guarantees every token fits u32.
            unsafe {
                self.device
                    .stream
                    .launch_builder(&self.indexing.scatter_owners)
                    .arg(indices.storage.as_ref())
                    .arg(output(&mut owners))
                    .arg(&metadata)
                    .arg(&index_count)
                    .arg(&index_rank)
                    .arg(&index_offset)
                    .arg(&axis_length)
                    .launch(self.config(index_count as usize))?;
            }
            Some(owners)
        } else {
            None
        };
        // Fold kernels never dereference owners. Reuse a live device pointer
        // for that unused parameter rather than allocate dummy storage.
        let owners = owners.as_ref().unwrap_or(&invalid_count);
        let mut metadata = layout_metadata(&updates.layout);
        metadata.extend(layout_metadata(&indices.layout));
        let metadata = self.metadata(&metadata)?;
        let count = updates.shape().numel() as u64;
        let inner = values.shape().dims()[axis + 1..].iter().product::<usize>() as u64;
        let update_rank = rank(updates.shape())?;
        let update_offset = updates.layout.offset() as u64;
        let operation = op as u32;
        // The base copy, owner election, and scatter share the retained stream.
        // Copying makes input/update aliases safe. Every output address is
        // guarded by chosen < axis_length before it is computed or accessed.
        unsafe {
            let mut launch = self.device.stream.launch_builder(kernel);
            launch
                .arg(indices.storage.as_ref())
                .arg(updates.storage.as_ref())
                .arg(output(values))
                .arg(owners.storage.as_ref())
                .arg(&metadata)
                .arg(&count)
                .arg(&index_count)
                .arg(&axis_length)
                .arg(&inner)
                .arg(&update_rank)
                .arg(&index_rank)
                .arg(&update_offset)
                .arg(&index_offset)
                .arg(&operation);
            if let Some(dtype) = dtype.as_ref() {
                launch.arg(dtype);
            }
            launch.launch(self.config(count as usize))?;
        }
        Ok(invalid_count)
    }
}

impl TensorScatterBackend for CudaRuntime {
    fn scatter_f32(
        &self,
        op: ScatterOp,
        input: &Self::Tensor,
        indices: &Self::UIntTensor,
        updates: &Self::Tensor,
        axis: usize,
    ) -> Result<Scattered<Self::Tensor, Self::UIntTensor>, CudaError> {
        self.scatter_typed(op, input, indices, updates, axis)
    }

    fn scatter_u32(
        &self,
        op: ScatterOp,
        input: &Self::UIntTensor,
        indices: &Self::UIntTensor,
        updates: &Self::UIntTensor,
        axis: usize,
    ) -> Result<Scattered<Self::UIntTensor, Self::UIntTensor>, CudaError> {
        self.scatter_typed(op, input, indices, updates, axis)
    }
}
