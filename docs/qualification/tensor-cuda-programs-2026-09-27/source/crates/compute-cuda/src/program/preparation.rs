use super::{
    CudaPrepareOptions, CudaProgramStats,
    execution::{CudaPreparedProgram, Instruction},
    plan::{BufferRef, CudaProgramPlan, PlannedValue, Step, allocation_bytes},
};
use crate::{
    CudaError, CudaRuntime,
    dispatch::{BinaryDispatch, UnaryDispatch},
    matmul::GemmPlan,
    reduction::dispatch::ReductionPass,
};
use tensor_core::{Layout, MatmulPrecision, ReduceOp, Shape};

#[derive(Clone, Copy, Debug)]
pub(super) enum Destination {
    Scratch(usize),
    Output(usize),
}
#[derive(Debug)]
pub(super) enum Operation {
    Unary {
        source: BufferRef,
        pass: UnaryDispatch,
    },
    Binary {
        left: BufferRef,
        right: BufferRef,
        pass: BinaryDispatch,
    },
    Reduction {
        source: BufferRef,
        pass: ReductionPass,
    },
    Gemm {
        left: BufferRef,
        right: BufferRef,
        plan: GemmPlan,
        precision: MatmulPrecision,
    },
    Fill {
        count: usize,
        value: f32,
    },
}
impl Operation {
    pub fn metadata(&self) -> Option<&[u64]> {
        match self {
            Self::Unary { pass, .. } => Some(&pass.metadata),
            Self::Binary { pass, .. } => Some(&pass.metadata),
            Self::Reduction { pass, .. } => Some(&pass.metadata),
            Self::Gemm { .. } | Self::Fill { .. } => None,
        }
    }
}
#[derive(Debug)]
pub(super) struct Scheduled {
    pub destination: Destination,
    pub operation: Operation,
}
pub(super) struct Expanded {
    pub inputs: Vec<Layout>,
    pub outputs: Vec<Shape>,
    pub precisions: Vec<MatmulPrecision>,
    pub scratch: Vec<Shape>,
    pub schedule: Vec<Scheduled>,
    pub stats: CudaProgramStats,
}
impl Expanded {
    fn scratch(&mut self, shape: Shape) -> Result<usize, CudaError> {
        allocation_bytes(&shape)?;
        let index = self.scratch.len();
        self.scratch.push(shape);
        Ok(index)
    }
    fn add(&mut self, output: usize, operation: Operation) {
        self.schedule.push(Scheduled {
            destination: Destination::Scratch(output),
            operation,
        });
    }
    fn copy(&mut self, source: PlannedValue, destination: Destination) -> Result<(), CudaError> {
        if source.layout.shape().is_empty() {
            return Ok(());
        }
        self.schedule.push(Scheduled {
            destination,
            operation: Operation::Unary {
                source: source.buffer,
                pass: UnaryDispatch::new(&source.layout, 10, 1., 0.)?,
            },
        });
        Ok(())
    }
    fn reduction(
        &mut self,
        mut source: PlannedValue,
        output: usize,
        op: ReduceOp,
        axes: &[usize],
        mean: bool,
        multiprocessors: u32,
    ) -> Result<(), CudaError> {
        let shape = self.scratch[output].clone();
        if shape.is_empty() {
            return Ok(());
        }
        if axes.is_empty() {
            return self.copy(source, Destination::Scratch(output));
        }
        if source.layout.shape().is_empty() {
            self.add(
                output,
                Operation::Fill {
                    count: shape.numel(),
                    value: if op == ReduceOp::Product { 1. } else { 0. },
                },
            );
            return Ok(());
        }
        let divisor = (source.layout.shape().numel() / shape.numel()) as f32;
        let reduction_output = if mean {
            self.scratch(shape.clone())?
        } else {
            output
        };
        if axes.len() == source.layout.shape().rank() {
            loop {
                let pass = ReductionPass::all(op, &source.layout, multiprocessors)?;
                let last = pass.output_shape.numel() == 1;
                let next = if last {
                    reduction_output
                } else {
                    self.scratch(pass.output_shape.clone())?
                };
                let next_layout = Layout::contiguous(pass.output_shape.clone())?;
                self.add(
                    next,
                    Operation::Reduction {
                        source: source.buffer,
                        pass,
                    },
                );
                if last {
                    break;
                }
                source = PlannedValue {
                    buffer: BufferRef::Scratch(next),
                    layout: next_layout,
                };
            }
        } else {
            let pass =
                ReductionPass::axes(op, &source.layout, axes, shape.clone(), multiprocessors)?;
            self.add(
                reduction_output,
                Operation::Reduction {
                    source: source.buffer,
                    pass,
                },
            );
        }
        if mean {
            self.add(
                output,
                Operation::Unary {
                    source: BufferRef::Scratch(reduction_output),
                    pass: UnaryDispatch::new(&Layout::contiguous(shape)?, 11, divisor, 0.)?,
                },
            );
        }
        Ok(())
    }
    fn account(&mut self, options: CudaPrepareOptions) -> Result<(), CudaError> {
        let add = |a: usize, b: usize| {
            a.checked_add(b).ok_or(CudaError::InvalidInput(
                "CUDA preparation byte/count overflow",
            ))
        };
        for shape in &self.scratch {
            self.stats.scratch_bytes = add(self.stats.scratch_bytes, allocation_bytes(shape)?)?;
        }
        for step in &self.schedule {
            if let Some(metadata) = step.operation.metadata() {
                let bytes = metadata
                    .len()
                    .max(1)
                    .checked_mul(8)
                    .ok_or(CudaError::InvalidInput("CUDA metadata byte count overflow"))?;
                self.stats.metadata_bytes = add(self.stats.metadata_bytes, bytes)?;
            }
            match &step.operation {
                Operation::Gemm { plan, .. } => {
                    self.stats.gemm_calls = add(self.stats.gemm_calls, plan.calls())?
                }
                _ => self.stats.kernel_launches = add(self.stats.kernel_launches, 1)?,
            }
        }
        for (kind, required, limit) in [
            (
                "scratch",
                self.stats.scratch_bytes,
                options.max_scratch_bytes,
            ),
            (
                "metadata",
                self.stats.metadata_bytes,
                options.max_metadata_bytes,
            ),
        ] {
            if required > limit {
                return Err(CudaError::PreparationBudget {
                    kind,
                    required,
                    limit,
                });
            }
        }
        Ok(())
    }
}
pub(super) fn expand(
    plan: CudaProgramPlan,
    multiprocessors: u32,
    options: CudaPrepareOptions,
    mut policy: impl FnMut(MatmulPrecision) -> Result<(), CudaError>,
) -> Result<Expanded, CudaError> {
    let mut expanded = Expanded {
        inputs: plan.inputs,
        precisions: Vec::new(),
        outputs: plan
            .outputs
            .iter()
            .map(|v| v.layout.shape().clone())
            .collect(),
        scratch: plan.scratch,
        schedule: Vec::new(),
        stats: CudaProgramStats::default(),
    };
    for step in plan.steps {
        match step {
            Step::Unary {
                source,
                output,
                op,
                scale,
                bias,
            } => {
                if !source.layout.shape().is_empty() {
                    expanded.add(
                        output,
                        Operation::Unary {
                            source: source.buffer,
                            pass: UnaryDispatch::new(&source.layout, op, scale, bias)?,
                        },
                    );
                }
            }
            Step::Binary {
                left,
                right,
                output,
                op,
            } => {
                let pass = BinaryDispatch::new(&left.layout, &right.layout, op)?;
                if !pass.shape.is_empty() {
                    expanded.add(
                        output,
                        Operation::Binary {
                            left: left.buffer,
                            right: right.buffer,
                            pass,
                        },
                    );
                }
            }
            Step::Reduce {
                source,
                output,
                op,
                axes,
                keep_dims,
                mean,
            } => {
                let shape = if mean {
                    tensor_core::mean_shape(source.layout.shape(), &axes, keep_dims)?
                } else {
                    tensor_core::reduction_shape(op, source.layout.shape(), &axes, keep_dims)?
                };
                if shape != expanded.scratch[output] {
                    return Err(CudaError::InvalidInput("invalid internal reduction output"));
                }
                expanded.reduction(source, output, op, &axes, mean, multiprocessors)?;
            }
            Step::Matmul {
                left,
                right,
                output,
                precision,
            } => {
                policy(precision)?; // Includes empty and zero-contraction plans.
                expanded.precisions.push(precision);
                let gemm = GemmPlan::new(&left.layout, &right.layout)?;
                if !expanded.scratch[output].is_empty() {
                    if gemm.is_zero() {
                        expanded.add(
                            output,
                            Operation::Fill {
                                count: expanded.scratch[output].numel(),
                                value: 0.,
                            },
                        );
                    } else {
                        expanded.add(
                            output,
                            Operation::Gemm {
                                left: left.buffer,
                                right: right.buffer,
                                plan: gemm,
                                precision,
                            },
                        );
                    }
                }
            }
        }
    }
    for (i, value) in plan.outputs.into_iter().enumerate() {
        expanded.copy(value, Destination::Output(i))?;
    }
    expanded.account(options)?;
    Ok(expanded)
}

pub(super) fn prepare(
    runtime: &CudaRuntime,
    plan: CudaProgramPlan,
    options: CudaPrepareOptions,
) -> Result<CudaPreparedProgram<'_>, CudaError> {
    let expanded = expand(plan, runtime.capabilities.multiprocessors, options, |p| {
        runtime.matmul_policy(p).map(|_| ())
    })?;
    // All layout/precision/budget checks above are host-only. Any cuBLAS setup,
    // tensor allocation or metadata upload happens only after they succeed.
    let mut instructions = Vec::with_capacity(expanded.schedule.len());
    for step in expanded.schedule {
        let gemm = match &step.operation {
            Operation::Gemm {
                plan, precision, ..
            } => Some(runtime.prepare_gemm_f32(plan.clone(), *precision)?),
            _ => None,
        };
        let metadata = step
            .operation
            .metadata()
            .map(|words| runtime.metadata(words))
            .transpose()?;
        instructions.push(Instruction {
            destination: step.destination,
            operation: step.operation,
            metadata,
            gemm,
        });
    }
    let scratch = expanded
        .scratch
        .into_iter()
        .map(|shape| {
            runtime
                .device
                .stream
                .alloc_zeros::<f32>(shape.numel().max(1))
                .map(Some)
                .map_err(CudaError::from)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CudaPreparedProgram {
        runtime,
        inputs: expanded.inputs,
        outputs: expanded.outputs,
        precisions: expanded.precisions,
        instructions,
        scratch,
        stats: expanded.stats,
        state: Default::default(),
    })
}
