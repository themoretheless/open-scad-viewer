use super::{
    CudaProgramStats,
    gate::ExecutionState,
    plan::BufferRef,
    preparation::{Destination, Operation},
    validation::{self, Binding},
};
use crate::{
    CudaError, CudaRuntime, CudaTensor, dispatch, matmul::PreparedGemm,
    reduction::dispatch::LoadedReduction,
};
use gpu_compute::cuda::CudaSlice;
use std::sync::Arc;
use tensor_core::{Layout, MatmulPrecision, Shape};

pub(super) struct Instruction {
    pub destination: Destination,
    pub operation: Operation,
    pub metadata: Option<CudaSlice<u64>>,
    pub gemm: Option<PreparedGemm>,
}
/// Prepared f32 launch sequence with fixed input layouts. Scratch is owned and
/// replay requires exclusive access. No caller input/output clones are retained.
/// A failed enqueue or explicit synchronize poisons this program permanently;
/// create a new prepared program after recovering the runtime.
pub struct CudaPreparedProgram<'rt> {
    pub(super) runtime: &'rt CudaRuntime,
    pub(super) inputs: Vec<Layout>,
    pub(super) outputs: Vec<Shape>,
    pub(super) precisions: Vec<MatmulPrecision>,
    pub(super) instructions: Vec<Instruction>,
    pub(super) scratch: Vec<Option<CudaSlice<f32>>>,
    pub(super) stats: CudaProgramStats,
    pub(super) state: ExecutionState,
}
fn binding(tensor: &CudaTensor) -> Binding<'_> {
    Binding {
        layout: &tensor.layout,
        storage_len: tensor.storage.len(),
        allocation: Arc::as_ptr(&tensor.storage) as usize,
        unique: Arc::strong_count(&tensor.storage) == 1 && Arc::weak_count(&tensor.storage) == 0,
    }
}
impl CudaPreparedProgram<'_> {
    pub fn stats(&self) -> CudaProgramStats {
        self.stats
    }
    pub fn is_poisoned(&self) -> bool {
        self.state.poisoned
    }
    pub fn input_layouts(&self) -> &[Layout] {
        &self.inputs
    }
    pub fn output_shapes(&self) -> &[Shape] {
        &self.outputs
    }
    fn validate_inputs(&self, inputs: &[&CudaTensor]) -> Result<(), CudaError> {
        self.state.check()?;
        validate_inputs(self.runtime, &self.inputs, &self.precisions, inputs)
    }

    /// Allocate fresh independent outputs, then submit the prepared schedule.
    /// Invalid inputs and poison are rejected before any output allocation.
    pub fn run(&mut self, inputs: &[&CudaTensor]) -> Result<Vec<CudaTensor>, CudaError> {
        self.validate_inputs(inputs)?;
        let mut outputs = self
            .outputs
            .iter()
            .cloned()
            .map(|s| self.runtime.zeros(s))
            .collect::<Result<Vec<_>, _>>()?;
        self.run_into(inputs, &mut outputs.iter_mut().collect::<Vec<_>>())?;
        Ok(outputs)
    }
    /// Rebind existing tensors without device tensor allocation, metadata upload
    /// or NVRTC compilation. All caller bindings are checked before the first
    /// enqueue; ordinary kernel and cuBLAS submission still occurs each run.
    pub fn run_into(
        &mut self,
        inputs: &[&CudaTensor],
        outputs: &mut [&mut CudaTensor],
    ) -> Result<(), CudaError> {
        let runtime = self.runtime;
        let permit = self.state.begin(|| {
            validate_inputs(runtime, &self.inputs, &self.precisions, inputs)?;
            for output in outputs.iter() {
                runtime.check(output)?;
            }
            validation::outputs(
                &self.outputs,
                &inputs.iter().map(|t| binding(t)).collect::<Vec<_>>(),
                &outputs.iter().map(|t| binding(t)).collect::<Vec<_>>(),
            )
        })?;
        permit.execute(|| {
            execute(
                runtime,
                &self.instructions,
                &mut self.scratch,
                inputs,
                outputs,
            )
        })
    }
    /// Observe deferred stream errors and poison this program on failure.
    pub fn synchronize(&mut self) -> Result<(), CudaError> {
        let runtime = self.runtime;
        self.state
            .begin(|| Ok(()))?
            .execute(|| runtime.synchronize())
    }
}
fn validate_inputs(
    runtime: &CudaRuntime,
    expected: &[Layout],
    precisions: &[MatmulPrecision],
    inputs: &[&CudaTensor],
) -> Result<(), CudaError> {
    for input in inputs {
        runtime.check(input)?;
    }
    validation::inputs(
        expected,
        &inputs.iter().map(|t| binding(t)).collect::<Vec<_>>(),
    )?;
    validation::precisions(precisions, |precision| {
        runtime.matmul_policy(precision).map(|_| ())
    })
}
fn execute(
    runtime: &CudaRuntime,
    instructions: &[Instruction],
    scratch: &mut [Option<CudaSlice<f32>>],
    inputs: &[&CudaTensor],
    outputs: &mut [&mut CudaTensor],
) -> Result<(), CudaError> {
    for instruction in instructions {
        match instruction.destination {
            Destination::Scratch(index) => {
                // Temporarily remove the unique destination for disjoint source
                // borrows without storage clones or bypassing cudarc events.
                let mut output = scratch[index].take().expect("prepared scratch exists");
                let result = instruction.launch(runtime, inputs, scratch, &mut output);
                scratch[index] = Some(output);
                result?;
            }
            Destination::Output(index) => {
                let output =
                    Arc::get_mut(&mut outputs[index].storage).ok_or(CudaError::SharedOutput)?;
                instruction.launch(runtime, inputs, scratch, output)?;
            }
        }
    }
    Ok(())
}

fn source<'a>(
    buffer: BufferRef,
    inputs: &'a [&CudaTensor],
    scratch: &'a [Option<CudaSlice<f32>>],
) -> Result<&'a CudaSlice<f32>, CudaError> {
    match buffer {
        BufferRef::Input(i) => inputs.get(i).map(|t| t.storage.as_ref()),
        BufferRef::Scratch(i) => scratch.get(i).and_then(Option::as_ref),
    }
    .ok_or(CudaError::InvalidInput("invalid internal prepared source"))
}
impl Instruction {
    fn launch(
        &self,
        rt: &CudaRuntime,
        inputs: &[&CudaTensor],
        scratch: &[Option<CudaSlice<f32>>],
        output: &mut CudaSlice<f32>,
    ) -> Result<(), CudaError> {
        let metadata = || self.metadata.as_ref().expect("prepared metadata exists");
        match &self.operation {
            Operation::Unary {
                source: input,
                pass,
            } => pass.launch(rt, source(*input, inputs, scratch)?, output, metadata()),
            Operation::Binary { left, right, pass } => pass.launch(
                rt,
                source(*left, inputs, scratch)?,
                source(*right, inputs, scratch)?,
                output,
                metadata(),
            ),
            Operation::Reduction {
                source: input,
                pass,
            } => pass.launch(
                rt,
                LoadedReduction {
                    function: if pass.is_all() {
                        &rt.reduction.all[0]
                    } else {
                        &rt.reduction.axes[0]
                    },
                    dtype: None,
                },
                source(*input, inputs, scratch)?,
                output,
                metadata(),
            ),
            Operation::Gemm { left, right, .. } => rt.launch_gemm_f32(
                self.gemm.as_ref().expect("prepared GEMM exists"),
                source(*left, inputs, scratch)?,
                source(*right, inputs, scratch)?,
                output,
            ),
            Operation::Fill { count, value } => dispatch::fill(rt, output, *count, *value),
        }
    }
}
