/// Storage-buffer binding layout entry used by all GPU stages.
pub fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Blocking readback of a copy-destination buffer into a fresh Vec.
/// Returns an empty Vec on map failure, letting callers treat it as unavailability.
pub fn read_buffer(device: &wgpu::Device, buffer: &wgpu::Buffer, size: usize) -> Vec<u8> {
    crate::try_read_buffer(device, buffer, size).unwrap_or_default()
}

/// Packs values into little-endian bytes for GPU buffers (LE is the WGSL wire
/// order; on the little-endian target platforms this equals `to_ne_bytes`).
pub fn pack_f32(values: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// Same packing for u32 payloads (indices, aux records).
pub fn pack_u32(values: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}
