use crate::{
    BinaryOp, ComputeBatch, ComputeError, ComputeRuntime, GpuArray, GpuElement, Readback,
    Reduction, UnaryOp, uniform_f32, wgpu,
};

/// A prepared sequence with explicit operation order. Intermediate arrays and
/// bindings are retained by the program. Submit repeatedly after updating inputs;
/// use `_into` operations and array prefixes to reuse caller-owned allocations.
pub struct ComputeProgram<'a> {
    pub(crate) runtime: &'a ComputeRuntime,
    pub(crate) batch: ComputeBatch<'a>,
}

impl<'a> ComputeProgram<'a> {
    fn same_length(&self, a: &GpuArray<f32>, b: &GpuArray<f32>) -> Result<(), ComputeError> {
        self.runtime.check(a)?;
        self.runtime.check(b)?;
        if a.len() != b.len() {
            return Err(ComputeError::LengthMismatch {
                expected: a.len(),
                actual: b.len(),
            });
        }
        Ok(())
    }

    pub fn affine(
        &mut self,
        input: &GpuArray<f32>,
        scale: f32,
        offset: f32,
    ) -> Result<GpuArray<f32>, ComputeError> {
        self.runtime.check(input)?;
        let output = self.runtime.zeros(input.len())?;
        self.affine_into(input, scale, offset, &output)?;
        Ok(output)
    }

    pub fn affine_into(
        &mut self,
        input: &GpuArray<f32>,
        scale: f32,
        offset: f32,
        output: &GpuArray<f32>,
    ) -> Result<(), ComputeError> {
        self.same_length(input, output)?;
        if input.aliases(output) {
            return Err(ComputeError::AliasedOutput);
        }
        if input.is_empty() {
            return Ok(());
        }
        let kernel = &self.runtime.affine;
        let params = uniform_f32(
            &self.runtime.device,
            &self.runtime.queue,
            &[f32::from_bits(input.len), scale, offset, 0.0],
        );
        let bindings = kernel.create_bind_group(
            &self.runtime.device,
            &[&params, input.buffer(), output.buffer()],
        );
        self.batch.push(
            kernel,
            &bindings,
            kernel.workgroup_count(input.len).min(65535),
        );
        Ok(())
    }

    pub fn unary(
        &mut self,
        op: UnaryOp,
        input: &GpuArray<f32>,
    ) -> Result<GpuArray<f32>, ComputeError> {
        self.runtime.check(input)?;
        let output = self.runtime.zeros(input.len())?;
        self.unary_into(op, input, &output)?;
        Ok(output)
    }

    pub fn unary_into(
        &mut self,
        op: UnaryOp,
        input: &GpuArray<f32>,
        output: &GpuArray<f32>,
    ) -> Result<(), ComputeError> {
        self.same_length(input, output)?;
        if input.aliases(output) {
            return Err(ComputeError::AliasedOutput);
        }
        if input.is_empty() {
            return Ok(());
        }
        let kernel = &self.runtime.unary;
        let params = uniform_f32(
            &self.runtime.device,
            &self.runtime.queue,
            &[
                f32::from_bits(input.len),
                f32::from_bits(op as u32),
                0.0,
                0.0,
            ],
        );
        let bindings = kernel.create_bind_group(
            &self.runtime.device,
            &[&params, input.buffer(), output.buffer()],
        );
        self.batch.push(
            kernel,
            &bindings,
            kernel.workgroup_count(input.len).min(65535),
        );
        Ok(())
    }

    fn binary_length(&self, a: &GpuArray<f32>, b: &GpuArray<f32>) -> Result<usize, ComputeError> {
        self.runtime.check(a)?;
        self.runtime.check(b)?;
        if a.len() == b.len() || b.len() == 1 {
            return Ok(a.len());
        }
        if a.len() == 1 {
            return Ok(b.len());
        }
        Err(ComputeError::LengthMismatch {
            expected: a.len(),
            actual: b.len(),
        })
    }

    /// Combines equal-length arrays, or broadcasts a single-element array over
    /// the other operand. This also lets GPU reductions feed array operations.
    pub fn binary(
        &mut self,
        op: BinaryOp,
        a: &GpuArray<f32>,
        b: &GpuArray<f32>,
    ) -> Result<GpuArray<f32>, ComputeError> {
        let len = self.binary_length(a, b)?;
        let output = self.runtime.zeros(len)?;
        self.binary_into(op, a, b, &output)?;
        Ok(output)
    }

    pub fn binary_into(
        &mut self,
        op: BinaryOp,
        a: &GpuArray<f32>,
        b: &GpuArray<f32>,
        output: &GpuArray<f32>,
    ) -> Result<(), ComputeError> {
        let len = self.binary_length(a, b)?;
        self.runtime.check(output)?;
        if output.len() != len {
            return Err(ComputeError::LengthMismatch {
                expected: len,
                actual: output.len(),
            });
        }
        if output.aliases(a) || output.aliases(b) {
            return Err(ComputeError::AliasedOutput);
        }
        if len == 0 {
            return Ok(());
        }
        let kernel = &self.runtime.binary;
        let params = uniform_f32(
            &self.runtime.device,
            &self.runtime.queue,
            &[
                f32::from_bits(len as u32),
                f32::from_bits(op as u32),
                f32::from_bits(u32::from(a.len() != 1)),
                f32::from_bits(u32::from(b.len() != 1)),
            ],
        );
        let bindings = kernel.create_bind_group(
            &self.runtime.device,
            &[&params, a.buffer(), b.buffer(), output.buffer()],
        );
        self.batch.push(
            kernel,
            &bindings,
            kernel.workgroup_count(len as u32).min(65535),
        );
        Ok(())
    }

    pub fn sum(&mut self, input: &GpuArray<f32>) -> Result<GpuArray<f32>, ComputeError> {
        self.runtime.check(input)?;
        let output = self.runtime.zeros::<f32>(1)?;
        let plan = Reduction::with_output(
            &self.runtime.device,
            &self.runtime.queue,
            &self.runtime.sum,
            input.buffer(),
            input.len,
            output.buffer(),
        );
        self.batch.push_reduction(&plan);
        Ok(output)
    }

    pub fn dot(
        &mut self,
        a: &GpuArray<f32>,
        b: &GpuArray<f32>,
    ) -> Result<GpuArray<f32>, ComputeError> {
        self.same_length(a, b)?;
        let products = self.binary(BinaryOp::Multiply, a, b)?;
        self.sum(&products)
    }

    pub fn record(&self, encoder: &mut wgpu::CommandEncoder) {
        self.batch.record(encoder);
    }

    pub fn submit(&self) -> wgpu::SubmissionIndex {
        self.batch.submit(&self.runtime.device, &self.runtime.queue)
    }

    /// Records all kernels and the final copy/map in one submission. Returns
    /// immediately with a ticket; no synchronous GPU wait is performed.
    pub fn submit_read<T: GpuElement>(
        &self,
        output: &GpuArray<T>,
    ) -> Result<Readback<T>, ComputeError> {
        self.runtime.check(output)?;
        let mut encoder =
            self.runtime
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("compute program and readback"),
                });
        self.batch.record(&mut encoder);
        let mut readback = Readback::record(&self.runtime.device, &mut encoder, output)?;
        let index = self.runtime.queue.submit([encoder.finish()]);
        readback.submitted(index);
        Ok(readback)
    }
}
