use super::plan::{CudaDtype, TensorSpec};
use crate::{CudaError, CudaRuntime};
use gpu_compute::cuda::CudaSlice;
use tensor_core::LowDtype;

pub(super) enum Storage {
    F32(CudaSlice<f32>),
    U32(CudaSlice<u32>),
    Low(CudaSlice<u16>, LowDtype),
}
#[derive(Clone, Copy)]
pub(super) enum StorageRef<'a> {
    F32(&'a CudaSlice<f32>),
    U32(&'a CudaSlice<u32>),
    Low(&'a CudaSlice<u16>, LowDtype),
}
pub(super) enum StorageMut<'a> {
    F32(&'a mut CudaSlice<f32>),
    U32(&'a mut CudaSlice<u32>),
    Low(&'a mut CudaSlice<u16>, LowDtype),
}
impl Storage {
    pub fn zeros(rt: &CudaRuntime, spec: &TensorSpec) -> Result<Self, CudaError> {
        let count = spec.shape().numel().max(1);
        Ok(match spec.dtype {
            CudaDtype::F32 => Self::F32(rt.device.stream.alloc_zeros(count)?),
            CudaDtype::U32 => Self::U32(rt.device.stream.alloc_zeros(count)?),
            CudaDtype::F16 | CudaDtype::Bf16 => Self::Low(
                rt.device.stream.alloc_zeros(count)?,
                spec.dtype.low_dtype().unwrap(),
            ),
        })
    }
    pub fn as_ref(&self) -> StorageRef<'_> {
        match self {
            Self::F32(s) => StorageRef::F32(s),
            Self::U32(s) => StorageRef::U32(s),
            Self::Low(s, d) => StorageRef::Low(s, *d),
        }
    }
    pub fn as_mut(&mut self) -> StorageMut<'_> {
        match self {
            Self::F32(s) => StorageMut::F32(s),
            Self::U32(s) => StorageMut::U32(s),
            Self::Low(s, d) => StorageMut::Low(s, *d),
        }
    }
}
impl<'a> StorageRef<'a> {
    pub fn f32(self) -> Result<&'a CudaSlice<f32>, CudaError> {
        if let Self::F32(s) = self {
            Ok(s)
        } else {
            Err(CudaError::Dtype)
        }
    }
    pub fn u32(self) -> Result<&'a CudaSlice<u32>, CudaError> {
        if let Self::U32(s) = self {
            Ok(s)
        } else {
            Err(CudaError::Dtype)
        }
    }
}
impl<'a> StorageMut<'a> {
    pub fn f32(self) -> Result<&'a mut CudaSlice<f32>, CudaError> {
        if let Self::F32(s) = self {
            Ok(s)
        } else {
            Err(CudaError::Dtype)
        }
    }
    pub fn u32(self) -> Result<&'a mut CudaSlice<u32>, CudaError> {
        if let Self::U32(s) = self {
            Ok(s)
        } else {
            Err(CudaError::Dtype)
        }
    }
}
