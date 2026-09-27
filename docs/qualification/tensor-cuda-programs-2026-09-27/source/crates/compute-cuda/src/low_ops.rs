use crate::{
    CudaError, CudaLowTensor, CudaRuntime, CudaTensor,
    indexing::output,
    runtime::{layout_metadata, rank},
};
use gpu_compute::cuda::PushKernelArg;
use tensor_core::{
    BinaryOp, ReduceOp, TensorLowBackend, TensorLowOpsBackend, TensorReduceBackend, UnaryOp,
    low_binary_shape, mean_shape, reduction_shape,
};

impl TensorLowOpsBackend for CudaRuntime {
    fn unary_low(&self, op: UnaryOp, input: &CudaLowTensor) -> Result<CudaLowTensor, CudaError> {
        self.check(&input.tensor)?;
        let mut result = self.zeros_typed::<u16>(input.shape().clone())?;
        if !input.shape().is_empty() {
            let metadata = self.metadata(&layout_metadata(input.layout()))?;
            let (n, r, offset, operation, dtype) = (
                input.shape().numel() as u64,
                rank(input.shape())?,
                input.layout().offset() as u64,
                op as u32,
                input.dtype as u32,
            );
            // Native two-byte input/output. Negate/abs manipulate the sign bit;
            // the other operations decode and round once in the kernel.
            unsafe {
                self.device
                    .stream
                    .launch_builder(&self.low.unary)
                    .arg(input.tensor.storage.as_ref())
                    .arg(output(&mut result))
                    .arg(&metadata)
                    .arg(&n)
                    .arg(&r)
                    .arg(&offset)
                    .arg(&operation)
                    .arg(&dtype)
                    .launch(self.config(n as usize))?;
            }
        }
        Ok(CudaLowTensor {
            tensor: result,
            dtype: input.dtype,
        })
    }

    fn binary_low(
        &self,
        op: BinaryOp,
        left: &CudaLowTensor,
        right: &CudaLowTensor,
    ) -> Result<CudaLowTensor, CudaError> {
        self.check(&left.tensor)?;
        self.check(&right.tensor)?;
        let shape = low_binary_shape(left, right)?;
        let left_layout = left.layout().broadcast_to(shape.clone())?;
        let right_layout = right.layout().broadcast_to(shape.clone())?;
        let mut result = self.zeros_typed::<u16>(shape.clone())?;
        if !shape.is_empty() {
            let values: Vec<u64> = shape
                .dims()
                .iter()
                .chain(left_layout.strides())
                .chain(right_layout.strides())
                .map(|&v| v as u64)
                .collect();
            let metadata = self.metadata(&values)?;
            let (n, r, lo, ro, operation, dtype) = (
                shape.numel() as u64,
                rank(&shape)?,
                left_layout.offset() as u64,
                right_layout.offset() as u64,
                op as u32,
                left.dtype as u32,
            );
            unsafe {
                self.device
                    .stream
                    .launch_builder(&self.low.binary)
                    .arg(left.tensor.storage.as_ref())
                    .arg(right.tensor.storage.as_ref())
                    .arg(output(&mut result))
                    .arg(&metadata)
                    .arg(&n)
                    .arg(&r)
                    .arg(&lo)
                    .arg(&ro)
                    .arg(&operation)
                    .arg(&dtype)
                    .launch(self.config(n as usize))?;
            }
        }
        Ok(CudaLowTensor {
            tensor: result,
            dtype: left.dtype,
        })
    }

    fn reduce_low_f32(
        &self,
        op: ReduceOp,
        input: &CudaLowTensor,
        axes: &[usize],
        keep_dims: bool,
    ) -> Result<CudaTensor, CudaError> {
        self.check(&input.tensor)?;
        let shape = reduction_shape(op, input.shape(), axes, keep_dims)?;
        if axes.is_empty() {
            return self.cast_to_f32(input);
        }
        if shape.is_empty() {
            return self.zeros(shape);
        }
        if input.shape().is_empty() {
            let mut result = self.zeros(shape.clone())?;
            if op == ReduceOp::Product {
                let n = shape.numel() as u64;
                let one = 1f32;
                unsafe {
                    self.device
                        .stream
                        .launch_builder(&self.reduction.fill[0])
                        .arg(output(&mut result))
                        .arg(&n)
                        .arg(&one)
                        .launch(self.config(shape.numel()))?;
                }
            }
            return Ok(result);
        }
        if axes.len() == input.shape().rank() {
            // The only low-input pass reads native u16. Later hierarchy levels
            // share the existing f32 reducer, including bit-exact extrema.
            let partials = self.reduce_all_loaded::<u16, f32>(
                op,
                &input.tensor,
                &self.low.reduce_all,
                Some(input.dtype as u32),
            )?;
            let reduced = if partials.shape().numel() == 1 {
                partials
            } else {
                self.reduce_f32(op, &partials, &[0], false)?
            };
            return self.view(&reduced, reduced.layout().reshape(shape)?);
        }
        self.reduce_axes_loaded(
            op,
            &input.tensor,
            axes,
            shape,
            &self.low.reduce_axes,
            Some(input.dtype as u32),
        )
    }

    fn mean_low_f32(
        &self,
        input: &CudaLowTensor,
        axes: &[usize],
        keep_dims: bool,
    ) -> Result<CudaTensor, CudaError> {
        self.check(&input.tensor)?;
        let shape = mean_shape(input.shape(), axes, keep_dims)?;
        if axes.is_empty() {
            return self.cast_to_f32(input);
        }
        if shape.is_empty() {
            return self.zeros(shape);
        }
        let count = (input.shape().numel() / shape.numel()) as f32;
        let sum = self.reduce_low_f32(ReduceOp::Sum, input, axes, keep_dims)?;
        self.unary_op(&sum, 11, count, 0.)
    }
}
