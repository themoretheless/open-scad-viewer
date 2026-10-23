use crate::{
    Binding, ComputeBatch, ComputeError, ComputeProgram, ComputeRuntime, GpuArray, Kernel, shaders,
    uniform_f32, wgpu,
};

pub(crate) struct ScanKernels {
    blocks: Kernel,
    add: Kernel,
}

/// Must match ITEMS in scan_blocks4.wgsl; each lane scans four adjacent scalars.
const ITEMS_PER_LANE: u32 = 4;

impl ScanKernels {
    pub(crate) fn new(device: &wgpu::Device) -> Result<Self, crate::KernelError> {
        Ok(Self {
            blocks: Kernel::new(
                device,
                "exclusive scan blocks",
                shaders::SCAN_BLOCKS4_WGSL,
                "main",
                &[
                    Binding::Uniform,
                    Binding::StorageRead,
                    Binding::StorageReadWrite,
                    Binding::StorageReadWrite,
                ],
            )?,
            add: Kernel::new(
                device,
                "exclusive scan offsets",
                shaders::SCAN_ADD_WGSL,
                "main",
                &[
                    Binding::Uniform,
                    Binding::StorageRead,
                    Binding::StorageReadWrite,
                ],
            )?,
        })
    }
}

/// Prepared off to the side so allocation errors never append partial work to
/// the caller's program. Bind groups retain intermediate buffers and uniforms.
pub(crate) struct ScanPlan<'a> {
    steps: Vec<(&'a Kernel, wgpu::BindGroup, u32)>,
}

impl<'a> ScanPlan<'a> {
    pub(crate) fn new(
        runtime: &'a ComputeRuntime,
        input: &GpuArray<u32>,
        output: &GpuArray<u32>,
        normalize: bool,
    ) -> Result<Self, ComputeError> {
        let mut plan = Self {
            steps: Vec::with_capacity(4),
        };
        if !input.is_empty() {
            plan.level(runtime, input, output, normalize)?;
        }
        Ok(plan)
    }

    fn level(
        &mut self,
        runtime: &'a ComputeRuntime,
        input: &GpuArray<u32>,
        output: &GpuArray<u32>,
        normalize: bool,
    ) -> Result<(), ComputeError> {
        let totals = self.record_blocks(runtime, input, output, normalize)?;
        let blocks = totals.len;
        if blocks > 1 {
            let offsets = runtime.zeros::<u32>(blocks as usize)?;
            self.level(runtime, &totals, &offsets, false)?;
            let add = &runtime.scan.add;
            let groups = add.workgroup_count(input.len).min(65535);
            let params = uniform_f32(
                &runtime.device,
                &runtime.queue,
                &[
                    f32::from_bits(input.len),
                    f32::from_bits(groups),
                    f32::from_bits(runtime.scan.blocks.workgroup_size() * ITEMS_PER_LANE),
                    0.0,
                ],
            );
            let bindings = add.create_bind_group(
                &runtime.device,
                &[&params, offsets.buffer(), output.buffer()],
            );
            self.steps.push((add, bindings, groups));
        }
        Ok(())
    }

    fn record_blocks(
        &mut self,
        runtime: &'a ComputeRuntime,
        input: &GpuArray<u32>,
        output: &GpuArray<u32>,
        normalize: bool,
    ) -> Result<GpuArray<u32>, ComputeError> {
        let kernel = &runtime.scan.blocks;
        let blocks = input.len.div_ceil(kernel.workgroup_size() * ITEMS_PER_LANE);
        let groups = blocks.min(65535);
        let totals = runtime.zeros::<u32>(blocks as usize)?;
        let params = uniform_f32(
            &runtime.device,
            &runtime.queue,
            &[
                f32::from_bits(input.len),
                f32::from_bits(groups),
                f32::from_bits(u32::from(normalize)),
                0.0,
            ],
        );
        let bindings = kernel.create_bind_group(
            &runtime.device,
            &[&params, input.buffer(), output.buffer(), totals.buffer()],
        );
        self.steps.push((kernel, bindings, groups));
        Ok(totals)
    }

    /// Produces per-block local offsets and separately scanned block offsets.
    /// Compaction combines them during scatter, avoiding a full-size add pass.
    pub(crate) fn local(
        runtime: &'a ComputeRuntime,
        input: &GpuArray<u32>,
        local: &GpuArray<u32>,
    ) -> Result<(Self, GpuArray<u32>, u32), ComputeError> {
        let mut plan = Self {
            steps: Vec::with_capacity(4),
        };
        let kernel = &runtime.scan.blocks;
        let block_size = kernel.workgroup_size() * ITEMS_PER_LANE;
        let blocks = input.len.div_ceil(block_size);
        let block_offsets = runtime.zeros::<u32>(blocks.max(1) as usize)?;
        if !input.is_empty() {
            let totals = plan.record_blocks(runtime, input, local, true)?;
            if blocks > 1 {
                plan.level(runtime, &totals, &block_offsets, false)?;
            }
        }
        Ok((plan, block_offsets, block_size))
    }

    pub(crate) fn append_to(&self, batch: &mut ComputeBatch<'a>) {
        for (kernel, bindings, groups) in &self.steps {
            batch.push(kernel, bindings, *groups);
        }
    }
}

impl ComputeProgram<'_> {
    /// Exclusive prefix sum: `output[i] = sum(input[..i])`, with u32 wrapping
    /// addition. Empty input produces an empty array. Every level stays on GPU.
    pub fn exclusive_scan(&mut self, input: &GpuArray<u32>) -> Result<GpuArray<u32>, ComputeError> {
        self.runtime.check(input)?;
        let output = self.runtime.zeros(input.len())?;
        self.exclusive_scan_into(input, &output)?;
        Ok(output)
    }

    /// Reuses a distinct, equal-length output allocation. The prepared program
    /// can run repeatedly after input updates without rebuilding scan levels.
    pub fn exclusive_scan_into(
        &mut self,
        input: &GpuArray<u32>,
        output: &GpuArray<u32>,
    ) -> Result<(), ComputeError> {
        self.runtime.check(input)?;
        self.runtime.check(output)?;
        if input.len() != output.len() {
            return Err(ComputeError::LengthMismatch {
                expected: input.len(),
                actual: output.len(),
            });
        }
        if input.aliases(output) {
            return Err(ComputeError::AliasedOutput);
        }
        let plan = ScanPlan::new(self.runtime, input, output, false)?;
        plan.append_to(&mut self.batch);
        Ok(())
    }
}

/// Deterministic CPU reference exclusive prefix scan (`output[i] = sum(input[..i])` with `u32` wrapping addition),
/// matching [`ComputeProgram::exclusive_scan`].
#[inline]
pub fn exclusive_scan_u32(input: &[u32]) -> Vec<u32> {
    let mut acc = 0_u32;
    input
        .iter()
        .map(|&v| {
            let prev = acc;
            acc = acc.wrapping_add(v);
            prev
        })
        .collect()
}

/// Deterministic CPU reference inclusive prefix scan (`output[i] = sum(input[..=i])` with `u32` wrapping addition).
#[inline]
pub fn inclusive_scan_u32(input: &[u32]) -> Vec<u32> {
    let mut acc = 0_u32;
    input
        .iter()
        .map(|&v| {
            acc = acc.wrapping_add(v);
            acc
        })
        .collect()
}

/// Deterministic CPU reference stream compaction keeping elements where `keep[i] != 0` in input order.
pub fn compact_if_u32<T: Copy>(input: &[T], keep: &[u32]) -> Result<Vec<T>, ComputeError> {
    if input.len() != keep.len() {
        return Err(ComputeError::LengthMismatch {
            expected: input.len(),
            actual: keep.len(),
        });
    }
    let count = keep.iter().filter(|&&m| m != 0).count();
    let mut out = Vec::with_capacity(count);
    for (&val, &mask) in input.iter().zip(keep.iter()) {
        if mask != 0 {
            out.push(val);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_reference_scans_and_compaction_match_expected_semantics() {
        let data = [3_u32, 1, 4, 1, 5, 9];
        assert_eq!(exclusive_scan_u32(&data), vec![0, 3, 4, 8, 9, 14]);
        assert_eq!(inclusive_scan_u32(&data), vec![3, 4, 8, 9, 14, 23]);
        assert!(exclusive_scan_u32(&[]).is_empty());
        assert!(inclusive_scan_u32(&[]).is_empty());

        let values = [10.0_f32, 20.0, 30.0, 40.0, 50.0];
        let keep = [0_u32, 1, 0, 5, 1];
        let compacted = compact_if_u32(&values, &keep).unwrap();
        assert_eq!(compacted, vec![20.0, 40.0, 50.0]);
        assert!(compact_if_u32(&values, &[1, 0]).is_err());
    }
}
