use crate::CudaError;
use tensor_core::{BinaryOp, Layout, MatmulPrecision, ReduceOp, Shape};

/// Allocation identity in a prepared schedule. Views change only the layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BufferRef {
    Input(usize),
    Scratch(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedValue {
    pub buffer: BufferRef,
    pub layout: Layout,
}

/// Pure recording. Device-specific preparation expands reduction levels and
/// GEMM calls, then checks the complete resource budget before allocation.
#[derive(Clone, Debug)]
pub(crate) enum Step {
    Unary {
        source: PlannedValue,
        output: usize,
        op: u32,
        scale: f32,
        bias: f32,
    },
    Binary {
        left: PlannedValue,
        right: PlannedValue,
        output: usize,
        op: BinaryOp,
    },
    Reduce {
        source: PlannedValue,
        output: usize,
        op: ReduceOp,
        axes: Vec<usize>,
        keep_dims: bool,
        mean: bool,
    },
    Matmul {
        left: PlannedValue,
        right: PlannedValue,
        output: usize,
        precision: MatmulPrecision,
    },
}

#[derive(Debug, Default)]
pub(crate) struct CudaProgramPlan {
    pub inputs: Vec<Layout>,
    pub scratch: Vec<Shape>,
    pub steps: Vec<Step>,
    /// Every entry is copied to a separate caller-owned output, including
    /// repeated entries and direct input/view outputs.
    pub outputs: Vec<PlannedValue>,
}

/// CUDA allocates one sentinel element for an empty logical tensor.
pub(crate) fn allocation_bytes(shape: &Shape) -> Result<usize, CudaError> {
    shape
        .numel()
        .max(1)
        .checked_mul(size_of::<f32>())
        .ok_or(CudaError::InvalidInput("CUDA tensor byte count overflows"))
}

pub(crate) fn validate_layout(layout: &Layout) -> Result<(), CudaError> {
    allocation_bytes(layout.shape())?;
    crate::runtime::rank(layout.shape())?;
    layout
        .required_storage_len()?
        .max(1)
        .checked_mul(size_of::<f32>())
        .ok_or(CudaError::InvalidInput("CUDA layout byte span overflows"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_byte_limits_apply_to_views_and_empty_sentinels() {
        let empty = Shape::new(vec![0, usize::MAX]).unwrap();
        assert_eq!(allocation_bytes(&empty).unwrap(), 4);
        let empty_offset = Layout::new(empty, vec![0, 0], usize::MAX).unwrap();
        assert!(validate_layout(&empty_offset).is_err());

        let huge = Shape::new(vec![usize::MAX / 4 + 1]).unwrap();
        // A tiny zero-stride backing allocation does not waive logical limits.
        assert!(validate_layout(&Layout::new(huge, vec![0], 0).unwrap()).is_err());
        let sparse = Layout::new(Shape::new(vec![2]).unwrap(), vec![usize::MAX / 4], 0).unwrap();
        assert!(validate_layout(&sparse).is_err());
    }
}
