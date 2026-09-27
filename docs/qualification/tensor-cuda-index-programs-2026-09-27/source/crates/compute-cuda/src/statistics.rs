use crate::{CudaError, CudaRuntime, CudaTensor, indexing::output};
use gpu_compute::cuda::{
    CudaFunction, CudaModule, CudaSlice, LaunchConfig, PushKernelArg, cudarc::driver::DeviceRepr,
};
use std::sync::Arc;
use tensor_core::{
    Layout, Moments, Shape, TensorBackend, TensorStatsBackend, normalization_shape,
    statistics_shape, validate_epsilon,
};

pub(crate) struct StatisticsKernels {
    partial: [CudaFunction; 2],
    merge: CudaFunction,
    emit: [CudaFunction; 2],
    lse: CudaFunction,
    moments: [CudaFunction; 2],
}
impl StatisticsKernels {
    pub(crate) fn load(module: &Arc<CudaModule>) -> Result<Self, CudaError> {
        let pair = |name: &str| -> Result<_, CudaError> {
            Ok([
                module.load_function(name)?,
                module.load_function(&format!("{name}_low"))?,
            ])
        };
        Ok(Self {
            partial: pair("stats_partial")?,
            merge: module.load_function("stats_merge")?,
            emit: pair("stats_emit")?,
            lse: module.load_function("stats_lse")?,
            moments: pair("stats_moments")?,
        })
    }
}

struct RowPlan {
    metadata: Vec<u64>,
    rows: usize,
    reduce_count: usize,
    chunks: usize,
    outer_rank: u32,
    reduce_rank: u32,
    offset: u64,
}
impl RowPlan {
    fn new(layout: &Layout, axes: &[usize], max_groups: usize) -> Result<Self, CudaError> {
        let outer = statistics_shape(layout.shape(), axes, false)?;
        let rows = outer.numel();
        let reduce_count = layout.shape().numel() / rows;
        let chunks = reduce_count
            .div_ceil(2048)
            .min((max_groups / rows).max(1))
            .max(1);
        let kept: Vec<usize> = (0..layout.shape().rank())
            .filter(|axis| !axes.contains(axis))
            .collect();
        let dense = Layout::contiguous(layout.shape().clone())?;
        let mut metadata = Vec::new();
        for list in [&kept[..], axes] {
            metadata.extend(list.iter().map(|&axis| layout.shape().dims()[axis] as u64));
            metadata.extend(list.iter().map(|&axis| layout.strides()[axis] as u64));
            metadata.extend(list.iter().map(|&axis| dense.strides()[axis] as u64));
        }
        Ok(Self {
            metadata,
            rows,
            reduce_count,
            chunks,
            outer_rank: kept.len() as u32,
            reduce_rank: axes.len() as u32,
            offset: layout.offset() as u64,
        })
    }
}
struct RowStats {
    first: CudaTensor<f64>,
    second: CudaTensor<f64>,
    metadata: CudaSlice<u64>,
    plan: RowPlan,
}

impl CudaRuntime {
    fn stats_rows<T: DeviceRepr>(
        &self,
        input: &CudaTensor<T>,
        axes: &[usize],
        moments: bool,
        dtype: Option<u32>,
    ) -> Result<RowStats, CudaError> {
        let plan = RowPlan::new(
            input.layout(),
            axes,
            self.device.multiprocessors as usize * 8,
        )?;
        let row_shape = Shape::new(vec![plan.rows])?;
        let mut first = self.zeros_typed::<f64>(row_shape.clone())?;
        let mut second = self.zeros_typed::<f64>(row_shape)?;
        let mut partials = self.zeros_typed::<f64>(Shape::new(vec![plan.rows, plan.chunks])?)?;
        let metadata = self.metadata(&plan.metadata)?;
        let (rows, count, chunks) = (
            plan.rows as u64,
            plan.reduce_count as u64,
            plan.chunks as u64,
        );
        let config = |jobs: usize| LaunchConfig {
            grid_dim: (
                jobs.min(self.device.multiprocessors as usize * 8) as u32,
                1,
                1,
            ),
            block_dim: (256, 1, 1),
            shared_mem_bytes: 0,
        };
        for operation in if moments { [2u32, 3] } else { [0u32, 1] } {
            // Fresh per-row allocations; same-stream ordering allows the partial
            // scratch to be reused after its merge without a host readback.
            unsafe {
                let mut launch = self
                    .device
                    .stream
                    .launch_builder(&self.statistics.partial[usize::from(dtype.is_some())]);
                launch
                    .arg(input.storage.as_ref())
                    .arg(first.storage.as_ref())
                    .arg(output(&mut partials))
                    .arg(&metadata)
                    .arg(&rows)
                    .arg(&count)
                    .arg(&chunks)
                    .arg(&plan.outer_rank)
                    .arg(&plan.reduce_rank)
                    .arg(&plan.offset)
                    .arg(&operation);
                if let Some(dtype) = dtype.as_ref() {
                    launch.arg(dtype);
                }
                launch.launch(config(plan.rows * plan.chunks))?;
            }
            let destination = if operation.is_multiple_of(2) {
                &mut first
            } else {
                &mut second
            };
            unsafe {
                self.device
                    .stream
                    .launch_builder(&self.statistics.merge)
                    .arg(partials.storage.as_ref())
                    .arg(output(destination))
                    .arg(&rows)
                    .arg(&chunks)
                    .arg(&count)
                    .arg(&operation)
                    .launch(config(plan.rows))?;
            }
        }
        Ok(RowStats {
            first,
            second,
            metadata,
            plan,
        })
    }

    pub(crate) fn normalize_loaded<T: DeviceRepr>(
        &self,
        input: &CudaTensor<T>,
        axes: &[usize],
        op: u32,
        epsilon: f32,
        dtype: Option<u32>,
    ) -> Result<CudaTensor, CudaError> {
        self.check(input)?;
        let shape = normalization_shape(input.shape(), axes)?;
        if shape.is_empty() {
            return self.zeros(shape);
        }
        let reduced = statistics_shape(input.shape(), axes, false)?;
        if input.shape().numel() == reduced.numel() {
            let mut result = self.zeros(shape)?;
            if op == 0 {
                let count = input.shape().numel() as u64;
                let one = 1f32;
                // Singleton softmax writes one without an input-sized temporary.
                unsafe {
                    self.device
                        .stream
                        .launch_builder(&self.reduction.fill[0])
                        .arg(output(&mut result))
                        .arg(&count)
                        .arg(&one)
                        .launch(self.config(count as usize))?;
                }
            }
            return Ok(result);
        }
        let mut result = self.zeros(shape)?;
        let rows = self.stats_rows(input, axes, op == 2, dtype)?;
        let count = input.shape().numel() as u64;
        let reduce_count = rows.plan.reduce_count as u64;
        // Logical row/reduction coordinates are scattered to a distinct dense
        // output using strides of the original logical input shape.
        unsafe {
            let mut launch = self
                .device
                .stream
                .launch_builder(&self.statistics.emit[usize::from(dtype.is_some())]);
            launch
                .arg(input.storage.as_ref())
                .arg(rows.first.storage.as_ref())
                .arg(rows.second.storage.as_ref())
                .arg(output(&mut result))
                .arg(&rows.metadata)
                .arg(&count)
                .arg(&reduce_count)
                .arg(&rows.plan.outer_rank)
                .arg(&rows.plan.reduce_rank)
                .arg(&rows.plan.offset)
                .arg(&op)
                .arg(&epsilon);
            if let Some(dtype) = dtype.as_ref() {
                launch.arg(dtype);
            }
            launch.launch(self.config(count as usize))?;
        }
        Ok(result)
    }
}

impl TensorStatsBackend for CudaRuntime {
    fn softmax(&self, input: &CudaTensor, axes: &[usize]) -> Result<CudaTensor, CudaError> {
        self.normalize_loaded(input, axes, 0, 0., None)
    }
    fn log_softmax(&self, input: &CudaTensor, axes: &[usize]) -> Result<CudaTensor, CudaError> {
        self.normalize_loaded(input, axes, 1, 0., None)
    }
    fn layer_norm(
        &self,
        input: &CudaTensor,
        axes: &[usize],
        epsilon: f32,
    ) -> Result<CudaTensor, CudaError> {
        validate_epsilon(epsilon)?;
        self.normalize_loaded(input, axes, 2, epsilon, None)
    }
    fn logsumexp(
        &self,
        input: &CudaTensor,
        axes: &[usize],
        keep_dims: bool,
    ) -> Result<CudaTensor, CudaError> {
        self.check(input)?;
        let shape = statistics_shape(input.shape(), axes, keep_dims)?;
        if shape.is_empty() {
            return self.zeros(shape);
        }
        if input.shape().numel() == shape.numel() {
            return self.reshape(input, shape);
        }
        self.stats_logsumexp_loaded(input, axes, shape, None)
    }
    fn moments(
        &self,
        input: &CudaTensor,
        axes: &[usize],
        keep_dims: bool,
    ) -> Result<Moments<CudaTensor>, CudaError> {
        self.check(input)?;
        let shape = statistics_shape(input.shape(), axes, keep_dims)?;
        let variance = self.zeros(shape.clone())?;
        if shape.is_empty() {
            return Ok(Moments {
                mean: self.zeros(shape)?,
                variance,
            });
        }
        if input.shape().numel() == shape.numel() {
            return Ok(Moments {
                mean: self.reshape(input, shape)?,
                variance,
            });
        }
        self.stats_moments_loaded(input, axes, shape, variance, None)
    }
}

impl CudaRuntime {
    // Callers validate shape/ownership and handle singleton/empty contractions.
    pub(crate) fn stats_logsumexp_loaded<T: DeviceRepr>(
        &self,
        input: &CudaTensor<T>,
        axes: &[usize],
        shape: Shape,
        dtype: Option<u32>,
    ) -> Result<CudaTensor, CudaError> {
        let mut result = self.zeros(shape)?;
        let rows = self.stats_rows(input, axes, false, dtype)?;
        let count = rows.plan.rows as u64;
        unsafe {
            self.device
                .stream
                .launch_builder(&self.statistics.lse)
                .arg(rows.first.storage.as_ref())
                .arg(rows.second.storage.as_ref())
                .arg(output(&mut result))
                .arg(&count)
                .launch(self.config(count as usize))?;
        }
        Ok(result)
    }
    pub(crate) fn stats_moments_loaded<T: DeviceRepr>(
        &self,
        input: &CudaTensor<T>,
        axes: &[usize],
        shape: Shape,
        mut variance: CudaTensor,
        dtype: Option<u32>,
    ) -> Result<Moments<CudaTensor>, CudaError> {
        let mut mean = self.zeros(shape)?;
        let rows = self.stats_rows(input, axes, true, dtype)?;
        let count = rows.plan.rows as u64;
        unsafe {
            let mut launch = self
                .device
                .stream
                .launch_builder(&self.statistics.moments[usize::from(dtype.is_some())]);
            launch
                .arg(input.storage.as_ref())
                .arg(rows.first.storage.as_ref())
                .arg(rows.second.storage.as_ref())
                .arg(output(&mut mean))
                .arg(output(&mut variance))
                .arg(&rows.metadata)
                .arg(&count)
                .arg(&rows.plan.outer_rank)
                .arg(&rows.plan.reduce_rank)
                .arg(&rows.plan.offset);
            if let Some(dtype) = dtype.as_ref() {
                launch.arg(dtype);
            }
            launch.launch(self.config(count as usize))?;
        }
        Ok(Moments { mean, variance })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn row_plan_preserves_strides_and_bounds_partial_storage() {
        let source = Layout::contiguous(Shape::new(vec![3, 5, 7]).unwrap())
            .unwrap()
            .permute(&[2, 0, 1])
            .unwrap();
        let plan = RowPlan::new(&source, &[2, 0], 1024).unwrap();
        assert_eq!((plan.rows, plan.reduce_count, plan.chunks), (3, 35, 1));
        assert_eq!(plan.metadata, [3, 35, 5, 5, 7, 7, 1, 1, 15]);
        for (dims, axes) in [(vec![1_048_583], vec![0]), (vec![257, 8193], vec![1])] {
            let shape = Shape::new(dims).unwrap();
            let plan =
                RowPlan::new(&Layout::contiguous(shape.clone()).unwrap(), &axes, 1024).unwrap();
            assert!(plan.rows * plan.chunks <= shape.numel());
            assert!(plan.chunks <= 1024);
        }
    }
}
