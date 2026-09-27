use super::plan::{BufferRef, CudaProgramPlan, PlannedValue, Step, validate_layout};
use crate::CudaError;
use std::sync::atomic::{AtomicU64, Ordering};
use tensor_core::{
    BinaryOp, Layout, MatmulPlan, MatmulPrecision, ReduceOp, Shape, TensorError, UnaryOp,
    mean_shape, reduction_shape,
};

/// A value belongs to exactly one CUDA program builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaValue {
    owner: u64,
    index: usize,
}

/// CPU-only recording used by the runtime-borrowing public builder. No CUDA
/// handles, tensor owners, storage allocations or metadata uploads are retained.
pub(crate) struct CudaProgramPlanBuilder {
    owner: u64,
    values: Vec<PlannedValue>,
    plan: CudaProgramPlan,
}

impl CudaProgramPlanBuilder {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let owner = NEXT
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .expect("CUDA program identity space exhausted");
        Self {
            owner,
            values: Vec::new(),
            plan: CudaProgramPlan::default(),
        }
    }

    fn value(&self, value: CudaValue) -> Result<PlannedValue, CudaError> {
        if value.owner != self.owner {
            return Err(CudaError::InvalidInput(
                "value belongs to a different CUDA program builder",
            ));
        }
        self.values
            .get(value.index)
            .cloned()
            .ok_or(CudaError::InvalidInput("invalid CUDA program value"))
    }

    fn push(&mut self, value: PlannedValue) -> CudaValue {
        let index = self.values.len();
        self.values.push(value);
        CudaValue {
            owner: self.owner,
            index,
        }
    }

    fn transaction<T>(
        &mut self,
        record: impl FnOnce(&mut Self) -> Result<T, CudaError>,
    ) -> Result<T, CudaError> {
        let checkpoint = (
            self.values.len(),
            self.plan.inputs.len(),
            self.plan.scratch.len(),
            self.plan.steps.len(),
        );
        let result = record(self);
        if result.is_err() {
            self.values.truncate(checkpoint.0);
            self.plan.inputs.truncate(checkpoint.1);
            self.plan.scratch.truncate(checkpoint.2);
            self.plan.steps.truncate(checkpoint.3);
        }
        result
    }

    fn output(
        &mut self,
        shape: Shape,
        step: impl FnOnce(usize) -> Step,
    ) -> Result<CudaValue, CudaError> {
        let layout = Layout::contiguous(shape.clone())?;
        validate_layout(&layout)?;
        let output = self.plan.scratch.len();
        self.plan.scratch.push(shape);
        self.plan.steps.push(step(output));
        Ok(self.push(PlannedValue {
            buffer: BufferRef::Scratch(output),
            layout,
        }))
    }

    pub fn input(&mut self, layout: Layout) -> Result<CudaValue, CudaError> {
        validate_layout(&layout)?;
        let input = self.plan.inputs.len();
        self.plan.inputs.push(layout.clone());
        Ok(self.push(PlannedValue {
            buffer: BufferRef::Input(input),
            layout,
        }))
    }

    fn unary_op(
        &mut self,
        value: CudaValue,
        op: u32,
        scale: f32,
        bias: f32,
    ) -> Result<CudaValue, CudaError> {
        let source = self.value(value)?;
        self.output(source.layout.shape().clone(), |output| Step::Unary {
            source,
            output,
            op,
            scale,
            bias,
        })
    }

    pub fn unary(&mut self, value: CudaValue, op: UnaryOp) -> Result<CudaValue, CudaError> {
        self.unary_op(value, op as u32, 1., 0.)
    }

    pub fn affine(
        &mut self,
        value: CudaValue,
        scale: f32,
        bias: f32,
    ) -> Result<CudaValue, CudaError> {
        self.unary_op(value, 9, scale, bias)
    }

    pub fn binary(
        &mut self,
        left: CudaValue,
        right: CudaValue,
        op: BinaryOp,
    ) -> Result<CudaValue, CudaError> {
        let mut left = self.value(left)?;
        let mut right = self.value(right)?;
        let shape = left.layout.shape().broadcast(right.layout.shape())?;
        left.layout = left.layout.broadcast_to(shape.clone())?;
        right.layout = right.layout.broadcast_to(shape.clone())?;
        validate_layout(&left.layout)?;
        validate_layout(&right.layout)?;
        self.output(shape, |output| Step::Binary {
            left,
            right,
            output,
            op,
        })
    }

    pub fn materialize(&mut self, value: CudaValue) -> Result<CudaValue, CudaError> {
        if self.value(value)?.layout.is_contiguous() {
            Ok(value)
        } else {
            self.unary_op(value, 10, 1., 0.)
        }
    }

    pub fn reshape(&mut self, value: CudaValue, shape: Shape) -> Result<CudaValue, CudaError> {
        let source = self.value(value)?;
        if source.layout.shape().numel() != shape.numel() {
            return Err(TensorError::ElementCountMismatch {
                expected: source.layout.shape().numel(),
                actual: shape.numel(),
            }
            .into());
        }
        self.transaction(|this| {
            let contiguous = this.materialize(value)?;
            let mut source = this.value(contiguous)?;
            source.layout = source.layout.reshape(shape)?;
            validate_layout(&source.layout)?;
            Ok(this.push(source))
        })
    }

    pub fn permute(&mut self, value: CudaValue, axes: &[usize]) -> Result<CudaValue, CudaError> {
        let mut source = self.value(value)?;
        source.layout = source.layout.permute(axes)?;
        validate_layout(&source.layout)?;
        Ok(self.push(source))
    }

    pub fn broadcast_to(&mut self, value: CudaValue, shape: Shape) -> Result<CudaValue, CudaError> {
        let mut source = self.value(value)?;
        source.layout = source.layout.broadcast_to(shape)?;
        validate_layout(&source.layout)?;
        Ok(self.push(source))
    }

    pub fn reduce(
        &mut self,
        value: CudaValue,
        op: ReduceOp,
        axes: &[usize],
        keep_dims: bool,
    ) -> Result<CudaValue, CudaError> {
        self.reduction(value, op, axes, keep_dims, false)
    }

    pub fn sum_axes(
        &mut self,
        value: CudaValue,
        axes: &[usize],
        keep_dims: bool,
    ) -> Result<CudaValue, CudaError> {
        self.reduce(value, ReduceOp::Sum, axes, keep_dims)
    }

    pub fn mean(
        &mut self,
        value: CudaValue,
        axes: &[usize],
        keep_dims: bool,
    ) -> Result<CudaValue, CudaError> {
        self.reduction(value, ReduceOp::Sum, axes, keep_dims, true)
    }

    fn reduction(
        &mut self,
        value: CudaValue,
        op: ReduceOp,
        axes: &[usize],
        keep_dims: bool,
        mean: bool,
    ) -> Result<CudaValue, CudaError> {
        let source = self.value(value)?;
        let shape = if mean {
            mean_shape(source.layout.shape(), axes, keep_dims)?
        } else {
            reduction_shape(op, source.layout.shape(), axes, keep_dims)?
        };
        if axes.is_empty() {
            return self.materialize(value);
        }
        self.output(shape, |output| Step::Reduce {
            source,
            output,
            op,
            axes: axes.to_vec(),
            keep_dims,
            mean,
        })
    }

    pub fn matmul(
        &mut self,
        left: CudaValue,
        right: CudaValue,
        precision: MatmulPrecision,
    ) -> Result<CudaValue, CudaError> {
        let a = self.value(left)?;
        let b = self.value(right)?;
        let plan = MatmulPlan::new(a.layout.shape(), b.layout.shape())?;
        let active = !plan.matrix_output.is_empty() && *plan.left.dims().last().unwrap() != 0;
        // Reject an impossible BLAS call before materializing either operand.
        if active {
            let dims = plan.matrix_output.dims();
            crate::policy::row_major_gemm(
                dims[dims.len() - 2],
                *plan.left.dims().last().unwrap(),
                dims[dims.len() - 1],
            )?;
        }
        self.transaction(|this| {
            let (mut a, mut b) = if active {
                let left = this.materialize(left)?;
                let right = this.materialize(right)?;
                (this.value(left)?, this.value(right)?)
            } else {
                (a, b)
            };
            a.layout = promote(a.layout, plan.left, true)?;
            b.layout = promote(b.layout, plan.right, false)?;
            // Retain even empty/K=0 operations so preparation still validates
            // requested precision and emits a fresh zero fill where required.
            this.output(plan.output, |output| Step::Matmul {
                left: a,
                right: b,
                output,
                precision,
            })
        })
    }

    pub(crate) fn finish(mut self, outputs: &[CudaValue]) -> Result<CudaProgramPlan, CudaError> {
        self.plan.outputs = outputs
            .iter()
            .map(|&value| self.value(value))
            .collect::<Result<_, _>>()?;
        Ok(self.plan)
    }
}

fn promote(layout: Layout, shape: Shape, left: bool) -> Result<Layout, CudaError> {
    if layout.shape().rank() != 1 {
        return Ok(layout);
    }
    let stride = layout.strides()[0];
    let strides = if left {
        vec![0, stride]
    } else {
        vec![stride, 0]
    };
    Ok(Layout::new(shape, strides, layout.offset())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(dims: &[usize]) -> Shape {
        Shape::new(dims.to_vec()).unwrap()
    }
    fn dense(dims: &[usize]) -> Layout {
        Layout::contiguous(shape(dims)).unwrap()
    }
    fn sizes(b: &CudaProgramPlanBuilder) -> (usize, usize, usize, usize) {
        (
            b.values.len(),
            b.plan.inputs.len(),
            b.plan.scratch.len(),
            b.plan.steps.len(),
        )
    }

    #[test]
    fn views_alias_and_strided_reshape_inserts_one_logical_copy() {
        let mut b = CudaProgramPlanBuilder::new();
        let x = b
            .input(Layout::new(shape(&[2, 3]), vec![1, 2], 3).unwrap())
            .unwrap();
        let transposed = b.permute(x, &[1, 0]).unwrap();
        let flat_alias = b.reshape(transposed, shape(&[6])).unwrap();
        let flat_copy = b.reshape(x, shape(&[6])).unwrap();
        let still_copy = b.materialize(flat_copy).unwrap();
        assert_eq!(flat_copy, still_copy);
        let p = b.finish(&[x, flat_alias, flat_copy, flat_copy]).unwrap();
        assert_eq!(p.inputs.len(), 1);
        assert_eq!(p.scratch, [shape(&[2, 3])]);
        assert_eq!(p.steps.len(), 1);
        let Step::Unary { source, op, .. } = &p.steps[0] else {
            panic!("reshape must materialize through the shared identity kernel")
        };
        assert_eq!(*op, 10);
        assert_eq!(
            (0..6)
                .map(|i| source.layout.element_offset(i).unwrap())
                .collect::<Vec<_>>(),
            [3, 5, 7, 4, 6, 8]
        );
        assert_eq!(p.outputs[0].buffer, BufferRef::Input(0));
        assert_eq!(p.outputs[1].buffer, BufferRef::Input(0));
        assert_eq!(p.outputs[1].layout.offset(), 3);
        assert_eq!(p.outputs[2].buffer, BufferRef::Scratch(0));
        assert_eq!(p.outputs[2], p.outputs[3]);
        assert_eq!(p.outputs[2].layout, dense(&[6]));
    }

    #[test]
    fn binary_broadcast_preserves_offsets_and_records_every_operation() {
        let mut b = CudaProgramPlanBuilder::new();
        let x = b
            .input(Layout::new(shape(&[1, 3]), vec![99, 2], 5).unwrap())
            .unwrap();
        let x = b.broadcast_to(x, shape(&[2, 3])).unwrap();
        let scalar = b
            .input(Layout::new(shape(&[]), vec![], 7).unwrap())
            .unwrap();
        let ops = [
            BinaryOp::Add,
            BinaryOp::Subtract,
            BinaryOp::Multiply,
            BinaryOp::Divide,
            BinaryOp::Min,
            BinaryOp::Max,
        ];
        for op in ops {
            b.binary(x, scalar, op).unwrap();
        }
        let p = b.finish(&[]).unwrap();
        assert_eq!(p.scratch, vec![shape(&[2, 3]); 6]);
        for (step, expected_op) in p.steps.iter().zip(ops) {
            let Step::Binary {
                left, right, op, ..
            } = step
            else {
                panic!()
            };
            assert_eq!(*op, expected_op);
            assert_eq!(left.layout.strides(), [0, 2]);
            assert_eq!(right.layout.strides(), [0, 0]);
            assert_eq!(left.layout.offset(), 5);
            assert_eq!(right.layout.offset(), 7);
        }
    }

    #[test]
    fn all_unary_operations_and_affine_keep_the_shared_kernel_abi() {
        let mut b = CudaProgramPlanBuilder::new();
        let x = b.input(dense(&[])).unwrap();
        for op in [
            UnaryOp::Negate,
            UnaryOp::Abs,
            UnaryOp::Square,
            UnaryOp::Sqrt,
            UnaryOp::Reciprocal,
            UnaryOp::Exp,
            UnaryOp::Log,
            UnaryOp::Sin,
            UnaryOp::Cos,
        ] {
            b.unary(x, op).unwrap();
        }
        let last = b.affine(x, -2.5, 3.25).unwrap();
        let p = b.finish(&[last]).unwrap();
        assert_eq!(p.steps.len(), 10);
        for (expected, step) in p.steps.iter().enumerate() {
            let Step::Unary {
                op, scale, bias, ..
            } = step
            else {
                panic!()
            };
            assert_eq!(*op, expected as u32);
            assert_eq!(
                (*scale, *bias),
                if expected == 9 {
                    (-2.5, 3.25)
                } else {
                    (1., 0.)
                }
            );
        }
        assert_eq!(p.outputs[0].layout, dense(&[]));
    }

    #[test]
    fn invalid_records_and_foreign_values_preserve_all_slots() {
        let mut other = CudaProgramPlanBuilder::new();
        let foreign = other.input(dense(&[2, 3])).unwrap();
        let mut b = CudaProgramPlanBuilder::new();
        let x = b.input(dense(&[2, 3])).unwrap();
        let before = sizes(&b);
        assert!(b.unary(foreign, UnaryOp::Square).is_err());
        assert!(b.binary(x, foreign, BinaryOp::Add).is_err());
        assert!(b.permute(x, &[0, 0]).is_err());
        assert!(b.reshape(x, shape(&[7])).is_err());
        assert!(b.broadcast_to(x, shape(&[3, 3])).is_err());
        assert!(b.reduce(x, ReduceOp::Sum, &[1, 1], false).is_err());
        let invalid = CudaValue {
            owner: b.owner,
            index: 99,
        };
        assert!(b.materialize(invalid).is_err());
        assert_eq!(sizes(&b), before);
        let y = b.input(dense(&[])).unwrap();
        assert_eq!(b.value(y).unwrap().buffer, BufferRef::Input(1));
        assert!(b.finish(&[foreign]).is_err());
    }

    #[test]
    fn compound_failure_rolls_back_new_inputs_values_scratch_and_steps() {
        let mut b = CudaProgramPlanBuilder::new();
        let x = b.input(dense(&[3])).unwrap();
        let before = sizes(&b);
        let failed: Result<(), _> = b.transaction(|b| {
            let y = b.input(dense(&[3]))?;
            let z = b.binary(x, y, BinaryOp::Multiply)?;
            b.unary(z, UnaryOp::Square)?;
            Err(CudaError::InvalidInput("injected late preparation failure"))
        });
        assert!(failed.is_err());
        assert_eq!(sizes(&b), before);
        let y = b.unary(x, UnaryOp::Negate).unwrap();
        let p = b.finish(&[y]).unwrap();
        assert_eq!(p.scratch, [shape(&[3])]);
        assert_eq!(p.outputs[0].buffer, BufferRef::Scratch(0));
    }

    #[test]
    fn broadcast_matmul_output_overflow_rolls_back_materialization() {
        let mut b = CudaProgramPlanBuilder::new();
        // Each operand fits the logical byte limit using one physical value;
        // their independent batch axes produce a representable element count
        // whose f32 byte count overflows. No large allocation is performed.
        let first = 1_usize << (usize::BITS / 2);
        let second = 1_usize << (usize::BITS - usize::BITS / 2 - 2);
        let a = b
            .input(Layout::new(shape(&[first, 1, 1, 1]), vec![0; 4], 0).unwrap())
            .unwrap();
        let c = b
            .input(Layout::new(shape(&[1, second, 1, 1]), vec![0; 4], 0).unwrap())
            .unwrap();
        let before = sizes(&b);
        assert!(b.matmul(a, c, MatmulPrecision::F32).is_err());
        assert_eq!(sizes(&b), before);
        assert!(b.finish(&[a, c]).unwrap().scratch.is_empty());
    }

    #[test]
    fn reduction_keeps_axis_order_broadcast_count_and_mean_policy() {
        let mut b = CudaProgramPlanBuilder::new();
        let x = b.input(dense(&[2, 1, 3])).unwrap();
        let x = b.broadcast_to(x, shape(&[2, 5, 3])).unwrap();
        let mean = b.mean(x, &[2, 0], true).unwrap();
        let sum = b.sum_axes(x, &[0, 1, 2], false).unwrap();
        let p = b.finish(&[mean, sum]).unwrap();
        assert_eq!(p.outputs[0].layout.shape(), &shape(&[1, 5, 1]));
        assert_eq!(p.outputs[1].layout.shape(), &shape(&[]));
        let Step::Reduce {
            source,
            axes,
            mean,
            op,
            keep_dims,
            ..
        } = &p.steps[0]
        else {
            panic!()
        };
        assert_eq!(axes, &[2, 0]);
        assert_eq!(source.layout.strides(), [3, 0, 1]);
        assert_eq!(source.layout.shape().numel() / p.scratch[0].numel(), 6);
        assert!(*mean && *keep_dims);
        assert_eq!(*op, ReduceOp::Sum);
    }

    #[test]
    fn empty_axes_alias_and_empty_contractions_remain_explicit_per_replay() {
        let mut b = CudaProgramPlanBuilder::new();
        let x = b.input(dense(&[2, 0, 3])).unwrap();
        assert_eq!(b.reduce(x, ReduceOp::Min, &[], false).unwrap(), x);
        assert_eq!(b.mean(x, &[], true).unwrap(), x);
        let sum = b.reduce(x, ReduceOp::Sum, &[1], false).unwrap();
        let product = b.reduce(x, ReduceOp::Product, &[1], false).unwrap();
        let before = sizes(&b);
        assert!(b.reduce(x, ReduceOp::Min, &[1], false).is_err());
        assert!(b.reduce(x, ReduceOp::Max, &[1], false).is_err());
        assert!(b.mean(x, &[1], false).is_err());
        assert_eq!(sizes(&b), before);
        let empty_mean = b.mean(x, &[0], false).unwrap();
        let p = b.finish(&[sum, product, empty_mean]).unwrap();
        assert_eq!(p.scratch, [shape(&[2, 3]), shape(&[2, 3]), shape(&[0, 3])]);
        assert!(matches!(
            p.steps[0],
            Step::Reduce {
                op: ReduceOp::Sum,
                ..
            }
        ));
        assert!(matches!(
            p.steps[1],
            Step::Reduce {
                op: ReduceOp::Product,
                ..
            }
        ));
        assert!(matches!(p.steps[2], Step::Reduce { mean: true, .. }));
    }

    #[test]
    fn matmul_materializes_strides_and_preserves_broadcast_vector_shape() {
        let mut b = CudaProgramPlanBuilder::new();
        let a = b
            .input(Layout::new(shape(&[2, 1, 3, 5]), vec![15, 0, 1, 3], 4).unwrap())
            .unwrap();
        let v = b
            .input(Layout::new(shape(&[5]), vec![2], 1).unwrap())
            .unwrap();
        let y = b.matmul(a, v, MatmulPrecision::AllowTf32).unwrap();
        let p = b.finish(&[y]).unwrap();
        assert_eq!(p.steps.len(), 3);
        assert!(matches!(p.steps[0], Step::Unary { op: 10, .. }));
        assert!(matches!(p.steps[1], Step::Unary { op: 10, .. }));
        let Step::Matmul {
            left,
            right,
            precision,
            ..
        } = &p.steps[2]
        else {
            panic!()
        };
        assert_eq!(*precision, MatmulPrecision::AllowTf32);
        assert_eq!(left.layout, dense(&[2, 1, 3, 5]));
        assert_eq!(right.layout.shape(), &shape(&[5, 1]));
        assert_eq!(right.layout.strides(), [1, 0]);
        assert_eq!(p.outputs[0].layout, dense(&[2, 1, 3]));
    }

    #[test]
    fn empty_matmul_retains_precision_and_skips_unused_materialization() {
        let mut b = CudaProgramPlanBuilder::new();
        let empty = b.input(dense(&[0])).unwrap();
        let dot = b.matmul(empty, empty, MatmulPrecision::AllowBf16).unwrap();
        let a = b.input(dense(&[0, 3, 5])).unwrap();
        let rhs = b
            .input(Layout::new(shape(&[5, 7]), vec![1, 5], 0).unwrap())
            .unwrap();
        let y = b.matmul(a, rhs, MatmulPrecision::F32).unwrap();
        let p = b.finish(&[dot, y]).unwrap();
        assert_eq!(p.steps.len(), 2);
        assert!(matches!(
            p.steps[0],
            Step::Matmul {
                precision: MatmulPrecision::AllowBf16,
                ..
            }
        ));
        assert!(matches!(
            p.steps[1],
            Step::Matmul {
                precision: MatmulPrecision::F32,
                ..
            }
        ));
        assert_eq!(p.outputs[0].layout.shape(), &shape(&[]));
        assert_eq!(p.outputs[1].layout.shape(), &shape(&[0, 3, 7]));
    }

    #[test]
    fn oversized_inputs_broadcasts_and_blas_dimensions_fail_before_recording() {
        let mut b = CudaProgramPlanBuilder::new();
        assert!(
            b.input(Layout::new(shape(&[usize::MAX / 4 + 1]), vec![0], 0).unwrap())
                .is_err()
        );
        assert_eq!(sizes(&b), (0, 0, 0, 0));
        let scalar = b.input(dense(&[])).unwrap();
        assert_eq!(b.value(scalar).unwrap().buffer, BufferRef::Input(0));
        assert!(
            b.broadcast_to(scalar, shape(&[usize::MAX / 4 + 1]))
                .is_err()
        );
        let huge = b
            .input(Layout::new(shape(&[i32::MAX as usize + 1, 1]), vec![0, 0], 0).unwrap())
            .unwrap();
        let one = b.input(dense(&[1, 1])).unwrap();
        let before = sizes(&b);
        assert!(b.matmul(huge, one, MatmulPrecision::F32).is_err());
        assert!(b.matmul(scalar, one, MatmulPrecision::F32).is_err());
        assert_eq!(sizes(&b), before);
        assert!(b.finish(&[]).unwrap().steps.is_empty());
    }
}
