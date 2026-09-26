use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferError {
    InvalidRange,
    InvalidUsage,
    MissingUsage,
    ForeignDevice,
    TooLarge,
    UnalignedBinding,
}
impl std::fmt::Display for BufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GPU buffer: {self:?}")
    }
}
impl std::error::Error for BufferError {}

/// Storage created on a known device. There is intentionally no unchecked
/// constructor from a raw buffer and a caller-asserted device identity.
#[derive(Clone)]
pub struct GpuBuffer {
    context: crate::GpuContext,
    buffer: wgpu::Buffer,
}
impl GpuBuffer {
    pub fn new(
        context: &crate::GpuContext,
        bytes: u64,
        usage: wgpu::BufferUsages,
    ) -> Result<Self, BufferError> {
        let device = &context.device;
        if usage.is_empty()
            || (!context
                .enabled_features()
                .contains(wgpu::Features::MAPPABLE_PRIMARY_BUFFERS)
                && ((usage.contains(wgpu::BufferUsages::MAP_READ)
                    && !(usage - wgpu::BufferUsages::MAP_READ - wgpu::BufferUsages::COPY_DST)
                        .is_empty())
                    || (usage.contains(wgpu::BufferUsages::MAP_WRITE)
                        && !(usage
                            - wgpu::BufferUsages::MAP_WRITE
                            - wgpu::BufferUsages::COPY_SRC)
                            .is_empty())))
        {
            return Err(BufferError::InvalidUsage);
        }
        let bytes = bytes.max(4);
        let limits = device.limits();
        if bytes > limits.max_buffer_size
            || (usage.contains(wgpu::BufferUsages::STORAGE)
                && bytes > limits.max_storage_buffer_binding_size)
        {
            return Err(BufferError::TooLarge);
        }
        if !bytes.is_multiple_of(4) {
            return Err(BufferError::InvalidRange);
        }
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("owned GPU buffer"),
            size: bytes,
            usage,
            mapped_at_creation: false,
        });
        Ok(Self {
            context: context.clone(),
            buffer,
        })
    }
    pub fn device(&self) -> &wgpu::Device {
        &self.context.device
    }
    pub fn raw(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn size(&self) -> u64 {
        self.buffer.size()
    }
    pub fn view(&self, range: Range<u64>) -> Result<GpuBufferView<'_>, BufferError> {
        if range.start > range.end
            || range.end > self.size()
            || !range.start.is_multiple_of(4)
            || !range.end.is_multiple_of(4)
        {
            return Err(BufferError::InvalidRange);
        }
        Ok(GpuBufferView {
            buffer: self,
            offset: range.start,
            bytes: range.end - range.start,
        })
    }
}

#[derive(Clone, Copy)]
pub struct GpuBufferView<'a> {
    buffer: &'a GpuBuffer,
    offset: u64,
    bytes: u64,
}
impl<'a> GpuBufferView<'a> {
    pub fn validate(
        &self,
        context: &crate::GpuContext,
        usage: wgpu::BufferUsages,
    ) -> Result<(), BufferError> {
        if !self.buffer.context.same_device(context) {
            return Err(BufferError::ForeignDevice);
        }
        if !self.buffer.raw().usage().contains(usage) {
            return Err(BufferError::MissingUsage);
        }
        Ok(())
    }
    pub fn len_bytes(&self) -> u64 {
        self.bytes
    }
    pub fn is_empty(&self) -> bool {
        self.bytes == 0
    }
    pub fn offset(&self) -> u64 {
        self.offset
    }
    pub fn raw(&self) -> &'a wgpu::Buffer {
        self.buffer.raw()
    }
    pub fn storage_binding(
        &self,
        context: &crate::GpuContext,
    ) -> Result<wgpu::BindingResource<'a>, BufferError> {
        self.validate(context, wgpu::BufferUsages::STORAGE)?;
        if !self.offset.is_multiple_of(u64::from(
            context.device.limits().min_storage_buffer_offset_alignment,
        )) {
            return Err(BufferError::UnalignedBinding);
        }
        let size = std::num::NonZeroU64::new(self.bytes).ok_or(BufferError::InvalidRange)?;
        Ok(wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: self.raw(),
            offset: self.offset,
            size: Some(size),
        }))
    }
}
