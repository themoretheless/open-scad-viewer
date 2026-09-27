use crate::{CudaError, CudaRuntime, CudaTensor, indexing::output, runtime::rank};
use gpu_compute::cuda::{
    CudaFunction, CudaModule, LaunchConfig, PushKernelArg, cudarc::driver::DeviceRepr,
};
use std::sync::Arc;
use tensor_core::{
    AttentionMask, AttentionOptions, AttentionPlan, Layout, Shape, TensorAttentionBackend,
};

pub(crate) struct AttentionKernel([CudaFunction; 2]);
impl AttentionKernel {
    pub(crate) fn load(module: &Arc<CudaModule>) -> Result<Self, CudaError> {
        Ok(Self([
            module.load_function("attention_f32")?,
            module.load_function("attention_low")?,
        ]))
    }
}

fn canonical_layout(layout: &Layout, promoted: &Shape, batch: &Shape) -> Result<Layout, CudaError> {
    let mut strides = layout.strides().to_vec();
    if layout.shape().rank() == 2 {
        strides.insert(0, 0);
    }
    let promoted_layout = Layout::new(promoted.clone(), strides, layout.offset())?;
    let mut dims = batch.dims().to_vec();
    dims.extend_from_slice(&promoted.dims()[promoted.rank() - 3..]);
    Ok(promoted_layout.broadcast_to(Shape::new(dims)?)?)
}

fn attention_metadata(
    plan: &AttentionPlan,
    query: &Layout,
    key: &Layout,
    value: &Layout,
    mask: Option<&Layout>,
) -> Result<Vec<u64>, CudaError> {
    let query = canonical_layout(query, &plan.query, &plan.batch)?;
    let key = canonical_layout(key, &plan.key, &plan.batch)?;
    let value = canonical_layout(value, &plan.value, &plan.batch)?;
    let mask = mask
        .map(|m| m.broadcast_to(plan.scores.clone()))
        .transpose()?;
    let r = query.shape().rank();
    let row_rank = r - 1;
    let mut metadata: Vec<u64> = plan.canonical_output.dims()[..row_rank]
        .iter()
        .map(|&v| v as u64)
        .collect();
    for (layout, kv) in [(&query, false), (&key, true), (&value, true)] {
        metadata.extend(
            layout.strides()[..row_rank]
                .iter()
                .enumerate()
                .map(|(axis, &s)| {
                    if kv && axis == row_rank - 1 {
                        0
                    } else {
                        s as u64
                    }
                }),
        );
    }
    metadata
        .extend((0..row_rank).map(|axis| mask.as_ref().map_or(0, |m| m.strides()[axis] as u64)));
    metadata.extend([
        query.offset() as u64,
        key.offset() as u64,
        value.offset() as u64,
        mask.as_ref().map_or(0, |m| m.offset() as u64),
        query.strides()[r - 1] as u64,
        key.strides()[r - 2] as u64,
        key.strides()[r - 1] as u64,
        value.strides()[r - 2] as u64,
        value.strides()[r - 1] as u64,
        mask.as_ref().map_or(0, |m| m.strides()[r - 1] as u64),
        plan.group_size as u64,
    ]);
    Ok(metadata)
}

impl CudaRuntime {
    pub(crate) fn attention_loaded<T: DeviceRepr>(
        &self,
        query: &CudaTensor<T>,
        key: &CudaTensor<T>,
        value: &CudaTensor<T>,
        mask: AttentionMask<'_, CudaTensor, CudaTensor<u32>>,
        plan: AttentionPlan,
        dtype: Option<u32>,
    ) -> Result<CudaTensor, CudaError> {
        self.check(query)?;
        self.check(key)?;
        self.check(value)?;
        let (mask_layout, mask_flag) = match mask {
            AttentionMask::None => (None, 0u32),
            AttentionMask::Keep(tensor) => {
                self.check(tensor)?;
                (Some(tensor.layout()), 1)
            }
            AttentionMask::Additive(tensor) => {
                self.check(tensor)?;
                (Some(tensor.layout()), 2)
            }
        };
        let mut result = self.zeros(plan.output.clone())?;
        if result.shape().is_empty() || plan.keys == 0 {
            return Ok(result);
        }
        let metadata = self.metadata(&attention_metadata(
            &plan,
            query.layout(),
            key.layout(),
            value.layout(),
            mask_layout,
        )?)?;
        let mut scratch = self.zeros_typed::<f64>(plan.output.clone())?;
        let rows = (result.shape().numel() / plan.value_depth) as u64;
        let (keys, depth, value_depth) =
            (plan.keys as u64, plan.depth as u64, plan.value_depth as u64);
        let row_rank = rank(&plan.canonical_output)? - 1;
        let flags = mask_flag | if plan.causal.is_some() { 4 } else { 0 };
        let causal_offset = plan.causal.unwrap_or(0);
        let mut builder = self
            .device
            .stream
            .launch_builder(&self.attention.0[usize::from(dtype.is_some())]);
        builder
            .arg(query.storage.as_ref())
            .arg(key.storage.as_ref())
            .arg(value.storage.as_ref());
        // Disabled pointers receive an existing valid allocation. The flags
        // ensure that the kernel never dereferences it through the unused type.
        match mask {
            AttentionMask::Keep(tensor) => {
                builder.arg(tensor.storage.as_ref());
            }
            _ => {
                builder.arg(query.storage.as_ref());
            }
        }
        match mask {
            AttentionMask::Additive(tensor) => {
                builder.arg(tensor.storage.as_ref());
            }
            _ => {
                builder.arg(query.storage.as_ref());
            }
        }
        // Each block uniquely owns a row of dense result/scratch. All source
        // views and metadata were validated before the launch; no host reads.
        unsafe {
            builder
                .arg(output(&mut result))
                .arg(output(&mut scratch))
                .arg(&metadata)
                .arg(&rows)
                .arg(&keys)
                .arg(&depth)
                .arg(&value_depth)
                .arg(&row_rank)
                .arg(&plan.scale)
                .arg(&flags)
                .arg(&causal_offset);
            if let Some(dtype) = dtype.as_ref() {
                builder.arg(dtype);
            }
            builder.launch(LaunchConfig {
                grid_dim: (
                    rows.min(u64::from(self.device.multiprocessors) * 8) as u32,
                    1,
                    1,
                ),
                block_dim: (256, 1, 1),
                shared_mem_bytes: 0,
            })?;
        }
        Ok(result)
    }
}

impl TensorAttentionBackend for CudaRuntime {
    fn attention(
        &self,
        query: &CudaTensor,
        key: &CudaTensor,
        value: &CudaTensor,
        mask: AttentionMask<'_, CudaTensor, CudaTensor<u32>>,
        options: AttentionOptions,
    ) -> Result<CudaTensor, CudaError> {
        let plan = AttentionPlan::new(
            query.shape(),
            key.shape(),
            value.shape(),
            mask.shape(),
            options,
        )?;
        self.attention_loaded(query, key, value, mask, plan, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shape(dims: &[usize]) -> Shape {
        Shape::new(dims.to_vec()).unwrap()
    }

    #[test]
    fn metadata_preserves_offsets_strides_broadcast_and_grouped_heads() {
        let query = Layout::new(shape(&[2, 4, 3, 5]), vec![60, 15, 1, 3], 7).unwrap();
        let key = Layout::new(shape(&[1, 2, 7, 5]), vec![70, 35, 5, 1], 11).unwrap();
        let value = Layout::new(shape(&[2, 2, 7, 9]), vec![126, 63, 9, 1], 13).unwrap();
        let mask = Layout::new(shape(&[3, 7]), vec![7, 1], 17).unwrap();
        let plan = AttentionPlan::new(
            query.shape(),
            key.shape(),
            value.shape(),
            Some(mask.shape()),
            AttentionOptions::default(),
        )
        .unwrap();
        let data = attention_metadata(&plan, &query, &key, &value, Some(&mask)).unwrap();
        assert_eq!(&data[..3], &[2, 4, 3]);
        assert_eq!(&data[3..6], &[60, 15, 1]);
        assert_eq!(&data[6..9], &[0, 35, 0]);
        assert_eq!(&data[9..12], &[126, 63, 0]);
        assert_eq!(&data[12..15], &[0, 0, 7]);
        assert_eq!(&data[15..], &[7, 11, 13, 17, 3, 5, 1, 9, 1, 1, 2]);
    }

    #[test]
    fn matrix_promotion_is_a_view_even_with_transposed_strides() {
        let q = Layout::new(shape(&[3, 5]), vec![1, 3], 2).unwrap();
        let k = Layout::contiguous(shape(&[7, 5])).unwrap();
        let v = Layout::contiguous(shape(&[7, 9])).unwrap();
        let plan = AttentionPlan::new(
            q.shape(),
            k.shape(),
            v.shape(),
            None,
            AttentionOptions::default(),
        )
        .unwrap();
        let data = attention_metadata(&plan, &q, &k, &v, None).unwrap();
        assert_eq!(&data[..4], &[1, 3, 0, 1]);
        assert_eq!(&data[10..], &[2, 0, 0, 0, 3, 5, 1, 9, 1, 0, 1]);
        assert_eq!(plan.output.dims(), &[3, 9]);
    }
}
