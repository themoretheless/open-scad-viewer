//! host tensor operations: conv1d.
use super::*;
impl Tensor {
/// 1D cross-correlation (`conv1d`) over a 3D input `[N, C_in, L]` with weight `[C_out, C_in, K]`,
    /// optional per-channel `bias` of length `C_out`, `stride >= 1`, and symmetric zero `padding`.
    pub fn conv1d(
        &self,
        weight: &Self,
        bias: Option<&[f32]>,
        stride: usize,
        padding: usize,
    ) -> Result<Self, TensorError> {
        let mut options = crate::ConvOptions::new(1);
        options.strides[0] = stride;
        options.padding_before[0] = padding;
        options.padding_after[0] = padding;
        let plan = crate::ConvPlan::new(&self.shape, &weight.shape, &options)?;
        let out_channels = weight.shape.dims()[0];
        if let Some(b) = bias
            && b.len() != out_channels
        {
            return Err(TensorError::ElementCountMismatch {
                expected: out_channels,
                actual: b.len(),
            });
        }
        if plan.output.numel() == 0 {
            return Ok(Self {
                shape: plan.output,
                data: Vec::new(),
            });
        }

        let n_batch = self.shape.dims()[0];
        let in_channels = self.shape.dims()[1];
        let in_len = self.shape.dims()[2];
        let kernel_size = weight.shape.dims()[2];
        let out_len = plan.output.dims()[2];
        let mut out = vec![0.0_f32; plan.output.numel()];

        for n in 0..n_batch {
            let n_in_base = n * in_channels * in_len;
            let n_out_base = n * out_channels * out_len;
            for oc in 0..out_channels {
                let b_val = bias.map_or(0.0_f64, |b| b[oc] as f64);
                let out_base = n_out_base + oc * out_len;
                let oc_w_base = oc * in_channels * kernel_size;
                for ol in 0..out_len {
                    let mut acc = b_val;
                    let in_start = (ol * stride) as isize - (padding as isize);
                    // Compute valid kernel range [k_min, k_max) once per output position to eliminate inner branches
                    let k_min = (-in_start).max(0) as usize;
                    let k_max = ((in_len as isize - in_start).max(0) as usize).min(kernel_size);
                    if k_min < k_max {
                        let valid_len = k_max - k_min;
                        let in_offset = (in_start + k_min as isize) as usize;
                        for ic in 0..in_channels {
                            let in_slice = &self.data[n_in_base + ic * in_len + in_offset
                                ..n_in_base + ic * in_len + in_offset + valid_len];
                            let w_slice = &weight.data[oc_w_base + ic * kernel_size + k_min
                                ..oc_w_base + ic * kernel_size + k_max];
                            for (&x, &w) in in_slice.iter().zip(w_slice.iter()) {
                                acc = (x as f64).mul_add(w as f64, acc);
                            }
                        }
                    }
                    out[out_base + ol] = acc as f32;
                }
            }
        }

        Ok(Self {
            shape: plan.output,
            data: out,
        })
    }
/// 1D max pooling over the trailing spatial dimension of a 2D `[C, L]` or 3D `[N, C, L]` tensor
    /// with window `kernel_size >= 1` and `stride >= 1`.
    pub fn max_pool1d(&self, kernel_size: usize, stride: usize) -> Result<Self, TensorError> {
        let rank = self.shape.rank();
        if rank != 2 && rank != 3 {
            return Err(TensorError::InvalidConvolution(
                "max_pool1d expects a 2D [C, L] or 3D [N, C, L] tensor",
            ));
        }
        if kernel_size == 0 || stride == 0 {
            return Err(TensorError::InvalidConvolution(
                "max_pool1d kernel_size and stride must be positive",
            ));
        }
        let dims = self.shape.dims();
        let in_len = dims[rank - 1];
        if in_len < kernel_size {
            return Err(TensorError::InvalidConvolution(
                "max_pool1d input length must be at least kernel_size",
            ));
        }
        let out_len = (in_len - kernel_size) / stride + 1;
        let mut out_dims = dims.to_vec();
        out_dims[rank - 1] = out_len;
        let out_shape = Shape::new(out_dims)?;
        if out_shape.numel() == 0 {
            return Ok(Self {
                shape: out_shape,
                data: Vec::new(),
            });
        }

        let outer: usize = dims[..rank - 1].iter().product();
        let mut out = Vec::with_capacity(outer * out_len);
        for c in 0..outer {
            let row = &self.data[c * in_len..(c + 1) * in_len];
            for ol in 0..out_len {
                let start = ol * stride;
                let window = &row[start..start + kernel_size];
                let mut max_val = f32::NEG_INFINITY;
                for &v in window {
                    if v > max_val || max_val.is_nan() {
                        max_val = v;
                    }
                }
                out.push(max_val);
            }
        }
        Ok(Self {
            shape: out_shape,
            data: out,
        })
    }
/// 1D average pooling over the trailing spatial dimension of a 2D `[C, L]` or 3D `[N, C, L]` tensor
    /// with window `kernel_size >= 1` and `stride >= 1`.
    pub fn avg_pool1d(&self, kernel_size: usize, stride: usize) -> Result<Self, TensorError> {
        let rank = self.shape.rank();
        if rank != 2 && rank != 3 {
            return Err(TensorError::InvalidConvolution(
                "avg_pool1d expects a 2D [C, L] or 3D [N, C, L] tensor",
            ));
        }
        if kernel_size == 0 || stride == 0 {
            return Err(TensorError::InvalidConvolution(
                "avg_pool1d kernel_size and stride must be positive",
            ));
        }
        let dims = self.shape.dims();
        let in_len = dims[rank - 1];
        if in_len < kernel_size {
            return Err(TensorError::InvalidConvolution(
                "avg_pool1d input length must be at least kernel_size",
            ));
        }
        let out_len = (in_len - kernel_size) / stride + 1;
        let mut out_dims = dims.to_vec();
        out_dims[rank - 1] = out_len;
        let out_shape = Shape::new(out_dims)?;
        if out_shape.numel() == 0 {
            return Ok(Self {
                shape: out_shape,
                data: Vec::new(),
            });
        }

        let inv_k = 1.0 / (kernel_size as f64);
        let outer: usize = dims[..rank - 1].iter().product();
        let mut out = Vec::with_capacity(outer * out_len);
        for c in 0..outer {
            let row = &self.data[c * in_len..(c + 1) * in_len];
            for ol in 0..out_len {
                let start = ol * stride;
                let window = &row[start..start + kernel_size];
                let mut sum = 0.0_f64;
                for &v in window {
                    sum += v as f64;
                }
                out.push((sum * inv_k) as f32);
            }
        }
        Ok(Self {
            shape: out_shape,
            data: out,
        })
    }
/// 1D inference batch normalization over channel axis (`axis = 0` for 1D `[C]`, `axis = 1` for
    /// 2D `[N, C]` or 3D `[N, C, L]`):
    /// `y = ((x - running_mean[c]) / sqrt(running_var[c] + eps)) * weight[c] + bias[c]`.
    pub fn batch_norm_1d(
        &self,
        running_mean: &[f32],
        running_var: &[f32],
        weight: Option<&[f32]>,
        bias: Option<&[f32]>,
        eps: f32,
    ) -> Result<Self, TensorError> {
        validate_epsilon(eps)?;
        let rank = self.shape.rank();
        if rank == 0 || rank > 3 {
            return Err(TensorError::RankMismatch {
                expected: 3,
                actual: rank,
            });
        }
        let dims = self.shape.dims();
        let (batch, channels, spatial) = match rank {
            1 => (1, dims[0], 1),
            2 => (dims[0], dims[1], 1),
            3 => (dims[0], dims[1], dims[2]),
            _ => unreachable!(),
        };
        if running_mean.len() != channels {
            return Err(TensorError::ElementCountMismatch {
                expected: channels,
                actual: running_mean.len(),
            });
        }
        if running_var.len() != channels {
            return Err(TensorError::ElementCountMismatch {
                expected: channels,
                actual: running_var.len(),
            });
        }
        if let Some(w) = weight
            && w.len() != channels
        {
            return Err(TensorError::ElementCountMismatch {
                expected: channels,
                actual: w.len(),
            });
        }
        if let Some(b) = bias
            && b.len() != channels
        {
            return Err(TensorError::ElementCountMismatch {
                expected: channels,
                actual: b.len(),
            });
        }
        if self.data.is_empty() {
            return Ok(self.clone());
        }

        // Precompute per-channel fused affine scale & shift: y = x * scale[c] + offset[c]
        let eps64 = eps as f64;
        let mut ch_scale = Vec::with_capacity(channels);
        let mut ch_offset = Vec::with_capacity(channels);
        for c in 0..channels {
            let mean = running_mean[c] as f64;
            let var = running_var[c] as f64;
            if !var.is_finite() || var < 0.0 {
                return Err(TensorError::InvalidEpsilon);
            }
            let gamma = weight.map_or(1.0_f64, |w| w[c] as f64);
            let beta = bias.map_or(0.0_f64, |b| b[c] as f64);
            let s = gamma / (var + eps64).sqrt();
            let o = beta - mean * s;
            ch_scale.push(s);
            ch_offset.push(o);
        }

        let mut out = vec![0.0_f32; self.data.len()];
        for n in 0..batch {
            let n_base = n * channels * spatial;
            for c in 0..channels {
                let s = ch_scale[c];
                let o = ch_offset[c];
                let c_base = n_base + c * spatial;
                let src_slice = &self.data[c_base..c_base + spatial];
                let dst_slice = &mut out[c_base..c_base + spatial];
                for (dst, &x) in dst_slice.iter_mut().zip(src_slice.iter()) {
                    *dst = (x as f64).mul_add(s, o) as f32;
                }
            }
        }

        Ok(Self {
            shape: self.shape.clone(),
            data: out,
        })
    }
/// Apply Rotary Position Embedding (RoPE) along the sequence dimension (`dims[rank - 2]`) and
    /// even-sized feature/head dimension (`dims[rank - 1]`) for 2D `[L, D]` or higher `[..., L, D]` tensors.
    ///
    /// Pairs of adjacent features `(x_{2i}, x_{2i+1})` at sequence position `m` are rotated by
    /// $\theta_{m, i} = m \cdot \text{base\_theta}^{-2i / D}$.
    pub fn apply_rotary_pos_emb(&self, base_theta: f32) -> Result<Self, TensorError> {
        if !base_theta.is_finite() || base_theta <= 0.0 {
            return Err(TensorError::InvalidAttention(
                "apply_rotary_pos_emb base_theta must be finite and positive",
            ));
        }
        let rank = self.shape.rank();
        if rank < 2 {
            return Err(TensorError::InvalidAttention(
                "apply_rotary_pos_emb expects rank >= 2 ([..., seq_len, head_dim])",
            ));
        }
        let dims = self.shape.dims();
        let seq_len = dims[rank - 2];
        let head_dim = dims[rank - 1];
        if !head_dim.is_multiple_of(2) || head_dim == 0 {
            return Err(TensorError::InvalidAttention(
                "apply_rotary_pos_emb requires a positive even head_dim",
            ));
        }
        if self.data.is_empty() {
            return Ok(self.clone());
        }

        let half_dim = head_dim / 2;
        let base64 = base_theta as f64;
        let inv_d = 1.0 / (head_dim as f64);
        let inv_freq: Vec<f64> = (0..half_dim)
            .map(|i| base64.powf(-((2 * i) as f64) * inv_d))
            .collect();

        // Precompute (cos, sin) table of shape [seq_len, half_dim]
        let mut cos_table = Vec::with_capacity(seq_len * half_dim);
        let mut sin_table = Vec::with_capacity(seq_len * half_dim);
        for m in 0..seq_len {
            let m_f = m as f64;
            for &freq in &inv_freq {
                let (s, c) = (m_f * freq).sin_cos();
                cos_table.push(c);
                sin_table.push(s);
            }
        }

        let outer: usize = dims[..rank - 2].iter().product();
        let mut out = vec![0.0_f32; self.data.len()];
        for b in 0..outer {
            let b_base = b * seq_len * head_dim;
            for m in 0..seq_len {
                let row_base = b_base + m * head_dim;
                let trig_base = m * half_dim;
                let src_row = &self.data[row_base..row_base + head_dim];
                let dst_row = &mut out[row_base..row_base + head_dim];
                for i in 0..half_dim {
                    let x0 = src_row[2 * i] as f64;
                    let x1 = src_row[2 * i + 1] as f64;
                    let c = cos_table[trig_base + i];
                    let s = sin_table[trig_base + i];
                    dst_row[2 * i] = (x0 * c - x1 * s) as f32;
                    dst_row[2 * i + 1] = (x0 * s + x1 * c) as f32;
                }
            }
        }

        Ok(Self {
            shape: self.shape.clone(),
            data: out,
        })
    }
/// Group normalization over `num_groups` channel groups (`axis = 0` for 1D `[C]`, `axis = 1`
    /// for `[N, C, ...]` tensors of rank $\ge 2$).
    ///
    /// Divides the `C` channels into `num_groups` equal-sized groups (`C % num_groups == 0`) and
    /// normalizes each `(n, g)` group slice of length `(C / num_groups) * spatial` to zero mean and
    /// unit variance using 4-wide FMA reductions.
    pub fn group_norm(&self, num_groups: usize, eps: f32) -> Result<Self, TensorError> {
        validate_epsilon(eps)?;
        let rank = self.shape.rank();
        if rank == 0 {
            return Err(TensorError::AxisOutOfBounds { axis: 0, rank: 0 });
        }
        let dims = self.shape.dims();
        let (batch, channels, spatial) = if rank == 1 {
            (1, dims[0], 1)
        } else {
            let sp: usize = dims[2..].iter().product();
            (dims[0], dims[1], sp)
        };
        if num_groups == 0 || channels == 0 || !channels.is_multiple_of(num_groups) {
            return Err(TensorError::ElementCountMismatch {
                expected: num_groups.max(1),
                actual: channels,
            });
        }
        if self.data.is_empty() {
            return Ok(self.clone());
        }

        let group_elems = (channels / num_groups) * spatial;
        if group_elems == 0 {
            return Ok(self.clone());
        }
        let inv_n = 1.0 / (group_elems as f64);
        let eps64 = eps as f64;
        let mut out = vec![0.0_f32; self.data.len()];
        let chunks4 = group_elems / 4;

        for (chunk, out_chunk) in self
            .data
            .chunks_exact(group_elems)
            .zip(out.chunks_exact_mut(group_elems))
        {
            let _ = batch;
            // Pass 1: 4-wide FMA mean reduction
            let mut s0 = 0.0_f64;
            let mut s1 = 0.0_f64;
            let mut s2 = 0.0_f64;
            let mut s3 = 0.0_f64;
            for c in 0..chunks4 {
                let b = c * 4;
                s0 += chunk[b] as f64;
                s1 += chunk[b + 1] as f64;
                s2 += chunk[b + 2] as f64;
                s3 += chunk[b + 3] as f64;
            }
            let mut sum = (s0 + s1) + (s2 + s3);
            for &v in &chunk[chunks4 * 4..] {
                sum += v as f64;
            }
            let mean = sum * inv_n;

            // Pass 2: 4-wide FMA centered variance reduction
            let mut v0 = 0.0_f64;
            let mut v1 = 0.0_f64;
            let mut v2 = 0.0_f64;
            let mut v3 = 0.0_f64;
            for c in 0..chunks4 {
                let b = c * 4;
                let d0 = chunk[b] as f64 - mean;
                let d1 = chunk[b + 1] as f64 - mean;
                let d2 = chunk[b + 2] as f64 - mean;
                let d3 = chunk[b + 3] as f64 - mean;
                v0 = d0.mul_add(d0, v0);
                v1 = d1.mul_add(d1, v1);
                v2 = d2.mul_add(d2, v2);
                v3 = d3.mul_add(d3, v3);
            }
            let mut var_sum = (v0 + v1) + (v2 + v3);
            for &v in &chunk[chunks4 * 4..] {
                let d = v as f64 - mean;
                var_sum = d.mul_add(d, var_sum);
            }
            let var = var_sum * inv_n;
            let inv_std = 1.0 / (var + eps64).sqrt();

            for (dst, &v) in out_chunk.iter_mut().zip(chunk.iter()) {
                *dst = ((v as f64 - mean) * inv_std) as f32;
            }
        }

        Ok(Self {
            shape: self.shape.clone(),
            data: out,
        })
    }
/// Fused SwiGLU gated activation $\operatorname{SwiGLU}(x, g) = \operatorname{SiLU}(x) \odot g = \frac{x}{1 + e^{-x}} \cdot g$
    /// in a single pass over identically-shaped tensors.
    pub fn swiglu(&self, gate: &Self) -> Result<Self, TensorError> {
        if self.shape != gate.shape {
            return Err(TensorError::IncompatibleBroadcast {
                left: self.shape.dims().to_vec(),
                right: gate.shape.dims().to_vec(),
            });
        }
        let data = self
            .data
            .iter()
            .zip(gate.data.iter())
            .map(|(&x, &g)| {
                let xf = x as f64;
                let gf = g as f64;
                let silu = xf / (1.0 + (-xf).exp());
                (silu * gf) as f32
            })
            .collect();
        Ok(Self {
            shape: self.shape.clone(),
            data,
        })
    }
}
