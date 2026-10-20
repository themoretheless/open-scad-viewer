use crate::{HasShape, Shape, TensorError, validate_epsilon};

/// Result of a `topk` selection along the last axis.
#[derive(Clone, Debug, PartialEq)]
pub struct TopKResult {
    pub values: Tensor,
    pub indices: Vec<u32>,
}

/// Host reference tensor with checked [`Shape`] and contiguous `f32` storage.
#[derive(Clone, Debug, PartialEq)]
pub struct Tensor {
    shape: Shape,
    data: Vec<f32>,
}

impl HasShape for Tensor {
    fn shape(&self) -> &Shape {
        &self.shape
    }
}

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
        let mut out = Vec::with_capacity(self.data.len());
        for chunk in self.data.chunks(group_len) {
            let n = chunk.len() as f64;
            let mean = chunk.iter().map(|&v| v as f64).sum::<f64>() / n;
            let var = chunk
                .iter()
                .map(|&v| {
                    let d = v as f64 - mean;
                    d * d
                })
                .sum::<f64>()
                / n;
            let inv_std = 1.0 / (var + eps).sqrt();
            for &v in chunk {
                out.push(((v as f64 - mean) * inv_std) as f32);
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
        let mut out = Vec::with_capacity(self.data.len());
        for chunk in self.data.chunks(group_len) {
            let n = chunk.len() as f64;
            let mean_sq = chunk
                .iter()
                .map(|&v| {
                    let x = v as f64;
                    x * x
                })
                .sum::<f64>()
                / n;
            let inv_rms = 1.0 / (mean_sq + eps).sqrt();
            for &v in chunk {
                out.push((v as f64 * inv_rms) as f32);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_norm_and_rms_norm_normalize_rows() {
        let t = Tensor::new(vec![2, 4], vec![1.0, 2.0, 3.0, 4.0, 10.0, 10.0, 10.0, 10.0]).unwrap();
        let ln = t.layer_norm(1e-5).unwrap();
        assert_eq!(ln.shape().dims(), &[2, 4]);
        let row0 = &ln.data()[0..4];
        let mean0: f32 = row0.iter().sum::<f32>() / 4.0;
        let var0: f32 = row0.iter().map(|v| v * v).sum::<f32>() / 4.0;
        assert!(mean0.abs() < 1e-6);
        assert!((var0 - 1.0).abs() < 1e-4);
        // Constant row normalizes to 0
        for &v in &ln.data()[4..8] {
            assert!(v.abs() < 1e-6);
        }

        let rms = t.rms_norm(1e-5).unwrap();
        let rms_sq0: f32 = rms.data()[0..4].iter().map(|v| v * v).sum::<f32>() / 4.0;
        assert!((rms_sq0 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn gelu_and_silu_match_reference_values() {
        let t = Tensor::new(vec![3], vec![-2.0, 0.0, 2.0]).unwrap();
        let g = t.gelu();
        let s = t.silu();
        assert!(g.data()[1].abs() < 1e-7);
        assert!(s.data()[1].abs() < 1e-7);
        assert!(g.data()[2] > 1.9 && g.data()[2] < 2.0);
        assert!(s.data()[2] > 1.7 && s.data()[2] < 1.8);
        assert!(g.data()[0] < 0.0 && g.data()[0] > -0.1);
    }

    #[test]
    fn cosine_similarity_and_topk_compute_expected_outputs() {
        let a = Tensor::new(vec![2, 3], vec![1.0, 0.0, 0.0, 1.0, 2.0, 2.0]).unwrap();
        let b = Tensor::new(vec![2, 3], vec![0.0, 1.0, 0.0, 2.0, 4.0, 4.0]).unwrap();
        let sim = a.cosine_similarity(&b, 1e-8).unwrap();
        assert_eq!(sim.shape().dims(), &[2]);
        assert!(sim.data()[0].abs() < 1e-6);
        assert!((sim.data()[1] - 1.0).abs() < 1e-6);

        let top = a.topk(2).unwrap();
        assert_eq!(top.values.shape().dims(), &[2, 2]);
        assert_eq!(top.values.data(), &[1.0, 0.0, 2.0, 2.0]);
        assert_eq!(top.indices, vec![0, 1, 1, 2]);
    }
}
