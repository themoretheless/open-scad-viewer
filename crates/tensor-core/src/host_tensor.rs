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



#[cfg(test)]
#[path = "host_tensor/tests/mod.rs"]
mod tests;

#[path="host_tensor/methods_Tensor_new.rs"]
mod methods_Tensor_new;
#[path="host_tensor/methods_Tensor_conv1d.rs"]
mod methods_Tensor_conv1d;
