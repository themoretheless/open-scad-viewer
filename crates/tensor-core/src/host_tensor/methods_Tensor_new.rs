//! host tensor operations: new.
use super::*;
impl Tensor {
pub fn new(dims: impl Into<Vec<usize>>, data: impl Into<Vec<f32>>) -> Result<Self, TensorError> {
        let shape = Shape::new(dims)?;
        let data = data.into();
        if data.len() != shape.numel() {
            return Err(TensorError::ElementCountMismatch {
                expected: shape.numel(),
                actual: data.len(),
            });
        }
        Ok(Self { shape, data })
    }
pub fn from_shape(shape: Shape, data: Vec<f32>) -> Result<Self, TensorError> {
        if data.len() != shape.numel() {
            return Err(TensorError::ElementCountMismatch {
                expected: shape.numel(),
                actual: data.len(),
            });
        }
        Ok(Self { shape, data })
    }
#[inline]
    pub fn shape(&self) -> &Shape {
        &self.shape
    }
#[inline]
    pub fn data(&self) -> &[f32] {
        &self.data
    }
#[inline]
    pub fn into_data(self) -> Vec<f32> {
        self.data
    }
/// Gaussian Error Linear Unit (exact erf-based formula via tanh approximation or exact polynomial).
    /// Uses `0.5 * x * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))`.
    pub fn gelu(&self) -> Self {
        const SQRT_2_OVER_PI: f64 = 0.797_884_560_802_865_4;
        const COEFF: f64 = 0.044_715;
        let data = self
            .data
            .iter()
            .map(|&v| {
                let x = v as f64;
                let inner = SQRT_2_OVER_PI * (x + COEFF * x * x * x);
                (0.5 * x * (1.0 + inner.tanh())) as f32
            })
            .collect();
        Self {
            shape: self.shape.clone(),
            data,
        }
    }
/// Sigmoid Linear Unit (Swish-1): `x / (1 + exp(-x))`.
    pub fn silu(&self) -> Self {
        let data = self
            .data
            .iter()
            .map(|&v| {
                let x = v as f64;
                (x / (1.0 + (-x).exp())) as f32
            })
            .collect();
        Self {
            shape: self.shape.clone(),
            data,
        }
    }
/// Layer normalization `(x - mean) / sqrt(var + eps)` over the trailing dimension (or all elements if rank <= 1).
    pub fn layer_norm(&self, epsilon: f32) -> Result<Self, TensorError> {
        validate_epsilon(epsilon)?;
        if self.data.is_empty() {
            return Ok(self.clone());
        }
        let group_len = *self.shape.dims().last().unwrap_or(&self.data.len());
        if group_len == 0 {
            return Ok(self.clone());
        }
        let eps = epsilon as f64;
        let inv_n = 1.0 / (group_len as f64);
        let mut out = vec![0.0_f32; self.data.len()];
        for (chunk, out_chunk) in self.data.chunks_exact(group_len).zip(out.chunks_exact_mut(group_len)) {
            let mut sum = 0.0_f64;
            let mut sum_sq = 0.0_f64;
            for &v in chunk {
                let x = v as f64;
                sum += x;
                sum_sq = x.mul_add(x, sum_sq);
            }
            let mean = sum * inv_n;
            // Two-pass variance for numerical stability on large offsets
            let mut var_acc = 0.0_f64;
            for &v in chunk {
                let d = v as f64 - mean;
                var_acc = d.mul_add(d, var_acc);
            }
            let _ = sum_sq;
            let var = var_acc * inv_n;
            let inv_std = 1.0 / (var + eps).sqrt();
            for (dst, &v) in out_chunk.iter_mut().zip(chunk.iter()) {
                *dst = ((v as f64 - mean) * inv_std) as f32;
            }
        }
        Ok(Self {
            shape: self.shape.clone(),
            data: out,
        })
    }
/// Root Mean Square normalization `x / sqrt(mean(x^2) + eps)` over the trailing dimension.
    pub fn rms_norm(&self, epsilon: f32) -> Result<Self, TensorError> {
        validate_epsilon(epsilon)?;
        if self.data.is_empty() {
            return Ok(self.clone());
        }
        let group_len = *self.shape.dims().last().unwrap_or(&self.data.len());
        if group_len == 0 {
            return Ok(self.clone());
        }
        let eps = epsilon as f64;
        let inv_n = 1.0 / (group_len as f64);
        let mut out = vec![0.0_f32; self.data.len()];
        for (chunk, out_chunk) in self.data.chunks_exact(group_len).zip(out.chunks_exact_mut(group_len)) {
            let mut sum_sq = 0.0_f64;
            for &v in chunk {
                let x = v as f64;
                sum_sq = x.mul_add(x, sum_sq);
            }
            let mean_sq = sum_sq * inv_n;
            let inv_rms = 1.0 / (mean_sq + eps).sqrt();
            for (dst, &v) in out_chunk.iter_mut().zip(chunk.iter()) {
                *dst = (v as f64 * inv_rms) as f32;
            }
        }
        Ok(Self {
            shape: self.shape.clone(),
            data: out,
        })
    }
/// Cosine similarity between two identically-shaped tensors along the last dimension.
    pub fn cosine_similarity(&self, other: &Self, epsilon: f32) -> Result<Self, TensorError> {
        validate_epsilon(epsilon)?;
        if self.shape != other.shape {
            return Err(TensorError::IncompatibleBroadcast {
                left: self.shape.dims().to_vec(),
                right: other.shape.dims().to_vec(),
            });
        }
        if self.shape.rank() == 0 {
            return Err(TensorError::AxisOutOfBounds { axis: 0, rank: 0 });
        }
        let last_axis = self.shape.rank() - 1;
        let out_shape = self.shape.reduce(&[last_axis], false)?;
        let group_len = self.shape.dims()[last_axis];
        if group_len == 0 {
            return Err(TensorError::EmptyReduction);
        }
        let eps = epsilon as f64;
        let mut out = Vec::with_capacity(out_shape.numel());
        for (a_chunk, b_chunk) in self.data.chunks(group_len).zip(other.data.chunks(group_len)) {
            let mut dot = 0.0_f64;
            let mut norm_a = 0.0_f64;
            let mut norm_b = 0.0_f64;
            for (&a, &b) in a_chunk.iter().zip(b_chunk.iter()) {
                let af = a as f64;
                let bf = b as f64;
                dot += af * bf;
                norm_a += af * af;
                norm_b += bf * bf;
            }
            let denom = (norm_a.sqrt() * norm_b.sqrt()).max(eps);
            out.push((dot / denom) as f32);
        }
        Ok(Self {
            shape: out_shape,
            data: out,
        })
    }
/// Select the top `k` largest elements (sorted descending, tie-breaking by lowest index) along the last axis.
    pub fn topk(&self, k: usize) -> Result<TopKResult, TensorError> {
        if self.shape.rank() == 0 {
            return Err(TensorError::AxisOutOfBounds { axis: 0, rank: 0 });
        }
        let rank = self.shape.rank();
        let last_dim = self.shape.dims()[rank - 1];
        if k > last_dim {
            return Err(TensorError::InvalidSlice {
                axis: rank - 1,
                start: 0,
                len: k,
                dimension: last_dim,
            });
        }
        let mut out_dims = self.shape.dims().to_vec();
        out_dims[rank - 1] = k;
        let out_shape = Shape::new(out_dims)?;
        if k == 0 || self.data.is_empty() {
            return Ok(TopKResult {
                values: Self {
                    shape: out_shape,
                    data: Vec::new(),
                },
                indices: Vec::new(),
            });
        }
        let num_groups = self.data.len() / last_dim;
        let mut values = Vec::with_capacity(num_groups * k);
        let mut indices = Vec::with_capacity(num_groups * k);
        let mut row: Vec<(u32, f32)> = Vec::with_capacity(last_dim);
        for chunk in self.data.chunks(last_dim) {
            row.clear();
            row.extend(
                chunk
                    .iter()
                    .enumerate()
                    .map(|(idx, &val)| (idx as u32, val)),
            );
            row.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            for &(idx, val) in &row[..k] {
                values.push(val);
                indices.push(idx);
            }
        }
        Ok(TopKResult {
            values: Self {
                shape: out_shape,
                data: values,
            },
            indices,
        })
    }
/// Numerically stable softmax `exp(x - max(x)) / sum(exp(x - max(x)))` along `axis`.
    pub fn softmax(&self, axis: usize) -> Result<Self, TensorError> {
        let rank = self.shape.rank();
        if axis >= rank {
            return Err(TensorError::AxisOutOfBounds { axis, rank });
        }
        let dims = self.shape.dims();
        let axis_len = dims[axis];
        if axis_len == 0 {
            return Err(TensorError::EmptyReduction);
        }
        if self.data.is_empty() {
            return Ok(self.clone());
        }
        let outer: usize = dims[..axis].iter().product();
        let inner: usize = dims[axis + 1..].iter().product();
        let mut out = vec![0.0_f32; self.data.len()];
        for o in 0..outer {
            let base_o = o * axis_len * inner;
            for i in 0..inner {
                let mut max_val = f64::NEG_INFINITY;
                for k in 0..axis_len {
                    let v = self.data[base_o + k * inner + i] as f64;
                    if v > max_val {
                        max_val = v;
                    }
                }
                let mut sum = 0.0_f64;
                for k in 0..axis_len {
                    let idx = base_o + k * inner + i;
                    let e = (self.data[idx] as f64 - max_val).exp();
                    out[idx] = e as f32;
                    sum += e;
                }
                let inv_sum = if sum > 0.0 { 1.0 / sum } else { 0.0 };
                for k in 0..axis_len {
                    let idx = base_o + k * inner + i;
                    out[idx] = (out[idx] as f64 * inv_sum) as f32;
                }
            }
        }
        Ok(Self {
            shape: self.shape.clone(),
            data: out,
        })
    }
/// Numerically stable log-softmax `(x - max(x)) - ln(sum(exp(x - max(x))))` along `axis`.
    pub fn log_softmax(&self, axis: usize) -> Result<Self, TensorError> {
        let rank = self.shape.rank();
        if axis >= rank {
            return Err(TensorError::AxisOutOfBounds { axis, rank });
        }
        let dims = self.shape.dims();
        let axis_len = dims[axis];
        if axis_len == 0 {
            return Err(TensorError::EmptyReduction);
        }
        if self.data.is_empty() {
            return Ok(self.clone());
        }
        let outer: usize = dims[..axis].iter().product();
        let inner: usize = dims[axis + 1..].iter().product();
        let mut out = vec![0.0_f32; self.data.len()];
        for o in 0..outer {
            let base_o = o * axis_len * inner;
            for i in 0..inner {
                let mut max_val = f64::NEG_INFINITY;
                for k in 0..axis_len {
                    let v = self.data[base_o + k * inner + i] as f64;
                    if v > max_val {
                        max_val = v;
                    }
                }
                let mut sum_exp = 0.0_f64;
                for k in 0..axis_len {
                    let idx = base_o + k * inner + i;
                    sum_exp += (self.data[idx] as f64 - max_val).exp();
                }
                let log_sum_exp = max_val + sum_exp.ln();
                for k in 0..axis_len {
                    let idx = base_o + k * inner + i;
                    out[idx] = (self.data[idx] as f64 - log_sum_exp) as f32;
                }
            }
        }
        Ok(Self {
            shape: self.shape.clone(),
            data: out,
        })
    }
/// Elementwise clamp each value to `[min_val, max_val]`.
    pub fn clamp(&self, min_val: f32, max_val: f32) -> Self {
        let (lo, hi) = if min_val <= max_val {
            (min_val, max_val)
        } else {
            (max_val, min_val)
        };
        let data = self.data.iter().map(|&v| v.clamp(lo, hi)).collect();
        Self {
            shape: self.shape.clone(),
            data,
        }
    }
/// Elementwise ternary selection `condition[i] ? a[i] : b[i]` for identically-shaped tensors.
    pub fn where_select(condition: &[bool], a: &Self, b: &Self) -> Result<Self, TensorError> {
        if a.shape != b.shape {
            return Err(TensorError::IncompatibleBroadcast {
                left: a.shape.dims().to_vec(),
                right: b.shape.dims().to_vec(),
            });
        }
        if condition.len() != a.data.len() {
            return Err(TensorError::ElementCountMismatch {
                expected: a.data.len(),
                actual: condition.len(),
            });
        }
        let data = condition
            .iter()
            .zip(a.data.iter().zip(b.data.iter()))
            .map(|(&cond, (&av, &bv))| if cond { av } else { bv })
            .collect();
        Ok(Self {
            shape: a.shape.clone(),
            data,
        })
    }
/// L2 normalization `x / max(||x||_2, eps)` along `axis`.
    pub fn l2_normalize(&self, axis: usize, eps: f32) -> Result<Self, TensorError> {
        validate_epsilon(eps)?;
        let rank = self.shape.rank();
        if axis >= rank {
            return Err(TensorError::AxisOutOfBounds { axis, rank });
        }
        let dims = self.shape.dims();
        let axis_len = dims[axis];
        if axis_len == 0 {
            return Err(TensorError::EmptyReduction);
        }
        if self.data.is_empty() {
            return Ok(self.clone());
        }
        let outer: usize = dims[..axis].iter().product();
        let inner: usize = dims[axis + 1..].iter().product();
        let eps64 = eps as f64;
        let mut out = vec![0.0_f32; self.data.len()];
        for o in 0..outer {
            let base_o = o * axis_len * inner;
            for i in 0..inner {
                let mut sum_sq = 0.0_f64;
                for k in 0..axis_len {
                    let v = self.data[base_o + k * inner + i] as f64;
                    sum_sq += v * v;
                }
                let denom = sum_sq.sqrt().max(eps64);
                let inv = 1.0 / denom;
                for k in 0..axis_len {
                    let idx = base_o + k * inner + i;
                    out[idx] = (self.data[idx] as f64 * inv) as f32;
                }
            }
        }
        Ok(Self {
            shape: self.shape.clone(),
            data: out,
        })
    }
/// Reference CPU scaled dot-product attention $\operatorname{softmax}\!\left(\frac{Q K^T}{\sqrt{d_k}} + M\right) V$
    /// for 2D `[Lq, d_k]` or 3D `[H, Lq, d_k]` (and higher batch-prefixed) tensors, with optional
    /// upper-left causal masking (`j <= i`). Supports Grouped-Query Attention (GQA) when `Q` has a
    /// head count that is a multiple of `K`/`V` head count.
    pub fn scaled_dot_product_attention(
        q: &Self,
        k: &Self,
        v: &Self,
        causal_mask: bool,
    ) -> Result<Self, TensorError> {
        let options = crate::AttentionOptions {
            scale: None,
            causal: if causal_mask { Some(0) } else { None },
        };
        let plan = crate::AttentionPlan::new(q.shape(), k.shape(), v.shape(), None, options)?;
        if plan.output.numel() == 0 {
            return Ok(Self {
                shape: plan.output,
                data: Vec::new(),
            });
        }
        if plan.keys == 0 {
            return Ok(Self {
                shape: plan.output.clone(),
                data: vec![0.0; plan.output.numel()],
            });
        }

        let batch_elems = plan.batch.numel().max(1);
        let q_batch_elems = q.data.len() / (plan.query_heads * plan.queries * plan.depth).max(1);
        let k_batch_elems = k.data.len() / (plan.key_heads * plan.keys * plan.depth).max(1);
        let v_batch_elems = v.data.len() / (plan.key_heads * plan.keys * plan.value_depth).max(1);

        let scale = plan.scale as f64;
        let mut out = vec![0.0_f32; plan.output.numel()];
        let mut logits = vec![0.0_f64; plan.keys];

        for b in 0..batch_elems {
            let qb = if q_batch_elems <= 1 { 0 } else { b % q_batch_elems };
            let kb = if k_batch_elems <= 1 { 0 } else { b % k_batch_elems };
            let vb = if v_batch_elems <= 1 { 0 } else { b % v_batch_elems };

            for h in 0..plan.query_heads {
                let kh = h / plan.group_size;
                let q_head_base = (qb * plan.query_heads + h) * plan.queries * plan.depth;
                let k_head_base = (kb * plan.key_heads + kh) * plan.keys * plan.depth;
                let v_head_base = (vb * plan.key_heads + kh) * plan.keys * plan.value_depth;
                let out_head_base = (b * plan.query_heads + h) * plan.queries * plan.value_depth;

                for qi in 0..plan.queries {
                    let q_row = &q.data[q_head_base + qi * plan.depth..q_head_base + (qi + 1) * plan.depth];
                    let mut max_logit = f64::NEG_INFINITY;
                    let max_allowed_j = if causal_mask {
                        qi.min(plan.keys.saturating_sub(1))
                    } else {
                        plan.keys - 1
                    };

                    for (kj, logit_slot) in logits.iter_mut().enumerate().take(plan.keys) {
                        if causal_mask && kj > max_allowed_j {
                            *logit_slot = f64::NEG_INFINITY;
                            continue;
                        }
                        let k_row = &k.data[k_head_base + kj * plan.depth..k_head_base + (kj + 1) * plan.depth];
                        // 4-wide unrolled FMA dot product for NEON/AVX auto-vectorization
                        let mut acc0 = 0.0_f64;
                        let mut acc1 = 0.0_f64;
                        let mut acc2 = 0.0_f64;
                        let mut acc3 = 0.0_f64;
                        let chunks4 = plan.depth / 4;
                        for c in 0..chunks4 {
                            let base = c * 4;
                            acc0 = (q_row[base] as f64).mul_add(k_row[base] as f64, acc0);
                            acc1 = (q_row[base + 1] as f64).mul_add(k_row[base + 1] as f64, acc1);
                            acc2 = (q_row[base + 2] as f64).mul_add(k_row[base + 2] as f64, acc2);
                            acc3 = (q_row[base + 3] as f64).mul_add(k_row[base + 3] as f64, acc3);
                        }
                        let mut dot = (acc0 + acc1) + (acc2 + acc3);
                        for d in (chunks4 * 4)..plan.depth {
                            dot = (q_row[d] as f64).mul_add(k_row[d] as f64, dot);
                        }
                        let s = dot * scale;
                        *logit_slot = s;
                        if s > max_logit {
                            max_logit = s;
                        }
                    }

                    if !max_logit.is_finite() {
                        continue;
                    }

                    let mut sum_exp = 0.0_f64;
                    for logit_slot in logits.iter_mut().take(plan.keys) {
                        if logit_slot.is_finite() {
                            let w = (*logit_slot - max_logit).exp();
                            *logit_slot = w;
                            sum_exp += w;
                        } else {
                            *logit_slot = 0.0;
                        }
                    }
                    let inv_sum = if sum_exp > 0.0 { 1.0 / sum_exp } else { 0.0 };
                    let out_row = &mut out[out_head_base + qi * plan.value_depth
                        ..out_head_base + (qi + 1) * plan.value_depth];
                    for (kj, &raw_w) in logits.iter().enumerate().take(plan.keys) {
                        let w = raw_w * inv_sum;
                        if w == 0.0 {
                            continue;
                        }
                        let v_row = &v.data[v_head_base + kj * plan.value_depth
                            ..v_head_base + (kj + 1) * plan.value_depth];
                        for (dst, &vv) in out_row.iter_mut().zip(v_row.iter()) {
                            *dst = (w as f32).mul_add(vv, *dst);
                        }
                    }
                }
            }
        }

        Ok(Self {
            shape: plan.output,
            data: out,
        })
    }
}
