//! Frozen first centered implementation, retained only as a benchmark control.
use compute_core::{Binding, ComputeBatch, ComputeRuntime, GpuArray, Kernel, uniform_f32};
use gpu_compute::GpuContext;

pub struct FrozenCentered {
    partial: Kernel,
    fold: Kernel,
    covariance: Kernel,
}
impl FrozenCentered {
    pub fn new(c: &GpuContext) -> Result<Self, Box<dyn std::error::Error>> {
        let unary = [
            Binding::Uniform,
            Binding::StorageRead,
            Binding::StorageReadWrite,
        ];
        Ok(Self {
            partial: Kernel::new(
                &c.device,
                "frozen centered aos256",
                include_str!("centered_aos256.wgsl"),
                "main",
                &[
                    Binding::Uniform,
                    Binding::StorageRead,
                    Binding::StorageRead,
                    Binding::StorageReadWrite,
                ],
            )?,
            fold: Kernel::new(
                &c.device,
                "frozen stats fold",
                include_str!("fold.wgsl"),
                "main",
                &unary,
            )?,
            covariance: Kernel::new(
                &c.device,
                "frozen covariance finalize",
                include_str!("covariance.wgsl"),
                "main",
                &unary,
            )?,
        })
    }
    pub fn prepare<'a>(
        &'a self,
        runtime: &ComputeRuntime,
        input: &GpuArray<f32>,
        stats: &GpuArray<f32>,
    ) -> Result<ComputeBatch<'a>, Box<dyn std::error::Error>> {
        let count = (input.len() / 3) as u32;
        let c = runtime.context();
        let params = uniform_f32(&c.device, &c.queue, &[f32::from_bits(count), 0., 0., 0.]);
        let mut groups = self.partial.workgroup_count(count);
        let mut partial = runtime.zeros::<f32>(groups as usize * 15)?;
        let binding = self.partial.create_bind_group_resources(
            &c.device,
            &[
                params.as_entire_binding(),
                input.view().storage_binding(c)?,
                stats.view().storage_binding(c)?,
                partial.view().storage_binding(c)?,
            ],
        );
        let mut batch = ComputeBatch::new();
        batch.push(&self.partial, &binding, groups);
        while groups > 1 {
            let params = uniform_f32(&c.device, &c.queue, &[f32::from_bits(groups), 0., 0., 0.]);
            groups = self.fold.workgroup_count(groups);
            let next = runtime.zeros::<f32>(groups as usize * 15)?;
            let binding = self.fold.create_bind_group_resources(
                &c.device,
                &[
                    params.as_entire_binding(),
                    partial.view().storage_binding(c)?,
                    next.view().storage_binding(c)?,
                ],
            );
            batch.push(&self.fold, &binding, groups);
            partial = next;
        }
        let binding = self.covariance.create_bind_group_resources(
            &c.device,
            &[
                params.as_entire_binding(),
                partial.view().storage_binding(c)?,
                stats.view().storage_binding(c)?,
            ],
        );
        batch.push(&self.covariance, &binding, 1);
        Ok(batch)
    }
}
