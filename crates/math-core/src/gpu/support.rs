pub(super) use crate::{M3, V3};
pub(super) use compute_core::gpu_compute::wgpu;
pub(super) use compute_core::gpu_compute::{GpuContext, pack_f32, pack_u32};
pub(super) use compute_core::{
    Binding, Kernel, try_read_f32 as read_f32, try_read_u32 as read_u32,
};
pub(super) use wgpu::{BindGroup, Buffer, BufferUsages, Device};

pub(super) const WG_METAL: u32 = 256;
pub(super) const WG_DEFAULT: u32 = 256;

pub(super) fn mk(device: &Device, label: &str, size: u64, usage: BufferUsages) -> Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(4),
        usage,
        mapped_at_creation: false,
    })
}

pub(super) fn push_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Uniform + read storage + write storage bindings, the common shape.
pub(super) const UNIFORM_STORAGE2: [Binding; 4] = [
    Binding::Uniform,
    Binding::StorageRead,
    Binding::StorageReadWrite,
    Binding::StorageReadWrite,
];

/// Uniform + two read-only inputs + two read-write outputs (nearest-neighbor).
pub(super) const NN_BINDINGS: [Binding; 5] = [
    Binding::Uniform,
    Binding::StorageRead,
    Binding::StorageRead,
    Binding::StorageReadWrite,
    Binding::StorageReadWrite,
];

/// Uniform + two read storages + one write storage (pairwise kernels).
pub(super) const UNIFORM_PAIR: [Binding; 4] = [
    Binding::Uniform,
    Binding::StorageRead,
    Binding::StorageRead,
    Binding::StorageReadWrite,
];

/// Uniform + read points + one partial-output reduction.
pub(super) const UNIFORM_REDUCE1: [Binding; 3] = [
    Binding::Uniform,
    Binding::StorageRead,
    Binding::StorageReadWrite,
];
