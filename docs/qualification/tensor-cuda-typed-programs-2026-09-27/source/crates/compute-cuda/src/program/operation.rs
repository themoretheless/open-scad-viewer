use super::plan::{BufferRef, CudaDtype};
use crate::{
    CudaError, CudaRuntime,
    dispatch::{BinaryDispatch, UnaryDispatch},
    indexing::dispatch::{CompareDispatch, CopyDispatch, SelectDispatch},
    low_dispatch::{LowBinaryDispatch, LowCastDispatch, LowUnaryDispatch},
    matmul::{GemmPlan, PreparedGemm},
    reduction::dispatch::ReductionPass,
};
use tensor_core::{LowDtype, MatmulPrecision};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MatmulRequest {
    F32(MatmulPrecision),
    Low(LowDtype),
}
impl MatmulRequest {
    pub fn validate(self, rt: &CudaRuntime) -> Result<(), CudaError> {
        match self {
            Self::F32(p) => rt.matmul_policy(p).map(|_| ()),
            Self::Low(d) => rt.validate_low_matmul(d),
        }
    }
    pub fn prepare(self, rt: &CudaRuntime, plan: GemmPlan) -> Result<PreparedGemm, CudaError> {
        match self {
            Self::F32(p) => rt.prepare_gemm_f32(plan, p),
            Self::Low(d) => rt.prepare_gemm_low(plan, d),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Destination {
    Scratch(usize),
    Output(usize),
}
#[derive(Debug)]
pub(super) enum Operation {
    Copy {
        source: BufferRef,
        pass: CopyDispatch,
    },
    Unary {
        source: BufferRef,
        pass: UnaryDispatch,
    },
    LowUnary {
        source: BufferRef,
        pass: LowUnaryDispatch,
    },
    Binary {
        left: BufferRef,
        right: BufferRef,
        pass: BinaryDispatch,
    },
    LowBinary {
        left: BufferRef,
        right: BufferRef,
        pass: LowBinaryDispatch,
    },
    Compare {
        left: BufferRef,
        right: BufferRef,
        pass: CompareDispatch,
    },
    Select {
        mask: BufferRef,
        yes: BufferRef,
        no: BufferRef,
        pass: SelectDispatch,
    },
    Cast {
        source: BufferRef,
        pass: LowCastDispatch,
        to: CudaDtype,
    },
    Reduction {
        source: BufferRef,
        pass: ReductionPass,
    },
    Gemm {
        left: BufferRef,
        right: BufferRef,
        plan: GemmPlan,
        request: MatmulRequest,
    },
    Fill {
        count: usize,
        value: u32,
    },
}
impl Operation {
    pub fn metadata(&self) -> Option<&[u64]> {
        match self {
            Self::Copy { pass, .. } => Some(&pass.metadata),
            Self::Unary { pass, .. } => Some(&pass.metadata),
            Self::LowUnary { pass, .. } => Some(&pass.geometry.metadata),
            Self::Binary { pass, .. } => Some(&pass.metadata),
            Self::LowBinary { pass, .. } => Some(&pass.geometry.metadata),
            Self::Compare { pass, .. } => Some(&pass.metadata),
            Self::Select { pass, .. } => Some(&pass.metadata),
            Self::Cast { pass, .. } => Some(&pass.geometry.metadata),
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
