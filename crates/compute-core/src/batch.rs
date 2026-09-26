use crate::{Kernel, Reduction};
use wgpu::{Device, Queue};

/// Reusable sequence of GPU dispatches. Binding groups and intermediate buffers
/// are retained between runs; each submission creates only a command encoder.
#[derive(Default)]
pub struct ComputeBatch<'a> {
    steps: Vec<(&'a Kernel, wgpu::BindGroup, u32)>,
}

impl<'a> ComputeBatch<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a dispatch using a cached binding group. Zero groups are a no-op.
    pub fn push(&mut self, kernel: &'a Kernel, bindings: &wgpu::BindGroup, groups: u32) {
        assert!(groups <= 65535, "batch dispatch exceeds workgroup limit");
        if groups != 0 {
            self.steps.push((kernel, bindings.clone(), groups));
        }
    }

    /// Adds every pass of a prepared reduction in dependency order.
    pub fn push_reduction(&mut self, reduction: &Reduction<'a>) {
        reduction.append_to(self);
    }

    /// Appends the batch to an encoder, allowing copies or other work around it.
    pub fn record(&self, encoder: &mut wgpu::CommandEncoder) {
        for (kernel, bindings, groups) in &self.steps {
            kernel.record_dispatch(encoder, bindings, *groups);
        }
    }

    /// Submits all steps once. Read back only the final outputs when needed.
    /// Queue writes made before this call are visible to all steps. Use distinct
    /// uniform buffers when different steps require different parameters.
    pub fn submit(&self, device: &Device, queue: &Queue) -> wgpu::SubmissionIndex {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compute batch"),
        });
        self.record(&mut encoder);
        queue.submit([encoder.finish()])
    }
}
