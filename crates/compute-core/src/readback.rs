use crate::{ComputeError, GpuArray, GpuElement, wgpu};
use gpu_compute::ByteReadback;
use std::{marker::PhantomData, time::Duration};

/// Typed decoding over the shared GPU byte transport. A result is consumed once.
/// Dropping/cancelling a ticket releases its mapping without waiting for the GPU.
pub struct Readback<T: GpuElement> {
    bytes: ByteReadback,
    marker: PhantomData<T>,
}
impl<T: GpuElement> Readback<T> {
    pub(crate) fn record(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        array: &GpuArray<T>,
    ) -> Result<Self, ComputeError> {
        Ok(Self {
            bytes: ByteReadback::copy_buffer(
                device,
                encoder,
                array.buffer(),
                0,
                array.len() as u64 * 4,
            )?,
            marker: PhantomData,
        })
    }
    pub fn submitted(&mut self, index: wgpu::SubmissionIndex) {
        self.bytes.submitted(index);
    }
    pub fn cancel(&mut self) {
        self.bytes.cancel();
    }
    pub fn try_read(&mut self) -> Result<Option<Vec<T>>, ComputeError> {
        Ok(self.bytes.try_read()?.map(decode::<T>))
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wait(mut self, timeout: Duration) -> Result<Vec<T>, ComputeError> {
        Ok(decode(self.bytes.wait(timeout)?))
    }
}
fn decode<T: GpuElement>(bytes: Vec<u8>) -> Vec<T> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| T::from_bytes(*bytes))
        .collect()
}
