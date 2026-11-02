#![allow(unsafe_code, unsafe_op_in_unsafe_fn, unused_variables, dead_code)]

pub mod cuda;
pub mod dx12;
pub mod metal;
pub mod util;
pub mod vulkan;
pub mod webgpu;

use metal::{
    metal_msg_send, metal_selectors, nsstring_from_str, MTLClearColor, MTLOrigin, MTLRegion,
    MTLScissorRect, MTLSize, MTLViewport, MetalDevice, NSRange, ObjCObject,
};
use naga::back::msl::{
    msl_buffer_slot, msl_sampler_slot, msl_texture_slot, BindingMap, MSL_ARRAY_LENGTHS_BUFFER_SLOT,
    MSL_IMMEDIATE_BUFFER_SLOT, MSL_VERTEX_BUFFER_BASE_SLOT,
};
use std::borrow::Cow;
use std::ffi::c_void;
use std::future::Future;
use std::num::{NonZeroU32, NonZeroU64};
use std::ops::{Bound, Deref, DerefMut, Range, RangeBounds};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub type BufferAddress = u64;
pub type BufferSize = NonZeroU64;
pub type DynamicOffset = u32;
pub type Label<'a> = Option<&'a str>;

macro_rules! impl_bitflags {
    ($name:ident, $ty:ty) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(pub $ty);

        impl $name {
            #[inline]
            pub const fn empty() -> Self {
                Self(0)
            }
            #[inline]
            pub const fn all() -> Self {
                Self(!0)
            }
            #[inline]
            pub const fn bits(self) -> $ty {
                self.0
            }
            #[inline]
            pub const fn from_bits_truncate(bits: $ty) -> Self {
                Self(bits)
            }
            #[inline]
            pub const fn is_empty(self) -> bool {
                self.0 == 0
            }
            #[inline]
            pub const fn contains(self, other: Self) -> bool {
                (self.0 & other.0) == other.0
            }
            #[inline]
            pub const fn intersects(self, other: Self) -> bool {
                (self.0 & other.0) != 0
            }
            #[inline]
            pub const fn union(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}(0x{:x})", stringify!($name), self.0)
            }
        }

        impl std::ops::BitOr for $name {
            type Output = Self;
            #[inline]
            fn bitor(self, rhs: Self) -> Self {
                Self(self.0 | rhs.0)
            }
        }

        impl std::ops::BitOrAssign for $name {
            #[inline]
            fn bitor_assign(&mut self, rhs: Self) {
                self.0 |= rhs.0;
            }
        }

        impl std::ops::BitAnd for $name {
            type Output = Self;
            #[inline]
            fn bitand(self, rhs: Self) -> Self {
                Self(self.0 & rhs.0)
            }
        }

        impl std::ops::BitAndAssign for $name {
            #[inline]
            fn bitand_assign(&mut self, rhs: Self) {
                self.0 &= rhs.0;
            }
        }

        impl std::ops::Sub for $name {
            type Output = Self;
            #[inline]
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 & !rhs.0)
            }
        }

        impl std::ops::Not for $name {
            type Output = Self;
            #[inline]
            fn not(self) -> Self {
                Self(!self.0)
            }
        }
    };
}

impl_bitflags!(Backends, u32);
impl Backends {
    pub const VULKAN: Self = Self(1 << 0);
    pub const GL: Self = Self(1 << 1);
    pub const METAL: Self = Self(1 << 2);
    pub const DX12: Self = Self(1 << 3);
    pub const BROWSER_WEBGPU: Self = Self(1 << 4);
    pub const CUDA: Self = Self(1 << 5);
    pub const PRIMARY: Self = Self(
        Self::VULKAN.0
            | Self::METAL.0
            | Self::DX12.0
            | Self::BROWSER_WEBGPU.0
            | Self::CUDA.0,
    );

    pub fn from_env() -> Option<Self> {
        let val = std::env::var("WGPU_BACKEND").ok()?;
        let mut out = Self::empty();
        for part in val.split(',') {
            match part.trim().to_ascii_lowercase().as_str() {
                "vulkan" | "vk" => out |= Self::VULKAN,
                "metal" | "mtl" => out |= Self::METAL,
                "dx12" | "d3d12" => out |= Self::DX12,
                "gl" | "opengl" | "gles" => out |= Self::GL,
                "webgpu" | "browser" => out |= Self::BROWSER_WEBGPU,
                "cuda" | "ptx" => out |= Self::CUDA,
                "primary" => out |= Self::PRIMARY,
                "all" => out |= Self::all(),
                _ => {}
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }
}

impl_bitflags!(InstanceFlags, u32);
impl InstanceFlags {
    pub const DEBUG: Self = Self(1 << 0);
    pub const VALIDATION: Self = Self(1 << 1);
    pub const DISCARD_HAL_LABELS: Self = Self(1 << 2);
}

impl_bitflags!(Features, u64);
impl Features {
    pub const TIMESTAMP_QUERY: Self = Self(1 << 0);
    pub const TIMESTAMP_QUERY_INSIDE_ENCODERS: Self = Self(1 << 1);
    pub const TIMESTAMP_QUERY_INSIDE_PASSES: Self = Self(1 << 2);
    pub const MAPPABLE_PRIMARY_BUFFERS: Self = Self(1 << 3);
    pub const TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES: Self = Self(1 << 4);
    pub const MULTI_DRAW_INDIRECT_COUNT: Self = Self(1 << 5);
    pub const SUBGROUP: Self = Self(1 << 6);
    pub const SHADER_F16: Self = Self(1 << 7);
    pub const FLOAT32_FILTERABLE: Self = Self(1 << 8);
    pub const IMMEDIATE_DATA: Self = Self(1 << 9);
    pub const PUSH_CONSTANTS: Self = Self::IMMEDIATE_DATA;
    pub const EXPERIMENTAL_RAY_QUERY: Self = Self(1 << 10);
}

impl_bitflags!(BufferUsages, u32);
impl BufferUsages {
    pub const MAP_READ: Self = Self(1 << 0);
    pub const MAP_WRITE: Self = Self(1 << 1);
    pub const COPY_SRC: Self = Self(1 << 2);
    pub const COPY_DST: Self = Self(1 << 3);
    pub const INDEX: Self = Self(1 << 4);
    pub const VERTEX: Self = Self(1 << 5);
    pub const UNIFORM: Self = Self(1 << 6);
    pub const STORAGE: Self = Self(1 << 7);
    pub const INDIRECT: Self = Self(1 << 8);
    pub const QUERY_RESOLVE: Self = Self(1 << 9);
    pub const BLAS_INPUT: Self = Self(1 << 10);
    pub const TLAS_INPUT: Self = Self(1 << 11);
}

impl_bitflags!(TextureUsages, u32);
impl TextureUsages {
    pub const COPY_SRC: Self = Self(1 << 0);
    pub const COPY_DST: Self = Self(1 << 1);
    pub const TEXTURE_BINDING: Self = Self(1 << 2);
    pub const STORAGE_BINDING: Self = Self(1 << 3);
    pub const RENDER_ATTACHMENT: Self = Self(1 << 4);
}

impl_bitflags!(ShaderStages, u32);
impl ShaderStages {
    pub const NONE: Self = Self(0);
    pub const VERTEX: Self = Self(1 << 0);
    pub const FRAGMENT: Self = Self(1 << 1);
    pub const COMPUTE: Self = Self(1 << 2);
    pub const VERTEX_FRAGMENT: Self = Self(Self::VERTEX.0 | Self::FRAGMENT.0);
}

impl_bitflags!(ColorWrites, u32);
impl ColorWrites {
    pub const RED: Self = Self(1 << 0);
    pub const GREEN: Self = Self(1 << 1);
    pub const BLUE: Self = Self(1 << 2);
    pub const ALPHA: Self = Self(1 << 3);
    pub const COLOR: Self = Self(Self::RED.0 | Self::GREEN.0 | Self::BLUE.0);
    pub const ALL: Self = Self(0xF);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    Noop,
    Vulkan,
    Metal,
    Dx12,
    Gl,
    BrowserWebGpu,
    Cuda,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    Other,
    IntegratedGpu,
    DiscreteGpu,
    VirtualGpu,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PowerPreference {
    #[default]
    None,
    LowPower,
    HighPerformance,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryBudgetThresholds {
    pub for_resource_creation: Option<u8>,
    pub for_device_loss: Option<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct BackendOptions {
    pub gl: (),
    pub dx12: (),
    pub noop: (),
}

#[derive(Debug, Clone)]
pub struct InstanceDescriptor {
    pub backends: Backends,
    pub flags: InstanceFlags,
    pub memory_budget_thresholds: MemoryBudgetThresholds,
    pub backend_options: BackendOptions,
    pub display: Option<()>,
}

impl Default for InstanceDescriptor {
    fn default() -> Self {
        Self {
            backends: Backends::PRIMARY,
            flags: InstanceFlags::empty(),
            memory_budget_thresholds: MemoryBudgetThresholds::default(),
            backend_options: BackendOptions::default(),
            display: None,
        }
    }
}

impl InstanceDescriptor {
    pub fn new_without_display_handle() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
pub struct AdapterInfo {
    pub name: String,
    pub vendor: u32,
    pub device: u32,
    pub device_type: DeviceType,
    pub device_pci_bus_id: String,
    pub driver: String,
    pub driver_info: String,
    pub backend: Backend,
    pub subgroup_min_size: u32,
    pub subgroup_max_size: u32,
    pub transient_saves_memory: bool,
}

#[derive(Debug, Clone)]
pub struct Limits {
    pub max_texture_dimension_1d: u32,
    pub max_texture_dimension_2d: u32,
    pub max_texture_dimension_3d: u32,
    pub max_texture_array_layers: u32,
    pub max_bind_groups: u32,
    pub max_bindings_per_bind_group: u32,
    pub max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    pub max_dynamic_storage_buffers_per_pipeline_layout: u32,
    pub max_sampled_textures_per_shader_stage: u32,
    pub max_samplers_per_shader_stage: u32,
    pub max_storage_buffers_per_shader_stage: u32,
    pub max_storage_textures_per_shader_stage: u32,
    pub max_uniform_buffers_per_shader_stage: u32,
    pub max_uniform_buffer_binding_size: u64,
    pub max_storage_buffer_binding_size: u64,
    pub max_buffer_size: u64,
    pub max_vertex_buffers: u32,
    pub max_vertex_attributes: u32,
    pub max_vertex_buffer_array_stride: u32,
    pub min_uniform_buffer_offset_alignment: u32,
    pub min_storage_buffer_offset_alignment: u32,
    pub max_inter_stage_shader_components: u32,
    pub max_color_attachments: u32,
    pub max_color_attachment_bytes_per_sample: u32,
    pub max_compute_workgroup_storage_size: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroup_size_y: u32,
    pub max_compute_workgroup_size_z: u32,
    pub max_compute_workgroups_per_dimension: u32,
    pub max_immediate_size: u32,
    pub max_non_sampler_bindings: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_texture_dimension_1d: 16384,
            max_texture_dimension_2d: 16384,
            max_texture_dimension_3d: 2048,
            max_texture_array_layers: 2048,
            max_bind_groups: 4,
            max_bindings_per_bind_group: 1000,
            max_dynamic_uniform_buffers_per_pipeline_layout: 8,
            max_dynamic_storage_buffers_per_pipeline_layout: 4,
            max_sampled_textures_per_shader_stage: 16,
            max_samplers_per_shader_stage: 16,
            max_storage_buffers_per_shader_stage: 16,
            max_storage_textures_per_shader_stage: 8,
            max_uniform_buffers_per_shader_stage: 12,
            max_uniform_buffer_binding_size: 65536,
            max_storage_buffer_binding_size: 1 << 30,
            max_buffer_size: 1 << 31,
            max_vertex_buffers: 8,
            max_vertex_attributes: 16,
            max_vertex_buffer_array_stride: 2048,
            min_uniform_buffer_offset_alignment: 256,
            min_storage_buffer_offset_alignment: 32,
            max_inter_stage_shader_components: 60,
            max_color_attachments: 8,
            max_color_attachment_bytes_per_sample: 64,
            max_compute_workgroup_storage_size: 32768,
            max_compute_invocations_per_workgroup: 1024,
            max_compute_workgroup_size_x: 1024,
            max_compute_workgroup_size_y: 1024,
            max_compute_workgroup_size_z: 64,
            max_compute_workgroups_per_dimension: 65535,
            max_immediate_size: 256,
            max_non_sampler_bindings: 1_000_000,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RequestAdapterOptions<'a, 'b> {
    pub power_preference: PowerPreference,
    pub force_fallback_adapter: bool,
    pub compatible_surface: Option<&'a &'b ()>,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryHints;

impl MemoryHints {
    #[allow(non_upper_case_globals)]
    pub const Performance: Self = Self;
    #[allow(non_upper_case_globals)]
    pub const MemoryUsage: Self = Self;
}

#[derive(Debug, Clone, Default)]
pub struct Trace;

impl Trace {
    #[allow(non_upper_case_globals)]
    pub const Off: Self = Self;
}

#[derive(Debug, Clone, Default)]
pub struct ExperimentalFeatures;

impl ExperimentalFeatures {
    pub const fn disabled() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub struct DeviceDescriptor<'a> {
    pub label: Label<'a>,
    pub required_features: Features,
    pub required_limits: Limits,
    pub experimental_features: ExperimentalFeatures,
    pub memory_hints: MemoryHints,
    pub trace: Trace,
}

impl Default for DeviceDescriptor<'_> {
    fn default() -> Self {
        Self {
            label: None,
            required_features: Features::empty(),
            required_limits: Limits::default(),
            experimental_features: ExperimentalFeatures::disabled(),
            memory_hints: MemoryHints::Performance,
            trace: Trace::Off,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestDeviceError(pub String);
impl std::fmt::Display for RequestDeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for RequestDeviceError {}

#[derive(Debug, Clone)]
pub struct RequestAdapterError(pub String);
impl std::fmt::Display for RequestAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for RequestAdapterError {}

#[derive(Debug, Clone)]
pub enum Error {
    OutOfMemory {
        source: Arc<dyn std::error::Error + Send + Sync>,
    },
    Validation {
        source: Arc<dyn std::error::Error + Send + Sync>,
        description: String,
    },
    Internal {
        source: Arc<dyn std::error::Error + Send + Sync>,
        description: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfMemory { source } => write!(f, "Out of memory: {source}"),
            Self::Validation { description, .. } => write!(f, "Validation error: {description}"),
            Self::Internal { description, .. } => write!(f, "Internal error: {description}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorFilter {
    OutOfMemory,
    Validation,
    Internal,
}

pub struct ErrorScopeGuard {
    stack: Arc<Mutex<Vec<(ErrorFilter, Option<Error>)>>>,
}

impl ErrorScopeGuard {
    pub fn pop(self) -> impl Future<Output = Option<Error>> {
        let err = self
            .stack
            .lock()
            .ok()
            .and_then(|mut s| s.pop())
            .and_then(|(_, e)| e);
        std::future::ready(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubmissionIndex(pub u64);

#[derive(Debug, Clone)]
pub enum PollType {
    Poll,
    Wait {
        submission_index: Option<SubmissionIndex>,
        timeout: Option<std::time::Duration>,
    },
}

impl PollType {
    pub fn wait_indefinitely() -> Self {
        Self::Wait {
            submission_index: None,
            timeout: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollStatus {
    QueueEmpty,
    WaitSucceeded,
    Poll,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollError {
    Timeout,
    WrongSubmissionIndex(SubmissionIndex, SubmissionIndex),
}

impl std::fmt::Display for PollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "Poll wait timed out"),
            Self::WrongSubmissionIndex(a, b) => {
                write!(f, "Wrong submission index: {:?} vs {:?}", a, b)
            }
        }
    }
}
impl std::error::Error for PollError {}

#[derive(Clone)]
pub struct Instance {
    desc: InstanceDescriptor,
}

impl Default for Instance {
    fn default() -> Self {
        Self::new(InstanceDescriptor::default())
    }
}

impl Instance {
    pub fn new(desc: InstanceDescriptor) -> Self {
        Self { desc }
    }

    pub fn enumerate_adapters(&self, backends: Backends) -> impl Future<Output = Vec<Adapter>> {
        let mut list = Vec::new();
        if backends.contains(Backends::METAL) {
            if let Some(m) = MetalDevice::create_system_default() {
                list.push(Adapter::from_metal(m));
            }
        }
        std::future::ready(list)
    }

    pub fn request_adapter(
        &self,
        _options: &RequestAdapterOptions<'_, '_>,
    ) -> impl Future<Output = Result<Adapter, RequestAdapterError>> {
        let res = if let Some(m) = MetalDevice::create_system_default() {
            Ok(Adapter::from_metal(m))
        } else {
            Err(RequestAdapterError("No supported GPU adapter found".to_string()))
        };
        std::future::ready(res)
    }
}

#[derive(Clone)]
pub struct Adapter {
    pub(crate) metal: MetalDevice,
    pub(crate) info: AdapterInfo,
    pub(crate) features: Features,
    pub(crate) limits: Limits,
}

impl Adapter {
    pub(crate) fn from_metal(metal: MetalDevice) -> Self {
        let device_type = if metal.has_unified_memory {
            DeviceType::IntegratedGpu
        } else {
            DeviceType::DiscreteGpu
        };
        let info = AdapterInfo {
            name: metal.name.clone(),
            vendor: 0x106B, // Apple
            device: 1,
            device_type,
            device_pci_bus_id: String::new(),
            driver: "Metal Direct Native Runtime".to_string(),
            driver_info: "ASD + UMA + dUMA Accelerated".to_string(),
            backend: Backend::Metal,
            subgroup_min_size: 32,
            subgroup_max_size: 32,
            transient_saves_memory: true,
        };
        let mut features = Features::TIMESTAMP_QUERY
            | Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
            | Features::TIMESTAMP_QUERY_INSIDE_PASSES
            | Features::SHADER_F16
            | Features::FLOAT32_FILTERABLE
            | Features::IMMEDIATE_DATA
            | Features::SUBGROUP;
        if metal.has_unified_memory {
            features |= Features::MAPPABLE_PRIMARY_BUFFERS;
        }
        let mut limits = Limits::default();
        limits.max_buffer_size = metal.max_buffer_length;
        limits.max_storage_buffer_binding_size = metal.max_buffer_length;
        Self {
            metal,
            info,
            features,
            limits,
        }
    }

    pub fn get_info(&self) -> AdapterInfo {
        self.info.clone()
    }

    pub fn features(&self) -> Features {
        self.features
    }

    pub fn limits(&self) -> Limits {
        self.limits.clone()
    }

    pub fn request_device(
        &self,
        desc: &DeviceDescriptor<'_>,
    ) -> impl Future<Output = Result<(Device, Queue), RequestDeviceError>> {
        let dev = Device {
            metal: self.metal.clone(),
            info: self.info.clone(),
            features: desc.required_features & self.features,
            limits: desc.required_limits.clone(),
            error_stack: Arc::new(Mutex::new(Vec::new())),
        };
        let queue = Queue {
            metal: self.metal.clone(),
        };
        std::future::ready(Ok((dev, queue)))
    }
}

#[derive(Clone)]
pub struct Device {
    pub(crate) metal: MetalDevice,
    pub(crate) info: AdapterInfo,
    pub(crate) features: Features,
    pub(crate) limits: Limits,
    pub(crate) error_stack: Arc<Mutex<Vec<(ErrorFilter, Option<Error>)>>>,
}

impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.error_stack, &other.error_stack)
    }
}
impl Eq for Device {}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Device")
            .field("name", &self.metal.name)
            .finish()
    }
}

#[derive(Clone)]
pub struct Queue {
    pub(crate) metal: MetalDevice,
}

impl std::fmt::Debug for Queue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Queue")
            .field("name", &self.metal.name)
            .finish()
    }
}

impl Device {
    pub fn adapter_info(&self) -> AdapterInfo {
        self.info.clone()
    }

    pub fn features(&self) -> Features {
        self.features
    }

    pub fn limits(&self) -> Limits {
        self.limits.clone()
    }

    pub fn push_error_scope(&self, filter: ErrorFilter) -> ErrorScopeGuard {
        if let Ok(mut stack) = self.error_stack.lock() {
            stack.push((filter, None));
        }
        ErrorScopeGuard {
            stack: self.error_stack.clone(),
        }
    }

    pub(crate) fn record_error(&self, err: Error) {
        let err_filter = match &err {
            Error::OutOfMemory { .. } => ErrorFilter::OutOfMemory,
            Error::Validation { .. } => ErrorFilter::Validation,
            Error::Internal { .. } => ErrorFilter::Internal,
        };
        if let Ok(mut stack) = self.error_stack.lock() {
            if let Some((_, slot)) = stack.iter_mut().rev().find(|(f, _)| *f == err_filter) {
                if slot.is_none() {
                    *slot = Some(err);
                }
            }
        }
    }

    pub fn poll(&self, poll_type: PollType) -> Result<PollStatus, PollError> {
        match poll_type {
            PollType::Poll => Ok(PollStatus::Poll),
            PollType::Wait {
                submission_index,
                timeout,
            } => {
                if let Some(sub_idx) = submission_index {
                    let current = self.metal.next_submission_index.load(Ordering::Relaxed);
                    if sub_idx.0 >= current {
                        return Err(PollError::Timeout);
                    }
                }
                if let Some(dur) = timeout {
                    if dur.is_zero() && !self.metal.is_last_cmd_completed() {
                        return Err(PollError::Timeout);
                    }
                }
                self.metal.wait_idle();
                Ok(PollStatus::WaitSucceeded)
            }
        }
    }

    pub fn create_buffer(&self, desc: &BufferDescriptor<'_>) -> Buffer {
        if desc.usage.is_empty() || desc.size > self.limits.max_buffer_size {
            let msg = format!(
                "Invalid buffer descriptor: size={}, usage={:?}",
                desc.size, desc.usage
            );
            self.record_error(Error::Validation {
                source: Arc::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    msg.clone(),
                )),
                description: msg,
            });
        }
        if desc.mapped_at_creation || desc.size >= 1_048_576 {
            self.metal.wait_idle();
        }
        let options = 0u64; // MTLResourceStorageModeShared: zero-copy UMA
        let (raw, ptr) = self.metal.alloc_buffer(desc.size as usize, options);
        Buffer {
            inner: Arc::new(BufferInner {
                device: self.metal.clone(),
                raw,
                ptr,
                size: desc.size,
                usage: desc.usage,
                options,
                mapped_ready: AtomicBool::new(desc.mapped_at_creation),
            }),
        }
    }

    /// ASD (Accelerated Storage Direct / Apple Silicon Direct) zero-copy page-aligned buffer creation
    pub unsafe fn create_buffer_no_copy(
        &self,
        page_aligned_ptr: *mut u8,
        page_aligned_size: usize,
        logical_size: u64,
        usage: BufferUsages,
    ) -> Option<Buffer> {
        let options = 0u64; // MTLResourceStorageModeShared
        let (raw, ptr) = self.metal.alloc_buffer_no_copy(
            page_aligned_ptr as *mut c_void,
            page_aligned_size,
            options,
        )?;
        Some(Buffer {
            inner: Arc::new(BufferInner {
                device: self.metal.clone(),
                raw,
                ptr,
                size: logical_size,
                usage,
                options: 0xFF, // Do not recycle external no-copy memory into pool
                mapped_ready: AtomicBool::new(true),
            }),
        })
    }

    pub fn create_shader_module(&self, desc: ShaderModuleDescriptor<'_>) -> ShaderModule {
        let wgsl_src = match &desc.source {
            ShaderSource::Wgsl(s) => s.as_ref(),
        };
        let module = match naga::front::wgsl::parse_str(wgsl_src) {
            Ok(m) => m,
            Err(e) => {
                self.record_error(Error::Validation {
                    source: Arc::new(e.clone()),
                    description: e.to_string(),
                });
                naga::Module::default()
            }
        };
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        let info = validator.validate(&module).unwrap_or_else(|_| {
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::empty(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&naga::Module::default())
            .unwrap()
        });
        let (msl_source, bindings) = naga::back::msl::write_string(
            &module,
            &info,
            &naga::back::msl::Options::default(),
            &naga::back::msl::PipelineOptions::default(),
        )
        .unwrap_or_else(|_| (String::new(), BindingMap {
            buffers: Default::default(),
            textures: Default::default(),
            samplers: Default::default(),
            immediate_slot: None,
            array_length_slots: Default::default(),
        }));

        ShaderModule {
            module: Arc::new(module),
            info: Arc::new(info),
            msl_source: Arc::new(msl_source),
            bindings: Arc::new(bindings),
        }
    }

    pub fn create_bind_group_layout(&self, desc: &BindGroupLayoutDescriptor<'_>) -> BindGroupLayout {
        BindGroupLayout {
            entries: desc.entries.to_vec(),
        }
    }

    pub fn create_bind_group(&self, desc: &BindGroupDescriptor<'_>) -> BindGroup {
        let mut buffers = Vec::new();
        let mut textures = Vec::new();
        let mut samplers = Vec::new();

        for entry in desc.entries {
            match &entry.resource {
                BindingResource::Buffer(b) => {
                    let byte_size = b
                        .size
                        .map(|s| s.get())
                        .unwrap_or_else(|| b.buffer.size().saturating_sub(b.offset));
                    buffers.push((entry.binding, b.buffer.clone(), b.offset, byte_size));
                }
                BindingResource::TextureView(tv) => {
                    textures.push((entry.binding, tv.texture.clone()));
                }
                BindingResource::Sampler(s) => {
                    samplers.push((entry.binding, (*s).clone()));
                }
            }
        }

        BindGroup {
            buffers,
            textures,
            samplers,
        }
    }

    pub fn create_pipeline_layout(&self, desc: &PipelineLayoutDescriptor<'_>) -> PipelineLayout {
        PipelineLayout {
            immediate_size: desc.immediate_size,
        }
    }

    pub fn create_compute_pipeline(&self, desc: &ComputePipelineDescriptor<'_>) -> ComputePipeline {
        let ep_name = desc
            .entry_point
            .or_else(|| {
                desc.module
                    .module
                    .entry_points
                    .iter()
                    .find(|ep| ep.stage == naga::ShaderStage::Compute)
                    .map(|ep| ep.name.as_str())
            })
            .unwrap_or("main");

        let wg_size = desc
            .module
            .module
            .entry_points
            .iter()
            .find(|ep| ep.name == ep_name)
            .map(|ep| ep.workgroup_size)
            .unwrap_or([64, 1, 1]);

        let msl_ep_name = if ep_name == "main" {
            "_wgpu_entry_main"
        } else {
            ep_name
        };

        let wg_total = wg_size[0].max(1) * wg_size[1].max(1) * wg_size[2].max(1);
        let pso = self
            .metal
            .compile_compute_pipeline(&desc.module.msl_source, msl_ep_name, wg_total)
            .expect("Metal compute pipeline compilation failed");

        ComputePipeline {
            pso,
            workgroup_size: wg_size,
            bindings: desc.module.bindings.clone(),
        }
    }

    pub fn create_render_pipeline(&self, desc: &RenderPipelineDescriptor<'_>) -> RenderPipeline {
        let vs_ep_raw = desc.vertex.entry_point.unwrap_or("vs");
        let vs_ep = if vs_ep_raw == "main" {
            "_wgpu_entry_main"
        } else {
            vs_ep_raw
        };
        let fs_state = desc.fragment.as_ref();
        let fs_ep_raw = fs_state.and_then(|f| f.entry_point).unwrap_or("fs");
        let fs_ep = if fs_ep_raw == "main" {
            "_wgpu_entry_main"
        } else {
            fs_ep_raw
        };

        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        desc.vertex.module.msl_source.hash(&mut hasher);
        vs_ep.hash(&mut hasher);
        fs_ep.hash(&mut hasher);
        for vb in desc.vertex.buffers.iter().flatten() {
            vb.array_stride.hash(&mut hasher);
            vb.step_mode.hash(&mut hasher);
            for attr in vb.attributes {
                attr.format.hash(&mut hasher);
                attr.offset.hash(&mut hasher);
                attr.shader_location.hash(&mut hasher);
            }
        }
        if let Some(fs) = fs_state {
            for target in fs.targets.iter().flatten() {
                target.format.hash(&mut hasher);
                target.write_mask.hash(&mut hasher);
                target.blend.is_some().hash(&mut hasher);
            }
        }
        if let Some(ds) = &desc.depth_stencil {
            ds.format.hash(&mut hasher);
            ds.depth_write_enabled.hash(&mut hasher);
            ds.depth_compare.hash(&mut hasher);
        }
        let cache_key = hasher.finish();

        if let Ok(cache) = self.metal.render_pipeline_cache.lock() {
            if let Some((pso, dss)) = cache.get(&cache_key) {
                return RenderPipeline {
                    pso: pso.clone(),
                    depth_stencil_state: dss.clone(),
                    primitive: desc.primitive,
                    depth_bias: desc.depth_stencil.as_ref().map(|d| d.bias).unwrap_or_default(),
                };
            }
        }

        #[cfg(target_os = "macos")]
        unsafe {
            let s = metal_selectors();
            let f_alloc: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_init: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_set_obj: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_set_u64: unsafe extern "C" fn(*mut c_void, *mut c_void, u64) =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_set_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, bool) =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_get_obj: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_sub: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);

            // Compile vertex library
            let f_lib: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
                *mut c_void,
                *mut *mut c_void,
            ) -> *mut c_void = std::mem::transmute(metal_msg_send as *const c_void);
            let mut err: *mut c_void = ptr::null_mut();
            let vs_src_ns = nsstring_from_str(&desc.vertex.module.msl_source);
            let vs_lib_ptr = f_lib(
                self.metal.raw.as_ptr(),
                s.new_library_with_source,
                vs_src_ns,
                ptr::null_mut(),
                &mut err,
            );
            if vs_lib_ptr.is_null() {
                panic!(
                    "Vertex MSL compilation failed: {}\n--- MSL ---\n{}",
                    metal::nserror_description(err),
                    desc.vertex.module.msl_source
                );
            }
            let vs_lib = ObjCObject(vs_lib_ptr);

            let f_fn: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);
            let vs_fn_ptr = f_fn(vs_lib.as_ptr(), s.new_function_with_name, nsstring_from_str(vs_ep));
            assert!(!vs_fn_ptr.is_null(), "Vertex function `{vs_ep}` not found");
            let vs_fn = ObjCObject(vs_fn_ptr);

            let rpd_ptr = f_init(f_alloc(s.cls_mtl_render_pipeline_descriptor, s.alloc), s.init);
            let rpd = ObjCObject(rpd_ptr);
            f_set_obj(rpd.as_ptr(), s.set_vertex_function, vs_fn.as_ptr());

            let _fs_lib_holder: Option<ObjCObject>;
            let _fs_fn_holder: Option<ObjCObject>;
            if let Some(fs) = fs_state {
                let fs_lib = if ptr::eq(
                    desc.vertex.module.msl_source.as_ptr(),
                    fs.module.msl_source.as_ptr(),
                ) {
                    vs_lib.clone()
                } else {
                    let fs_src_ns = nsstring_from_str(&fs.module.msl_source);
                    let p = f_lib(
                        self.metal.raw.as_ptr(),
                        s.new_library_with_source,
                        fs_src_ns,
                        ptr::null_mut(),
                        &mut err,
                    );
                    if p.is_null() {
                        panic!(
                            "Fragment MSL compilation failed: {}",
                            metal::nserror_description(err)
                        );
                    }
                    ObjCObject(p)
                };
                let fs_fn_ptr = f_fn(fs_lib.as_ptr(), s.new_function_with_name, nsstring_from_str(fs_ep));
                assert!(!fs_fn_ptr.is_null(), "Fragment function `{fs_ep}` not found");
                let fs_fn = ObjCObject(fs_fn_ptr);
                f_set_obj(rpd.as_ptr(), s.set_fragment_function, fs_fn.as_ptr());

                let color_atts = f_get_obj(rpd.as_ptr(), s.color_attachments);
                for (i, target_opt) in fs.targets.iter().enumerate() {
                    if let Some(target) = target_opt {
                        let att = f_sub(color_atts, s.object_at_indexed_subscript, i);
                        f_set_u64(att, s.set_pixel_format, target.format.to_mtl_pixel_format());
                        f_set_u64(att, s.set_write_mask, target.write_mask.bits() as u64);
                        if let Some(blend) = &target.blend {
                            f_set_bool(att, s.set_blending_enabled, true);
                            f_set_u64(att, s.set_source_rgb_blend_factor, blend.color.src_factor.to_mtl());
                            f_set_u64(att, s.set_destination_rgb_blend_factor, blend.color.dst_factor.to_mtl());
                            f_set_u64(att, s.set_rgb_blend_operation, blend.color.operation.to_mtl());
                            f_set_u64(att, s.set_source_alpha_blend_factor, blend.alpha.src_factor.to_mtl());
                            f_set_u64(att, s.set_destination_alpha_blend_factor, blend.alpha.dst_factor.to_mtl());
                            f_set_u64(att, s.set_alpha_blend_operation, blend.alpha.operation.to_mtl());
                        }
                    }
                }
                _fs_lib_holder = Some(fs_lib);
                _fs_fn_holder = Some(fs_fn);
            }

            // Vertex descriptor
            if !desc.vertex.buffers.is_empty() {
                let vd = f_get_obj(rpd.as_ptr(), s.vertex_descriptor);
                let attrs = f_get_obj(vd, s.attributes);
                let layouts = f_get_obj(vd, s.layouts);
                for (slot_idx, vb_opt) in desc.vertex.buffers.iter().enumerate() {
                    let Some(vb) = vb_opt else { continue };
                    let mtl_buf_idx = MSL_VERTEX_BUFFER_BASE_SLOT as usize + slot_idx;
                    let layout = f_sub(layouts, s.object_at_indexed_subscript, mtl_buf_idx);
                    f_set_u64(layout, s.set_stride, vb.array_stride);
                    let step_fn = match vb.step_mode {
                        VertexStepMode::Vertex => 1u64,   // MTLVertexStepFunctionPerVertex
                        VertexStepMode::Instance => 2u64, // MTLVertexStepFunctionPerInstance
                    };
                    f_set_u64(layout, s.set_step_function, step_fn);
                    f_set_u64(layout, s.set_step_rate, 1);

                    for attr in vb.attributes {
                        let a_desc = f_sub(
                            attrs,
                            s.object_at_indexed_subscript,
                            attr.shader_location as usize,
                        );
                        f_set_u64(a_desc, s.set_format, attr.format.to_mtl());
                        f_set_u64(a_desc, s.set_offset, attr.offset);
                        f_set_u64(a_desc, s.set_buffer_index, mtl_buf_idx as u64);
                    }
                }
            }

            // Depth stencil
            let mut depth_stencil_state = None;
            if let Some(ds) = &desc.depth_stencil {
                f_set_u64(
                    rpd.as_ptr(),
                    s.set_depth_attachment_pixel_format,
                    ds.format.to_mtl_pixel_format(),
                );
                let dsd_ptr = f_init(f_alloc(s.cls_mtl_depth_stencil_descriptor, s.alloc), s.init);
                let dsd = ObjCObject(dsd_ptr);
                f_set_bool(
                    dsd.as_ptr(),
                    s.set_depth_write_enabled,
                    ds.depth_write_enabled.unwrap_or(false),
                );
                f_set_u64(
                    dsd.as_ptr(),
                    s.set_depth_compare_function,
                    ds.depth_compare.unwrap_or(CompareFunction::Always).to_mtl(),
                );
                let f_new_dss: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(metal_msg_send as *const c_void);
                let dss_ptr = f_new_dss(self.metal.raw.as_ptr(), s.new_depth_stencil_state, dsd.as_ptr());
                if !dss_ptr.is_null() {
                    depth_stencil_state = Some(Arc::new(ObjCObject(dss_ptr)));
                }
            }

            if desc.multisample.count > 1 {
                f_set_u64(rpd.as_ptr(), s.set_sample_count, desc.multisample.count as u64);
            }

            let f_new_rpso: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
                *mut *mut c_void,
            ) -> *mut c_void = std::mem::transmute(metal_msg_send as *const c_void);
            let mut pso_err: *mut c_void = ptr::null_mut();
            let pso_ptr = f_new_rpso(
                self.metal.raw.as_ptr(),
                s.new_render_pipeline_with_desc,
                rpd.as_ptr(),
                &mut pso_err,
            );
            if pso_ptr.is_null() {
                panic!(
                    "MTLRenderPipelineState creation failed: {}",
                    metal::nserror_description(pso_err)
                );
            }
            let pso = Arc::new(ObjCObject(pso_ptr));
            if let Ok(mut cache) = self.metal.render_pipeline_cache.lock() {
                cache.insert(cache_key, (pso.clone(), depth_stencil_state.clone()));
            }
            RenderPipeline {
                pso,
                depth_stencil_state,
                primitive: desc.primitive,
                depth_bias: desc.depth_stencil.as_ref().map(|d| d.bias).unwrap_or_default(),
            }
        }
    }

    pub fn create_texture(&self, desc: &TextureDescriptor<'_>) -> Texture {
        #[cfg(target_os = "macos")]
        unsafe {
            let s = metal_selectors();
            let f_td: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                u64,
                usize,
                usize,
                bool,
            ) -> *mut c_void = std::mem::transmute(metal_msg_send as *const c_void);
            let f_set_u64: unsafe extern "C" fn(*mut c_void, *mut c_void, u64) =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_new_tex: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);

            let td = f_td(
                s.cls_mtl_texture_descriptor,
                s.tex_desc_2d,
                desc.format.to_mtl_pixel_format(),
                desc.size.width.max(1) as usize,
                desc.size.height.max(1) as usize,
                desc.mip_level_count > 1,
            );
            f_set_u64(td, s.set_mipmap_level_count, desc.mip_level_count.max(1) as u64);
            if desc.sample_count > 1 {
                f_set_u64(td, s.set_sample_count, desc.sample_count as u64);
                f_set_u64(td, s.set_texture_type, 4); // MTLTextureType2DMultisample
            } else if desc.dimension == TextureDimension::D3 {
                f_set_u64(td, s.set_texture_type, 7); // MTLTextureType3D
                f_set_u64(td, s.set_depth, desc.size.depth_or_array_layers.max(1) as u64);
            } else if desc.size.depth_or_array_layers > 1 {
                f_set_u64(td, s.set_texture_type, 3); // MTLTextureType2DArray
                f_set_u64(td, s.set_array_length, desc.size.depth_or_array_layers as u64);
            }

            let mut mtl_usage = 0u64;
            if desc.usage.contains(TextureUsages::TEXTURE_BINDING)
                || desc.usage.contains(TextureUsages::COPY_SRC)
            {
                mtl_usage |= 1; // MTLTextureUsageShaderRead
            }
            if desc.usage.contains(TextureUsages::STORAGE_BINDING) {
                mtl_usage |= 2; // MTLTextureUsageShaderWrite
            }
            if desc.usage.contains(TextureUsages::RENDER_ATTACHMENT) {
                mtl_usage |= 4; // MTLTextureUsageRenderTarget
            }
            f_set_u64(td, s.set_usage, mtl_usage.max(1));

            // Use Shared storage mode (0) on Apple Silicon for colour textures so CPU/GPU transfers are fast,
            // and Private storage mode (2) for depth/multisample attachments as required by Metal!
            let is_depth = matches!(
                desc.format,
                TextureFormat::Depth32Float | TextureFormat::Depth24Plus
            );
            let storage_mode = if is_depth || desc.sample_count > 1 || !self.metal.has_unified_memory
            {
                2u64 // MTLStorageModePrivate
            } else {
                0u64 // MTLStorageModeShared
            };
            f_set_u64(td, s.set_storage_mode, storage_mode);

            let tex_ptr = f_new_tex(self.metal.raw.as_ptr(), s.new_texture_with_desc, td);
            assert!(!tex_ptr.is_null(), "Failed to create MTLTexture");
            Texture {
                raw: Arc::new(ObjCObject(tex_ptr)),
                size: desc.size,
                mip_level_count: desc.mip_level_count,
                sample_count: desc.sample_count,
                dimension: desc.dimension,
                format: desc.format,
                usage: desc.usage,
                storage_mode,
            }
        }
    }

    pub fn create_sampler(&self, desc: &SamplerDescriptor<'_>) -> Sampler {
        #[cfg(target_os = "macos")]
        unsafe {
            let s = metal_selectors();
            let f_alloc: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_init: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_set_u64: unsafe extern "C" fn(*mut c_void, *mut c_void, u64) =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_new_samp: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);

            let sd_ptr = f_init(f_alloc(s.cls_mtl_sampler_descriptor, s.alloc), s.init);
            let sd = ObjCObject(sd_ptr);
            f_set_u64(sd.as_ptr(), s.set_min_filter, desc.min_filter.to_mtl());
            f_set_u64(sd.as_ptr(), s.set_mag_filter, desc.mag_filter.to_mtl());
            let mip = match desc.mipmap_filter {
                MipmapFilterMode::Nearest => 1u64,
                MipmapFilterMode::Linear => 2u64,
            };
            f_set_u64(sd.as_ptr(), s.set_mip_filter, mip);
            f_set_u64(sd.as_ptr(), s.set_s_address_mode, desc.address_mode_u.to_mtl());
            f_set_u64(sd.as_ptr(), s.set_t_address_mode, desc.address_mode_v.to_mtl());
            f_set_u64(sd.as_ptr(), s.set_r_address_mode, desc.address_mode_w.to_mtl());
            if let Some(cmp) = desc.compare {
                f_set_u64(sd.as_ptr(), s.set_compare_function, cmp.to_mtl());
            }

            let samp_ptr = f_new_samp(self.metal.raw.as_ptr(), s.new_sampler_state, sd.as_ptr());
            Sampler {
                raw: Arc::new(ObjCObject(samp_ptr)),
            }
        }
    }

    pub fn create_query_set(&self, desc: &QuerySetDescriptor<'_>) -> QuerySet {
        QuerySet {
            count: desc.count,
            timestamps_ns: Arc::new(Mutex::new(vec![0u64; desc.count as usize])),
        }
    }

    pub fn create_command_encoder(&self, _desc: &CommandEncoderDescriptor<'_>) -> CommandEncoder {
        CommandEncoder {
            device: self.metal.clone(),
            error_stack: self.error_stack.clone(),
            ops: Vec::with_capacity(16),
            on_submit_callbacks: Mutex::new(Vec::new()),
        }
    }
}

pub struct WriteOnly<'a, T: ?Sized> {
    slice: &'a mut T,
}

impl<'a> WriteOnly<'a, [u8]> {
    #[inline]
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self { slice }
    }

    #[inline]
    pub fn copy_from_slice(&mut self, src: &[u8]) {
        self.slice.copy_from_slice(src);
    }

    #[inline]
    pub fn fill(&mut self, value: u8) {
        self.slice.fill(value);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.slice.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    #[inline]
    pub fn subslice(&mut self, range: Range<usize>) -> WriteOnly<'_, [u8]> {
        WriteOnly {
            slice: &mut self.slice[range],
        }
    }

    #[inline]
    pub fn slice<S: RangeBounds<usize>>(&mut self, bounds: S) -> WriteOnly<'_, [u8]> {
        let start = match bounds.start_bound() {
            Bound::Included(&v) => v,
            Bound::Excluded(&v) => v + 1,
            Bound::Unbounded => 0,
        };
        let end = match bounds.end_bound() {
            Bound::Included(&v) => v + 1,
            Bound::Excluded(&v) => v,
            Bound::Unbounded => self.slice.len(),
        };
        WriteOnly {
            slice: &mut self.slice[start..end],
        }
    }

    #[inline]
    pub fn into_slice<S: RangeBounds<usize>>(self, bounds: S) -> WriteOnly<'a, [u8]> {
        let start = match bounds.start_bound() {
            Bound::Included(&v) => v,
            Bound::Excluded(&v) => v + 1,
            Bound::Unbounded => 0,
        };
        let end = match bounds.end_bound() {
            Bound::Included(&v) => v + 1,
            Bound::Excluded(&v) => v,
            Bound::Unbounded => self.slice.len(),
        };
        WriteOnly {
            slice: &mut self.slice[start..end],
        }
    }

    #[inline]
    pub fn into_chunks<const N: usize>(self) -> (WriteOnly<'a, [[u8; N]]>, WriteOnly<'a, [u8]>) {
        assert!(N > 0);
        let len = self.slice.len();
        let num_chunks = len / N;
        let split_at = num_chunks * N;
        let (head, tail) = self.slice.split_at_mut(split_at);
        let chunks =
            unsafe { std::slice::from_raw_parts_mut(head.as_mut_ptr() as *mut [u8; N], num_chunks) };
        (WriteOnly { slice: chunks }, WriteOnly { slice: tail })
    }

    #[inline]
    pub fn chunks_exact_mut(&mut self, chunk_size: usize) -> impl Iterator<Item = WriteOnly<'_, [u8]>> {
        self.slice
            .chunks_exact_mut(chunk_size)
            .map(|c| WriteOnly { slice: c })
    }
}

impl<'a, const N: usize> WriteOnly<'a, [[u8; N]]> {
    #[inline]
    pub fn write_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = [u8; N]>,
    {
        for (dst, val) in self.slice.iter_mut().zip(iter) {
            *dst = val;
        }
    }
}

impl<'a> Deref for WriteOnly<'a, [u8]> {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl<'a> DerefMut for WriteOnly<'a, [u8]> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.slice
    }
}

impl Queue {
    #[inline]
    pub fn write_buffer(&self, buffer: &Buffer, offset: BufferAddress, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.metal.flush_pending_compute();
        if !self.metal.is_last_cmd_completed() {
            self.metal.wait_idle();
        }
        if buffer.inner.ptr.is_null() {
            // Private buffer: stage via temporary shared buffer + blit
            #[cfg(target_os = "macos")]
            unsafe {
                let (stg_raw, _) = self.metal.alloc_buffer_with_bytes(data, 0);
                let s = metal_selectors();
                let f_obj: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(metal_msg_send as *const c_void);
                let f_void: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                    std::mem::transmute(metal_msg_send as *const c_void);
                let f_copy: unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    *mut c_void,
                    usize,
                    *mut c_void,
                    usize,
                    usize,
                ) = std::mem::transmute(metal_msg_send as *const c_void);
                let cmd_buf = f_obj(self.metal.queue.as_ptr(), s.command_buffer);
                let blit = f_obj(cmd_buf, s.blit_command_encoder);
                f_copy(
                    blit,
                    s.copy_from_buffer,
                    stg_raw.as_ptr(),
                    0,
                    buffer.inner.raw.as_ptr(),
                    offset as usize,
                    data.len(),
                );
                f_void(blit, s.end_encoding);
                f_void(cmd_buf, s.commit);
                f_void(cmd_buf, s.wait_until_completed);
            }
            return;
        }
        unsafe {
            let dst = buffer.inner.ptr.add(offset as usize);
            ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        }
    }

    #[inline]
    pub fn write_buffer_with<'a>(
        &'a self,
        buffer: &'a Buffer,
        offset: BufferAddress,
        size: BufferSize,
    ) -> Option<WriteOnly<'a, [u8]>> {
        let len = size.get() as usize;
        if offset as usize + len > buffer.inner.size as usize || buffer.inner.ptr.is_null() {
            return None;
        }
        let slice = unsafe {
            std::slice::from_raw_parts_mut(buffer.inner.ptr.add(offset as usize), len)
        };
        Some(WriteOnly::new(slice))
    }

    pub fn write_texture(
        &self,
        texture: TexelCopyTextureInfo<'_>,
        data: &[u8],
        data_layout: TexelCopyBufferLayout,
        size: Extent3d,
    ) {
        if data.is_empty() || size.width == 0 || size.height == 0 {
            return;
        }
        #[cfg(target_os = "macos")]
        unsafe {
            let s = metal_selectors();
            let bpr = data_layout
                .bytes_per_row
                .unwrap_or(size.width * texture.texture.format.bytes_per_pixel())
                as usize;
            let bpi = data_layout
                .rows_per_image
                .map(|r| r as usize * bpr)
                .unwrap_or(size.height as usize * bpr);
            let src_ptr = data.as_ptr().add(data_layout.offset as usize) as *const c_void;

            if texture.texture.storage_mode == 0 {
                // MTLStorageModeShared: direct CPU-to-unified-texture replaceRegion!
                let f_rep: unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    MTLRegion,
                    usize,
                    usize,
                    *const c_void,
                    usize,
                    usize,
                ) = std::mem::transmute(metal_msg_send as *const c_void);
                let depth = if texture.texture.dimension == TextureDimension::D3 {
                    size.depth_or_array_layers.max(1) as usize
                } else {
                    1usize
                };
                let slices = if texture.texture.dimension == TextureDimension::D3 {
                    1usize
                } else {
                    size.depth_or_array_layers.max(1) as usize
                };
                for slice_idx in 0..slices {
                    let region = MTLRegion {
                        origin: MTLOrigin {
                            x: texture.origin.x as usize,
                            y: texture.origin.y as usize,
                            z: if texture.texture.dimension == TextureDimension::D3 {
                                texture.origin.z as usize
                            } else {
                                0
                            },
                        },
                        size: MTLSize {
                            width: size.width as usize,
                            height: size.height as usize,
                            depth,
                        },
                    };
                    let slice_ptr = (src_ptr as *const u8).add(slice_idx * bpi) as *const c_void;
                    f_rep(
                        texture.texture.raw.as_ptr(),
                        s.replace_region,
                        region,
                        texture.mip_level as usize,
                        if texture.texture.dimension == TextureDimension::D3 {
                            0
                        } else {
                            texture.origin.z as usize + slice_idx
                        },
                        slice_ptr,
                        bpr,
                        bpi,
                    );
                }
            }
        }
    }

    pub fn submit<I: IntoIterator<Item = CommandBuffer>>(&self, command_buffers: I) -> SubmissionIndex {
        let idx = self
            .metal
            .next_submission_index
            .fetch_add(1, Ordering::Relaxed);

        #[cfg(target_os = "macos")]
        unsafe {
            let cbs: Vec<CommandBuffer> = command_buffers.into_iter().collect();
            let is_pure_untimed_compute = !cbs.is_empty()
                && cbs.iter().all(|cb| {
                    cb.on_submit_callbacks.is_empty()
                        && !cb.ops.is_empty()
                        && cb.ops.iter().all(|op| {
                            matches!(
                                op,
                                RecordedOp::ComputePass {
                                    dispatches,
                                    start_ts: None,
                                    end_ts: None,
                                } if !dispatches.is_empty()
                            )
                        })
                });
            if is_pure_untimed_compute {
                let mut should_flush = false;
                if let Ok(mut pending) = self.metal.pending_compute_ops.lock() {
                    for cb in cbs {
                        for op in cb.ops {
                            if let RecordedOp::ComputePass { mut dispatches, .. } = op {
                                pending.append(&mut dispatches);
                            }
                        }
                    }
                    if pending.len() >= 1 {
                        should_flush = true;
                    }
                }
                if should_flush {
                    self.metal.flush_pending_compute();
                }
                return SubmissionIndex(idx);
            }

            let is_callback_only = cbs.iter().all(|cb| cb.ops.is_empty());
            if is_callback_only {
                self.metal.wait_idle();
                for cb in cbs {
                    for callback in cb.on_submit_callbacks {
                        callback();
                    }
                }
                return SubmissionIndex(idx);
            }

            let is_small_uma_copy = cbs.len() == 1
                && cbs[0].ops.len() == 1
                && matches!(
                    &cbs[0].ops[0],
                    RecordedOp::CopyBufferToBuffer { src, dst, size, .. }
                        if *size <= 65536 && !src.inner.ptr.is_null() && !dst.inner.ptr.is_null()
                );
            if is_small_uma_copy {
                self.metal.wait_idle();
                let mut cb = cbs.into_iter().next().unwrap();
                if let Some(RecordedOp::CopyBufferToBuffer {
                    src,
                    src_offset,
                    dst,
                    dst_offset,
                    size,
                }) = cb.ops.pop()
                {
                    if size > 0 {
                        ptr::copy_nonoverlapping(
                            src.inner.ptr.add(src_offset as usize),
                            dst.inner.ptr.add(dst_offset as usize),
                            size as usize,
                        );
                    }
                }
                for callback in cb.on_submit_callbacks {
                    callback();
                }
                return SubmissionIndex(idx);
            }

            let mut pending: Vec<ComputeDispatchOp> = self
                .metal
                .pending_compute_ops
                .lock()
                .map(|mut v| std::mem::take(&mut *v))
                .unwrap_or_default();

            let s = metal_selectors();
            let f_obj: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(metal_msg_send as *const c_void);
            let f_void: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                std::mem::transmute(metal_msg_send as *const c_void);

            let cmd_buf_ptr = f_obj(self.metal.queue.as_ptr(), s.command_buffer);
            assert!(!cmd_buf_ptr.is_null(), "Failed to create MTLCommandBuffer");
            let cmd_buf = Arc::new(ObjCObject::from_unretained(cmd_buf_ptr).unwrap());

            let mut active_compute_enc: *mut c_void = ptr::null_mut();
            if !pending.is_empty() {
                active_compute_enc = f_obj(cmd_buf.as_ptr(), s.compute_command_encoder);
                encode_compute_dispatches(active_compute_enc, &pending);
                pending.clear();
            }
            let mut callbacks: Vec<Box<dyn FnOnce() + Send>> = Vec::new();

            for mut cb in cbs {
                callbacks.append(&mut cb.on_submit_callbacks);
                for op in cb.ops {
                    match op {
                        RecordedOp::ComputePass {
                            dispatches,
                            start_ts,
                            end_ts,
                        } => {
                            if dispatches.is_empty() {
                                if let Some((qs, i)) = start_ts {
                                    encode_non_compute_op(
                                        &self.metal,
                                        cmd_buf.as_ptr(),
                                        RecordedOp::WriteTimestamp {
                                            query_set: qs,
                                            index: i,
                                        },
                                    );
                                }
                                if let Some((qs, i)) = end_ts {
                                    encode_non_compute_op(
                                        &self.metal,
                                        cmd_buf.as_ptr(),
                                        RecordedOp::WriteTimestamp {
                                            query_set: qs,
                                            index: i,
                                        },
                                    );
                                }
                                continue;
                            }
                            if start_ts.is_some() || end_ts.is_some() {
                                // Flush previous compute encoder so this timed pass gets isolated GPU hardware execution timing
                                if !active_compute_enc.is_null() {
                                    f_void(active_compute_enc, s.end_encoding);
                                    active_compute_enc = ptr::null_mut();
                                }
                                let sub_cmd_ptr = f_obj(self.metal.queue.as_ptr(), s.command_buffer);
                                let sub_enc = f_obj(sub_cmd_ptr, s.compute_command_encoder);
                                encode_compute_dispatches(sub_enc, &dispatches);
                                f_void(sub_enc, s.end_encoding);
                                f_void(sub_cmd_ptr, s.commit);
                                f_void(sub_cmd_ptr, s.wait_until_completed);

                                let f_f64: unsafe extern "C" fn(*mut c_void, *mut c_void) -> f64 =
                                    std::mem::transmute(metal_msg_send as *const c_void);
                                let t_start_s = f_f64(sub_cmd_ptr, s.gpu_start_time);
                                let t_end_s = f_f64(sub_cmd_ptr, s.gpu_end_time);
                                let now_ns = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_nanos() as u64)
                                    .unwrap_or(0);
                                let (start_ns, end_ns) = if t_start_s > 0.0 && t_end_s >= t_start_s {
                                    (
                                        (t_start_s * 1e9) as u64,
                                        (t_end_s * 1e9) as u64,
                                    )
                                } else {
                                    (now_ns, now_ns + 1000)
                                };
                                if let Some((qs, i)) = start_ts {
                                    if let Ok(mut ts) = qs.timestamps_ns.lock() {
                                        if (i as usize) < ts.len() {
                                            ts[i as usize] = start_ns;
                                        }
                                    }
                                }
                                if let Some((qs, i)) = end_ts {
                                    if let Ok(mut ts) = qs.timestamps_ns.lock() {
                                        if (i as usize) < ts.len() {
                                            ts[i as usize] = end_ns;
                                        }
                                    }
                                }
                                continue;
                            }
                            if active_compute_enc.is_null() {
                                active_compute_enc =
                                    f_obj(cmd_buf.as_ptr(), s.compute_command_encoder);
                            }
                            encode_compute_dispatches(active_compute_enc, &dispatches);
                        }
                        other => {
                            if !active_compute_enc.is_null() {
                                f_void(active_compute_enc, s.end_encoding);
                                active_compute_enc = ptr::null_mut();
                            }
                            encode_non_compute_op(&self.metal, cmd_buf.as_ptr(), other);
                        }
                    }
                }
            }

            if !active_compute_enc.is_null() {
                f_void(active_compute_enc, s.end_encoding);
            }

            f_void(cmd_buf.as_ptr(), s.commit);
            if let Ok(mut last) = self.metal.last_cmd_buf.lock() {
                *last = Some((cmd_buf, idx));
            }
            if !callbacks.is_empty() {
                self.metal.wait_idle();
                for cb in callbacks {
                    cb();
                }
            }
        }

        SubmissionIndex(idx)
    }

    pub fn on_submitted_work_done<F>(&self, callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.metal.wait_idle();
        callback();
    }

    pub fn get_timestamp_period(&self) -> f32 {
        1.0
    }
}

#[cfg(target_os = "macos")]
pub(crate) unsafe fn encode_compute_dispatches(enc: *mut c_void, dispatches: &[ComputeDispatchOp]) {
    let s = metal_selectors();
    let f_set_pso: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_buf: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, usize, usize) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_bytes: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void, usize, usize) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_tex: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, usize) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_samp: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, usize) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_dispatch: unsafe extern "C" fn(*mut c_void, *mut c_void, MTLSize, MTLSize) =
        std::mem::transmute(metal_msg_send as *const c_void);

    let mut last_pso: *mut c_void = ptr::null_mut();
    for d in dispatches {
        let pso_ptr = d.pipeline.pso.as_ptr();
        if pso_ptr != last_pso {
            f_set_pso(enc, s.set_compute_pipeline_state, pso_ptr);
            last_pso = pso_ptr;
        }
        let mut buffer_sizes = [0u32; 32];
        let any_buf_sizes = !d.pipeline.bindings.array_length_slots.is_empty();

        for (group_idx, bg_opt, dyn_offsets) in &d.bind_groups {
            let Some(bg) = bg_opt else { continue };
            for (b_idx, (binding, buf, base_off, byte_size)) in bg.buffers.iter().enumerate() {
                let slot = msl_buffer_slot(*group_idx, *binding) as usize;
                let dyn_off = dyn_offsets.get(b_idx).copied().unwrap_or(0) as u64;
                let final_off = (*base_off + dyn_off) as usize;
                f_set_buf(
                    enc,
                    s.set_buffer,
                    buf.inner.raw.as_ptr(),
                    final_off,
                    slot,
                );
                if slot < 32 {
                    let rem = (*byte_size).saturating_sub(dyn_off);
                    buffer_sizes[slot] = rem as u32;
                }
            }
            for (binding, tex) in &bg.textures {
                let slot = msl_texture_slot(*group_idx, *binding) as usize;
                f_set_tex(enc, s.set_texture, tex.raw.as_ptr(), slot);
            }
            for (binding, samp) in &bg.samplers {
                let slot = msl_sampler_slot(*group_idx, *binding) as usize;
                f_set_samp(enc, s.set_sampler_state, samp.raw.as_ptr(), slot);
            }
        }

        if any_buf_sizes {
            f_set_bytes(
                enc,
                s.set_bytes,
                buffer_sizes.as_ptr() as *const c_void,
                std::mem::size_of_val(&buffer_sizes),
                MSL_ARRAY_LENGTHS_BUFFER_SLOT as usize,
            );
        }

        if !d.immediates.is_empty() {
            f_set_bytes(
                enc,
                s.set_bytes,
                d.immediates.as_ptr() as *const c_void,
                d.immediates.len(),
                MSL_IMMEDIATE_BUFFER_SLOT as usize,
            );
        }

        let grid = MTLSize {
            width: d.groups[0].max(1) as usize,
            height: d.groups[1].max(1) as usize,
            depth: d.groups[2].max(1) as usize,
        };
        let tg = MTLSize {
            width: d.pipeline.workgroup_size[0].max(1) as usize,
            height: d.pipeline.workgroup_size[1].max(1) as usize,
            depth: d.pipeline.workgroup_size[2].max(1) as usize,
        };
        f_dispatch(enc, s.dispatch_threadgroups, grid, tg);
    }
}

#[cfg(target_os = "macos")]
unsafe fn encode_non_compute_op(metal: &MetalDevice, cmd_buf: *mut c_void, op: RecordedOp) {
        let s = metal_selectors();
        let f_obj: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(metal_msg_send as *const c_void);
        let f_void: unsafe extern "C" fn(*mut c_void, *mut c_void) =
            std::mem::transmute(metal_msg_send as *const c_void);

        match op {
            RecordedOp::CopyBufferToBuffer {
                src,
                src_offset,
                dst,
                dst_offset,
                size,
            } => {
                if size == 0 {
                    return;
                }
                let blit = f_obj(cmd_buf, s.blit_command_encoder);
            let f_copy: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
                usize,
                *mut c_void,
                usize,
                usize,
            ) = std::mem::transmute(metal_msg_send as *const c_void);
            f_copy(
                blit,
                s.copy_from_buffer,
                src.inner.raw.as_ptr(),
                src_offset as usize,
                dst.inner.raw.as_ptr(),
                dst_offset as usize,
                size as usize,
            );
            f_void(blit, s.end_encoding);
        }
        RecordedOp::ClearBuffer {
            dst,
            offset,
            size,
        } => {
            if size == 0 {
                return;
            }
            let blit = f_obj(cmd_buf, s.blit_command_encoder);
            let f_fill: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, NSRange, u8) =
                std::mem::transmute(metal_msg_send as *const c_void);
            f_fill(
                blit,
                s.fill_buffer,
                dst.inner.raw.as_ptr(),
                NSRange {
                    location: offset as usize,
                    length: size as usize,
                },
                0,
            );
            f_void(blit, s.end_encoding);
        }
        RecordedOp::CopyTextureToBuffer {
            src_tex,
            mip_level,
            origin,
            dst_buf,
            dst_offset,
            bytes_per_row,
            rows_per_image,
            size,
        } => {
            let blit = f_obj(cmd_buf, s.blit_command_encoder);
            let f_t2b: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
                usize,
                usize,
                MTLOrigin,
                MTLSize,
                *mut c_void,
                usize,
                usize,
                usize,
            ) = std::mem::transmute(metal_msg_send as *const c_void);
            let bpr = bytes_per_row as usize;
            let bpi = rows_per_image.unwrap_or(size.height) as usize * bpr;
            f_t2b(
                blit,
                s.copy_from_texture_to_buffer,
                src_tex.raw.as_ptr(),
                origin.z as usize,
                mip_level as usize,
                MTLOrigin {
                    x: origin.x as usize,
                    y: origin.y as usize,
                    z: 0,
                },
                MTLSize {
                    width: size.width as usize,
                    height: size.height as usize,
                    depth: 1,
                },
                dst_buf.inner.raw.as_ptr(),
                dst_offset as usize,
                bpr,
                bpi,
            );
            f_void(blit, s.end_encoding);
        }
        RecordedOp::CopyTextureToTexture {
            src_tex,
            src_mip,
            src_origin,
            dst_tex,
            dst_mip,
            dst_origin,
            size,
        } => {
            let blit = f_obj(cmd_buf, s.blit_command_encoder);
            let f_t2t: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
                usize,
                usize,
                MTLOrigin,
                MTLSize,
                *mut c_void,
                usize,
                usize,
                MTLOrigin,
            ) = std::mem::transmute(metal_msg_send as *const c_void);
            let slices = size.depth_or_array_layers.max(1) as usize;
            for slice_i in 0..slices {
                f_t2t(
                    blit,
                    s.copy_from_texture_to_texture,
                    src_tex.raw.as_ptr(),
                    src_origin.z as usize + slice_i,
                    src_mip as usize,
                    MTLOrigin {
                        x: src_origin.x as usize,
                        y: src_origin.y as usize,
                        z: 0,
                    },
                    MTLSize {
                        width: size.width as usize,
                        height: size.height as usize,
                        depth: 1,
                    },
                    dst_tex.raw.as_ptr(),
                    dst_origin.z as usize + slice_i,
                    dst_mip as usize,
                    MTLOrigin {
                        x: dst_origin.x as usize,
                        y: dst_origin.y as usize,
                        z: 0,
                    },
                );
            }
            f_void(blit, s.end_encoding);
        }
        RecordedOp::WriteTimestamp { query_set, index } => {
            let now_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            if let Ok(mut ts) = query_set.timestamps_ns.lock() {
                if (index as usize) < ts.len() {
                    ts[index as usize] = now_ns;
                }
            }
        }
        RecordedOp::ResolveQuerySet {
            query_set,
            first_query,
            query_count,
            destination,
            destination_offset,
        } => {
            if let Ok(ts) = query_set.timestamps_ns.lock() {
                let start = first_query as usize;
                let end = (start + query_count as usize).min(ts.len());
                if start < end && !destination.inner.ptr.is_null() {
                    let slice = &ts[start..end];
                    ptr::copy_nonoverlapping(
                        slice.as_ptr() as *const u8,
                        destination.inner.ptr.add(destination_offset as usize),
                        slice.len() * 8,
                    );
                }
            }
        }
        RecordedOp::RenderPass(rp) => {
            encode_render_pass(cmd_buf, rp);
        }
        RecordedOp::ComputePass { .. } => {}
    }
}

#[cfg(target_os = "macos")]
unsafe fn encode_render_pass(cmd_buf: *mut c_void, rp: RenderPassOp) {
    let s = metal_selectors();
    let f_obj: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_sub: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_obj: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_u64: unsafe extern "C" fn(*mut c_void, *mut c_void, u64) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_f64: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_cc: unsafe extern "C" fn(*mut c_void, *mut c_void, MTLClearColor) =
        std::mem::transmute(metal_msg_send as *const c_void);

    let rpd = f_obj(s.cls_mtl_render_pass_descriptor, s.render_pass_descriptor);
    let color_atts = f_obj(rpd, s.color_attachments);

    for (i, ca_opt) in rp.color_attachments.iter().enumerate() {
        if let Some(ca) = ca_opt {
            let att = f_sub(color_atts, s.object_at_indexed_subscript, i);
            f_set_obj(att, s.set_attachment_texture, ca.view.texture.raw.as_ptr());
            f_set_u64(att, s.set_level, ca.view.base_mip_level as u64);
            f_set_u64(att, s.set_slice, ca.view.base_array_layer as u64);
            match ca.ops.load {
                LoadOp::Clear(c) => {
                    f_set_u64(att, s.set_load_action, 2); // MTLLoadActionClear
                    f_set_cc(
                        att,
                        s.set_clear_color,
                        MTLClearColor {
                            red: c.r,
                            green: c.g,
                            blue: c.b,
                            alpha: c.a,
                        },
                    );
                }
                LoadOp::Load => {
                    f_set_u64(att, s.set_load_action, 1); // MTLLoadActionLoad
                }
            }
            let store = match ca.ops.store {
                StoreOp::Store => 1u64,   // MTLStoreActionStore
                StoreOp::Discard => 0u64, // MTLStoreActionDontCare
            };
            f_set_u64(att, s.set_store_action, store);
        }
    }

    if let Some(dsa) = &rp.depth_stencil_attachment {
        let d_att = f_obj(rpd, s.depth_attachment);
        f_set_obj(d_att, s.set_attachment_texture, dsa.view.texture.raw.as_ptr());
        f_set_u64(d_att, s.set_level, dsa.view.base_mip_level as u64);
        f_set_u64(d_att, s.set_slice, dsa.view.base_array_layer as u64);
        if let Some(d_ops) = &dsa.depth_ops {
            match d_ops.load {
                LoadOp::Clear(v) => {
                    f_set_u64(d_att, s.set_load_action, 2);
                    f_set_f64(d_att, s.set_clear_depth, v as f64);
                }
                LoadOp::Load => {
                    f_set_u64(d_att, s.set_load_action, 1);
                }
            }
            let store = match d_ops.store {
                StoreOp::Store => 1u64,
                StoreOp::Discard => 0u64,
            };
            f_set_u64(d_att, s.set_store_action, store);
        }
    }

    let f_new_enc: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(metal_msg_send as *const c_void);
    let enc = f_new_enc(cmd_buf, s.render_command_encoder_with_desc, rpd);
    assert!(!enc.is_null(), "Failed to create MTLRenderCommandEncoder");

    let f_set_buf: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, usize, usize) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_bytes: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void, usize, usize) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_tex: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, usize) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_samp: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, usize) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_vp: unsafe extern "C" fn(*mut c_void, *mut c_void, MTLViewport) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_sc: unsafe extern "C" fn(*mut c_void, *mut c_void, MTLScissorRect) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_set_bias: unsafe extern "C" fn(*mut c_void, *mut c_void, f32, f32, f32) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_draw: unsafe extern "C" fn(*mut c_void, *mut c_void, u64, usize, usize, usize, usize) =
        std::mem::transmute(metal_msg_send as *const c_void);
    let f_draw_idx: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        u64,
        usize,
        u64,
        *mut c_void,
        usize,
        usize,
        isize,
        usize,
    ) = std::mem::transmute(metal_msg_send as *const c_void);
    let f_void: unsafe extern "C" fn(*mut c_void, *mut c_void) =
        std::mem::transmute(metal_msg_send as *const c_void);

    let mut current_primitive_type = 3u64; // MTLPrimitiveTypeTriangle
    let mut active_index_buffer: Option<(Buffer, IndexFormat, u64)> = None;

    for cmd in rp.commands {
        match cmd {
            RenderCommand::SetPipeline(pipe) => {
                f_set_obj(enc, s.set_render_pipeline_state, pipe.pso.as_ptr());
                if let Some(dss) = &pipe.depth_stencil_state {
                    f_set_obj(enc, s.set_depth_stencil_state, dss.as_ptr());
                }
                let winding = match pipe.primitive.front_face {
                    FrontFace::Ccw => 1u64, // MTLWindingCounterClockwise
                    FrontFace::Cw => 0u64,  // MTLWindingClockwise
                };
                f_set_u64(enc, s.set_front_facing_winding, winding);
                let cull = match pipe.primitive.cull_mode {
                    None => 0u64,              // MTLCullModeNone
                    Some(Face::Front) => 1u64, // MTLCullModeFront
                    Some(Face::Back) => 2u64,  // MTLCullModeBack
                };
                f_set_u64(enc, s.set_cull_mode, cull);
                if pipe.depth_bias.constant != 0 || pipe.depth_bias.slope_scale != 0.0 {
                    f_set_bias(
                        enc,
                        s.set_depth_bias,
                        pipe.depth_bias.constant as f32,
                        pipe.depth_bias.slope_scale,
                        pipe.depth_bias.clamp,
                    );
                }
                current_primitive_type = pipe.primitive.topology.to_mtl();
            }
            RenderCommand::SetBindGroup {
                index,
                bind_group,
                offsets,
            } => {
                for (b_i, (binding, buf, base_off, _)) in bind_group.buffers.iter().enumerate() {
                    let slot = msl_buffer_slot(index, *binding) as usize;
                    let dyn_off = offsets.get(b_i).copied().unwrap_or(0) as u64;
                    let final_off = (*base_off + dyn_off) as usize;
                    f_set_buf(
                        enc,
                        s.set_vertex_buffer,
                        buf.inner.raw.as_ptr(),
                        final_off,
                        slot,
                    );
                    f_set_buf(
                        enc,
                        s.set_fragment_buffer,
                        buf.inner.raw.as_ptr(),
                        final_off,
                        slot,
                    );
                }
                for (binding, tex) in &bind_group.textures {
                    let slot = msl_texture_slot(index, *binding) as usize;
                    f_set_tex(enc, s.set_vertex_texture, tex.raw.as_ptr(), slot);
                    f_set_tex(enc, s.set_fragment_texture, tex.raw.as_ptr(), slot);
                }
                for (binding, samp) in &bind_group.samplers {
                    let slot = msl_sampler_slot(index, *binding) as usize;
                    f_set_samp(enc, s.set_vertex_sampler_state, samp.raw.as_ptr(), slot);
                    f_set_samp(enc, s.set_fragment_sampler_state, samp.raw.as_ptr(), slot);
                }
            }
            RenderCommand::SetVertexBuffer {
                slot,
                buffer,
                offset,
            } => {
                let mtl_slot = (MSL_VERTEX_BUFFER_BASE_SLOT + slot) as usize;
                f_set_buf(
                    enc,
                    s.set_vertex_buffer,
                    buffer.inner.raw.as_ptr(),
                    offset as usize,
                    mtl_slot,
                );
            }
            RenderCommand::SetIndexBuffer {
                buffer,
                format,
                offset,
            } => {
                active_index_buffer = Some((buffer, format, offset));
            }
            RenderCommand::SetImmediates { offset, data } => {
                let slot = MSL_IMMEDIATE_BUFFER_SLOT as usize;
                f_set_bytes(
                    enc,
                    s.set_vertex_bytes,
                    data.as_ptr() as *const c_void,
                    data.len(),
                    slot,
                );
                f_set_bytes(
                    enc,
                    s.set_fragment_bytes,
                    data.as_ptr() as *const c_void,
                    data.len(),
                    slot,
                );
            }
            RenderCommand::SetViewport {
                x,
                y,
                w,
                h,
                min_depth,
                max_depth,
            } => {
                f_set_vp(
                    enc,
                    s.set_viewport,
                    MTLViewport {
                        origin_x: x as f64,
                        origin_y: y as f64,
                        width: w as f64,
                        height: h as f64,
                        znear: min_depth as f64,
                        zfar: max_depth as f64,
                    },
                );
            }
            RenderCommand::SetScissorRect { x, y, w, h } => {
                f_set_sc(
                    enc,
                    s.set_scissor_rect,
                    MTLScissorRect {
                        x: x as usize,
                        y: y as usize,
                        width: w as usize,
                        height: h as usize,
                    },
                );
            }
            RenderCommand::Draw {
                vertices,
                instances,
            } => {
                let v_count = vertices.end.saturating_sub(vertices.start) as usize;
                let i_count = instances.end.saturating_sub(instances.start) as usize;
                if v_count > 0 && i_count > 0 {
                    f_draw(
                        enc,
                        s.draw_primitives,
                        current_primitive_type,
                        vertices.start as usize,
                        v_count,
                        i_count,
                        instances.start as usize,
                    );
                }
            }
            RenderCommand::DrawIndexed {
                indices,
                base_vertex,
                instances,
            } => {
                let idx_count = indices.end.saturating_sub(indices.start) as usize;
                let i_count = instances.end.saturating_sub(instances.start) as usize;
                if idx_count > 0 && i_count > 0 {
                    if let Some((ibuf, fmt, base_off)) = &active_index_buffer {
                        let elem_size = match fmt {
                            IndexFormat::Uint16 => 2u64,
                            IndexFormat::Uint32 => 4u64,
                        };
                        let mtl_idx_type = match fmt {
                            IndexFormat::Uint16 => 0u64, // MTLIndexTypeUInt16
                            IndexFormat::Uint32 => 1u64, // MTLIndexTypeUInt32
                        };
                        let byte_off = (*base_off + indices.start as u64 * elem_size) as usize;
                        f_draw_idx(
                            enc,
                            s.draw_indexed_primitives,
                            current_primitive_type,
                            idx_count,
                            mtl_idx_type,
                            ibuf.inner.raw.as_ptr(),
                            byte_off,
                            i_count,
                            base_vertex as isize,
                            instances.start as usize,
                        );
                    }
                }
            }
        }
    }

    f_void(enc, s.end_encoding);
}

// ============================================================================
// Resources, Descriptors, Buffers, Textures, Pipelines, Passes
// ============================================================================

#[derive(Debug, Clone)]
pub struct BufferDescriptor<'a> {
    pub label: Label<'a>,
    pub size: BufferAddress,
    pub usage: BufferUsages,
    pub mapped_at_creation: bool,
}

pub(crate) struct BufferInner {
    pub(crate) device: MetalDevice,
    pub(crate) raw: Arc<ObjCObject>,
    pub(crate) ptr: *mut u8,
    pub(crate) size: BufferAddress,
    pub(crate) usage: BufferUsages,
    pub(crate) options: u64,
    pub(crate) mapped_ready: AtomicBool,
}

unsafe impl Send for BufferInner {}
unsafe impl Sync for BufferInner {}

impl Drop for BufferInner {
    fn drop(&mut self) {
        if self.options != 0xFF {
            self.device
                .recycle_buffer(self.size as usize, self.options, self.raw.clone(), self.ptr);
        }
    }
}

#[derive(Clone)]
pub struct Buffer {
    pub(crate) inner: Arc<BufferInner>,
}

impl PartialEq for Buffer {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Buffer {}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer")
            .field("size", &self.inner.size)
            .field("usage", &self.inner.usage)
            .finish()
    }
}

impl Buffer {
    #[inline]
    pub fn size(&self) -> BufferAddress {
        self.inner.size
    }

    #[inline]
    pub fn usage(&self) -> BufferUsages {
        self.inner.usage
    }

    /// Zero-copy UMA direct slice view into unified memory without map_async overhead
    #[inline]
    pub fn uma_slice(&self) -> &[u8] {
        if !self.inner.device.is_last_cmd_completed() {
            self.inner.device.wait_idle();
        }
        unsafe { std::slice::from_raw_parts(self.inner.ptr, self.inner.size as usize) }
    }

    /// Zero-copy UMA direct mutable slice into unified memory
    #[inline]
    pub fn uma_slice_mut(&mut self) -> &mut [u8] {
        if !self.inner.device.is_last_cmd_completed() {
            self.inner.device.wait_idle();
        }
        unsafe { std::slice::from_raw_parts_mut(self.inner.ptr, self.inner.size as usize) }
    }

    #[inline]
    pub fn slice<S: RangeBounds<BufferAddress>>(&self, bounds: S) -> BufferSlice<'_> {
        let start = match bounds.start_bound() {
            Bound::Included(&v) => v,
            Bound::Excluded(&v) => v + 1,
            Bound::Unbounded => 0,
        };
        let end = match bounds.end_bound() {
            Bound::Included(&v) => v + 1,
            Bound::Excluded(&v) => v,
            Bound::Unbounded => self.inner.size,
        };
        BufferSlice {
            buffer: self,
            offset: start,
            size: end.saturating_sub(start),
        }
    }

    #[inline]
    pub fn map_async<S, F>(&self, mode: MapMode, bounds: S, callback: F)
    where
        S: RangeBounds<BufferAddress>,
        F: FnOnce(Result<(), BufferAsyncError>) + Send + 'static,
    {
        self.slice(bounds).map_async(mode, callback);
    }

    #[inline]
    pub fn get_mapped_range<S: RangeBounds<BufferAddress>>(
        &self,
        bounds: S,
    ) -> Result<BufferView<'_>, BufferAccessError> {
        self.slice(bounds).get_mapped_range()
    }

    #[inline]
    pub fn get_mapped_range_mut<S: RangeBounds<BufferAddress>>(
        &self,
        bounds: S,
    ) -> Result<WriteOnly<'_, [u8]>, BufferAccessError> {
        self.slice(bounds).get_mapped_range_mut()
    }

    #[inline]
    pub fn as_entire_binding(&self) -> BindingResource<'_> {
        BindingResource::Buffer(BufferBinding {
            buffer: self,
            offset: 0,
            size: None,
        })
    }

    #[inline]
    pub fn as_entire_buffer_binding(&self) -> BufferBinding<'_> {
        BufferBinding {
            buffer: self,
            offset: 0,
            size: None,
        }
    }

    #[inline]
    pub fn unmap(&self) {
        self.inner.mapped_ready.store(false, Ordering::Release);
    }

    #[inline]
    pub fn destroy(&self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapMode {
    Read,
    Write,
}

#[derive(Debug, Clone)]
pub struct BufferAsyncError;
impl std::fmt::Display for BufferAsyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BufferAsyncError")
    }
}
impl std::error::Error for BufferAsyncError {}

#[derive(Debug, Clone)]
pub struct BufferAccessError(pub String);
impl std::fmt::Display for BufferAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for BufferAccessError {}

#[derive(Clone, Copy)]
pub struct BufferSlice<'a> {
    pub buffer: &'a Buffer,
    pub offset: BufferAddress,
    pub size: BufferAddress,
}

impl<'a> BufferSlice<'a> {
    #[inline]
    pub fn map_async<F>(&self, _mode: MapMode, callback: F)
    where
        F: FnOnce(Result<(), BufferAsyncError>) + Send + 'static,
    {
        // On Apple Silicon UMA, shared memory is already mapped; ensure GPU work is complete before reading
        if !self.buffer.inner.device.is_last_cmd_completed() {
            self.buffer.inner.device.wait_idle();
        }
        self.buffer.inner.mapped_ready.store(true, Ordering::Release);
        callback(Ok(()));
    }

    #[inline]
    pub fn get_mapped_range(&self) -> Result<BufferView<'a>, BufferAccessError> {
        if !self.buffer.inner.device.is_last_cmd_completed() {
            self.buffer.inner.device.wait_idle();
        }
        let slice = unsafe {
            std::slice::from_raw_parts(
                self.buffer.inner.ptr.add(self.offset as usize),
                self.size as usize,
            )
        };
        Ok(BufferView { slice })
    }

    #[inline]
    pub fn get_mapped_range_mut(&self) -> Result<WriteOnly<'a, [u8]>, BufferAccessError> {
        if !self.buffer.inner.device.is_last_cmd_completed() {
            self.buffer.inner.device.wait_idle();
        }
        let slice = unsafe {
            std::slice::from_raw_parts_mut(
                self.buffer.inner.ptr.add(self.offset as usize),
                self.size as usize,
            )
        };
        Ok(WriteOnly::new(slice))
    }
}

pub struct BufferView<'a> {
    slice: &'a [u8],
}

impl<'a> Deref for BufferView<'a> {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl<'a> AsRef<[u8]> for BufferView<'a> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.slice
    }
}

#[derive(Debug, Clone)]
pub enum ShaderSource<'a> {
    Wgsl(Cow<'a, str>),
}

#[derive(Debug, Clone)]
pub struct ShaderModuleDescriptor<'a> {
    pub label: Label<'a>,
    pub source: ShaderSource<'a>,
}

#[derive(Clone)]
pub struct ShaderModule {
    pub(crate) module: Arc<naga::Module>,
    pub(crate) info: Arc<naga::valid::ModuleInfo>,
    pub(crate) msl_source: Arc<String>,
    pub(crate) bindings: Arc<BindingMap>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferBindingType {
    Uniform,
    Storage { read_only: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerBindingType {
    Filtering,
    NonFiltering,
    Comparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureSampleType {
    Float { filterable: bool },
    Depth,
    Sint,
    Uint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureViewDimension {
    D1,
    #[default]
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageTextureAccess {
    WriteOnly,
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingType {
    Buffer {
        ty: BufferBindingType,
        has_dynamic_offset: bool,
        min_binding_size: Option<BufferSize>,
    },
    Sampler(SamplerBindingType),
    Texture {
        sample_type: TextureSampleType,
        view_dimension: TextureViewDimension,
        multisampled: bool,
    },
    StorageTexture {
        access: StorageTextureAccess,
        format: TextureFormat,
        view_dimension: TextureViewDimension,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct BindGroupLayoutEntry {
    pub binding: u32,
    pub visibility: ShaderStages,
    pub ty: BindingType,
    pub count: Option<NonZeroU32>,
}

#[derive(Debug, Clone)]
pub struct BindGroupLayoutDescriptor<'a> {
    pub label: Label<'a>,
    pub entries: &'a [BindGroupLayoutEntry],
}

#[derive(Clone, Debug)]
pub struct BindGroupLayout {
    pub(crate) entries: Vec<BindGroupLayoutEntry>,
}

#[derive(Clone)]
pub struct BufferBinding<'a> {
    pub buffer: &'a Buffer,
    pub offset: BufferAddress,
    pub size: Option<BufferSize>,
}

#[derive(Clone)]
pub enum BindingResource<'a> {
    Buffer(BufferBinding<'a>),
    Sampler(&'a Sampler),
    TextureView(&'a TextureView),
}

#[derive(Clone)]
pub struct BindGroupEntry<'a> {
    pub binding: u32,
    pub resource: BindingResource<'a>,
}

#[derive(Clone)]
pub struct BindGroupDescriptor<'a> {
    pub label: Label<'a>,
    pub layout: &'a BindGroupLayout,
    pub entries: &'a [BindGroupEntry<'a>],
}

#[derive(Clone)]
pub struct BindGroup {
    pub(crate) buffers: Vec<(u32, Buffer, u64, u64)>,
    pub(crate) textures: Vec<(u32, Texture)>,
    pub(crate) samplers: Vec<(u32, Sampler)>,
}

#[derive(Debug, Clone)]
pub struct PipelineLayoutDescriptor<'a> {
    pub label: Label<'a>,
    pub bind_group_layouts: &'a [Option<&'a BindGroupLayout>],
    pub immediate_size: u32,
}

impl Default for PipelineLayoutDescriptor<'_> {
    fn default() -> Self {
        Self {
            label: None,
            bind_group_layouts: &[],
            immediate_size: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PipelineLayout {
    pub(crate) immediate_size: u32,
}

#[derive(Debug, Clone, Default)]
pub struct PipelineCompilationOptions<'a> {
    pub constants: &'a [(&'a str, f64)],
    pub zero_initialize_workgroup_memory: bool,
}

#[derive(Clone)]
pub struct ComputePipelineDescriptor<'a> {
    pub label: Label<'a>,
    pub layout: Option<&'a PipelineLayout>,
    pub module: &'a ShaderModule,
    pub entry_point: Option<&'a str>,
    pub compilation_options: PipelineCompilationOptions<'a>,
    pub cache: Option<&'a ()>,
}

#[derive(Clone)]
pub struct ComputePipeline {
    pub(crate) pso: Arc<ObjCObject>,
    pub(crate) workgroup_size: [u32; 3],
    pub(crate) bindings: Arc<BindingMap>,
}

impl ComputePipeline {
    pub fn get_bind_group_layout(&self, _index: u32) -> BindGroupLayout {
        BindGroupLayout {
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VertexStepMode {
    #[default]
    Vertex,
    Instance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexFormat {
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
    Uint32,
    Uint32x2,
    Uint32x3,
    Uint32x4,
    Sint32,
    Sint32x2,
    Sint32x3,
    Sint32x4,
}

impl VertexFormat {
    pub(crate) const fn to_mtl(self) -> u64 {
        match self {
            Self::Float32 => 28,   // MTLVertexFormatFloat
            Self::Float32x2 => 29, // MTLVertexFormatFloat2
            Self::Float32x3 => 30, // MTLVertexFormatFloat3
            Self::Float32x4 => 31, // MTLVertexFormatFloat4
            Self::Sint32 => 32,
            Self::Sint32x2 => 33,
            Self::Sint32x3 => 34,
            Self::Sint32x4 => 35,
            Self::Uint32 => 36,
            Self::Uint32x2 => 37,
            Self::Uint32x3 => 38,
            Self::Uint32x4 => 39,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VertexAttribute {
    pub format: VertexFormat,
    pub offset: BufferAddress,
    pub shader_location: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct VertexBufferLayout<'a> {
    pub array_stride: BufferAddress,
    pub step_mode: VertexStepMode,
    pub attributes: &'a [VertexAttribute],
}

#[derive(Clone)]
pub struct VertexState<'a> {
    pub module: &'a ShaderModule,
    pub entry_point: Option<&'a str>,
    pub compilation_options: PipelineCompilationOptions<'a>,
    pub buffers: &'a [Option<VertexBufferLayout<'a>>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendFactor {
    #[default]
    Zero,
    One,
    Src,
    OneMinusSrc,
    SrcAlpha,
    OneMinusSrcAlpha,
    Dst,
    OneMinusDst,
    DstAlpha,
    OneMinusDstAlpha,
}

impl BlendFactor {
    pub(crate) const fn to_mtl(self) -> u64 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::Src => 2,
            Self::OneMinusSrc => 3,
            Self::SrcAlpha => 4,
            Self::OneMinusSrcAlpha => 5,
            Self::Dst => 6,
            Self::OneMinusDst => 7,
            Self::DstAlpha => 8,
            Self::OneMinusDstAlpha => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendOperation {
    #[default]
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

impl BlendOperation {
    pub(crate) const fn to_mtl(self) -> u64 {
        match self {
            Self::Add => 0,
            Self::Subtract => 1,
            Self::ReverseSubtract => 2,
            Self::Min => 3,
            Self::Max => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlendComponent {
    pub src_factor: BlendFactor,
    pub dst_factor: BlendFactor,
    pub operation: BlendOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlendState {
    pub color: BlendComponent,
    pub alpha: BlendComponent,
}

impl BlendState {
    pub const ALPHA_BLENDING: Self = Self {
        color: BlendComponent {
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOperation::Add,
        },
    };
}

#[derive(Debug, Clone)]
pub struct ColorTargetState {
    pub format: TextureFormat,
    pub blend: Option<BlendState>,
    pub write_mask: ColorWrites,
}

#[derive(Clone)]
pub struct FragmentState<'a> {
    pub module: &'a ShaderModule,
    pub entry_point: Option<&'a str>,
    pub compilation_options: PipelineCompilationOptions<'a>,
    pub targets: &'a [Option<ColorTargetState>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    #[default]
    TriangleList,
    TriangleStrip,
}

impl PrimitiveTopology {
    pub(crate) const fn to_mtl(self) -> u64 {
        match self {
            Self::PointList => 0,
            Self::LineList => 1,
            Self::LineStrip => 2,
            Self::TriangleList => 3,
            Self::TriangleStrip => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IndexFormat {
    Uint16,
    #[default]
    Uint32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FrontFace {
    #[default]
    Ccw,
    Cw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Front,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PolygonMode {
    #[default]
    Fill,
    Line,
    Point,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PrimitiveState {
    pub topology: PrimitiveTopology,
    pub strip_index_format: Option<IndexFormat>,
    pub front_face: FrontFace,
    pub cull_mode: Option<Face>,
    pub unclipped_depth: bool,
    pub polygon_mode: PolygonMode,
    pub conservative: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompareFunction {
    Never = 1,
    Less = 2,
    Equal = 3,
    #[default]
    LessEqual = 4,
    Greater = 5,
    NotEqual = 6,
    GreaterEqual = 7,
    Always = 8,
}

impl CompareFunction {
    pub(crate) const fn to_mtl(self) -> u64 {
        match self {
            Self::Never => 0,
            Self::Less => 1,
            Self::Equal => 2,
            Self::LessEqual => 3,
            Self::Greater => 4,
            Self::NotEqual => 5,
            Self::GreaterEqual => 6,
            Self::Always => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StencilState;

#[derive(Debug, Clone, Copy, Default)]
pub struct DepthBiasState {
    pub constant: i32,
    pub slope_scale: f32,
    pub clamp: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct DepthStencilState {
    pub format: TextureFormat,
    pub depth_write_enabled: Option<bool>,
    pub depth_compare: Option<CompareFunction>,
    pub stencil: StencilState,
    pub bias: DepthBiasState,
}

#[derive(Debug, Clone, Copy)]
pub struct MultisampleState {
    pub count: u32,
    pub mask: u64,
    pub alpha_to_coverage_enabled: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }
}

#[derive(Clone)]
pub struct RenderPipelineDescriptor<'a> {
    pub label: Label<'a>,
    pub layout: Option<&'a PipelineLayout>,
    pub vertex: VertexState<'a>,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub multisample: MultisampleState,
    pub fragment: Option<FragmentState<'a>>,
    pub multiview_mask: Option<NonZeroU32>,
    pub cache: Option<&'a ()>,
}

#[derive(Clone)]
pub struct RenderPipeline {
    pub(crate) pso: Arc<ObjCObject>,
    pub(crate) depth_stencil_state: Option<Arc<ObjCObject>>,
    pub(crate) primitive: PrimitiveState,
    pub(crate) depth_bias: DepthBiasState,
}

impl RenderPipeline {
    pub fn get_bind_group_layout(&self, _index: u32) -> BindGroupLayout {
        BindGroupLayout {
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureDimension {
    D1,
    #[default]
    D2,
    D3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    R8Unorm,
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Bgra8Unorm,
    Bgra8UnormSrgb,
    R32Float,
    Rg32Float,
    Rgba16Float,
    Rgba32Float,
    Depth24Plus,
    Depth32Float,
}

impl TextureFormat {
    pub(crate) const fn to_mtl_pixel_format(self) -> u64 {
        match self {
            Self::R8Unorm => 10,          // MTLPixelFormatR8Unorm
            Self::Rgba8Unorm => 70,       // MTLPixelFormatRGBA8Unorm
            Self::Rgba8UnormSrgb => 71,   // MTLPixelFormatRGBA8Unorm_sRGB
            Self::Bgra8Unorm => 80,       // MTLPixelFormatBGRA8Unorm
            Self::Bgra8UnormSrgb => 81,   // MTLPixelFormatBGRA8Unorm_sRGB
            Self::R32Float => 55,         // MTLPixelFormatR32Float
            Self::Rg32Float => 105,       // MTLPixelFormatRG32Float
            Self::Rgba16Float => 115,     // MTLPixelFormatRGBA16Float
            Self::Rgba32Float => 125,     // MTLPixelFormatRGBA32Float
            Self::Depth24Plus => 252,     // MTLPixelFormatDepth32Float
            Self::Depth32Float => 252,    // MTLPixelFormatDepth32Float
        }
    }

    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::R8Unorm => 1,
            Self::Rgba8Unorm
            | Self::Rgba8UnormSrgb
            | Self::Bgra8Unorm
            | Self::Bgra8UnormSrgb
            | Self::R32Float
            | Self::Depth24Plus
            | Self::Depth32Float => 4,
            Self::Rg32Float | Self::Rgba16Float => 8,
            Self::Rgba32Float => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent3d {
    pub width: u32,
    pub height: u32,
    pub depth_or_array_layers: u32,
}

impl Default for Extent3d {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Origin3d {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Origin3d {
    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureAspect {
    #[default]
    All,
    StencilOnly,
    DepthOnly,
}

#[derive(Debug, Clone)]
pub struct TextureDescriptor<'a> {
    pub label: Label<'a>,
    pub size: Extent3d,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: TextureDimension,
    pub format: TextureFormat,
    pub usage: TextureUsages,
    pub view_formats: &'a [TextureFormat],
}

#[derive(Debug, Clone)]
pub struct Texture {
    pub(crate) raw: Arc<ObjCObject>,
    pub(crate) size: Extent3d,
    pub(crate) mip_level_count: u32,
    pub(crate) sample_count: u32,
    pub(crate) dimension: TextureDimension,
    pub(crate) format: TextureFormat,
    pub(crate) usage: TextureUsages,
    pub(crate) storage_mode: u64,
}

impl Texture {
    pub fn create_view(&self, desc: &TextureViewDescriptor<'_>) -> TextureView {
        TextureView {
            texture: self.clone(),
            base_mip_level: desc.base_mip_level,
            base_array_layer: desc.base_array_layer,
        }
    }

    pub fn size(&self) -> Extent3d {
        self.size
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn mip_level_count(&self) -> u32 {
        self.mip_level_count
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    pub fn dimension(&self) -> TextureDimension {
        self.dimension
    }

    pub fn usage(&self) -> TextureUsages {
        self.usage
    }

    pub fn destroy(&self) {}
}

#[derive(Debug, Clone, Default)]
pub struct TextureViewDescriptor<'a> {
    pub label: Label<'a>,
    pub format: Option<TextureFormat>,
    pub dimension: Option<TextureViewDimension>,
    pub usage: Option<TextureUsages>,
    pub aspect: TextureAspect,
    pub base_mip_level: u32,
    pub mip_level_count: Option<u32>,
    pub base_array_layer: u32,
    pub array_layer_count: Option<u32>,
}

#[derive(Clone)]
pub struct TextureView {
    pub(crate) texture: Texture,
    pub(crate) base_mip_level: u32,
    pub(crate) base_array_layer: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddressMode {
    #[default]
    ClampToEdge,
    Repeat,
    MirrorRepeat,
    ClampToBorder,
}

impl AddressMode {
    pub(crate) const fn to_mtl(self) -> u64 {
        match self {
            Self::ClampToEdge => 0,
            Self::MirrorRepeat => 3,
            Self::Repeat => 2,
            Self::ClampToBorder => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    #[default]
    Nearest,
    Linear,
}

impl FilterMode {
    pub(crate) const fn to_mtl(self) -> u64 {
        match self {
            Self::Nearest => 0,
            Self::Linear => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MipmapFilterMode {
    #[default]
    Nearest,
    Linear,
}

#[derive(Debug, Clone)]
pub struct SamplerDescriptor<'a> {
    pub label: Label<'a>,
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mipmap_filter: MipmapFilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<CompareFunction>,
    pub anisotropy_clamp: u16,
    pub border_color: Option<()>,
}

impl Default for SamplerDescriptor<'_> {
    fn default() -> Self {
        Self {
            label: None,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        }
    }
}

#[derive(Clone)]
pub struct Sampler {
    pub(crate) raw: Arc<ObjCObject>,
}

#[derive(Debug, Clone, Copy)]
pub struct TexelCopyTextureInfo<'a> {
    pub texture: &'a Texture,
    pub mip_level: u32,
    pub origin: Origin3d,
    pub aspect: TextureAspect,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TexelCopyBufferLayout {
    pub offset: BufferAddress,
    pub bytes_per_row: Option<u32>,
    pub rows_per_image: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct TexelCopyBufferInfo<'a> {
    pub buffer: &'a Buffer,
    pub layout: TexelCopyBufferLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    Occlusion,
    Timestamp,
}

#[derive(Debug, Clone)]
pub struct QuerySetDescriptor<'a> {
    pub label: Label<'a>,
    pub ty: QueryType,
    pub count: u32,
}

#[derive(Clone)]
pub struct QuerySet {
    pub(crate) count: u32,
    pub(crate) timestamps_ns: Arc<Mutex<Vec<u64>>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
}

#[derive(Debug, Clone, Copy)]
pub enum LoadOp<V> {
    Clear(V),
    Load,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreOp {
    Store,
    Discard,
}

#[derive(Debug, Clone, Copy)]
pub struct Operations<V> {
    pub load: LoadOp<V>,
    pub store: StoreOp,
}

#[derive(Clone)]
pub struct RenderPassColorAttachment<'a> {
    pub view: &'a TextureView,
    pub resolve_target: Option<&'a TextureView>,
    pub ops: Operations<Color>,
    pub depth_slice: Option<u32>,
}

#[derive(Clone)]
pub struct RenderPassDepthStencilAttachment<'a> {
    pub view: &'a TextureView,
    pub depth_ops: Option<Operations<f32>>,
    pub stencil_ops: Option<Operations<u32>>,
}

#[derive(Clone)]
pub struct RenderPassTimestampWrites<'a> {
    pub query_set: &'a QuerySet,
    pub beginning_of_pass_write_index: Option<u32>,
    pub end_of_pass_write_index: Option<u32>,
}

#[derive(Clone, Default)]
pub struct RenderPassDescriptor<'a> {
    pub label: Label<'a>,
    pub color_attachments: &'a [Option<RenderPassColorAttachment<'a>>],
    pub depth_stencil_attachment: Option<RenderPassDepthStencilAttachment<'a>>,
    pub timestamp_writes: Option<RenderPassTimestampWrites<'a>>,
    pub occlusion_query_set: Option<&'a QuerySet>,
    pub multiview_mask: Option<NonZeroU32>,
}

#[derive(Clone)]
pub struct ComputePassTimestampWrites<'a> {
    pub query_set: &'a QuerySet,
    pub beginning_of_pass_write_index: Option<u32>,
    pub end_of_pass_write_index: Option<u32>,
}

#[derive(Clone, Default)]
pub struct ComputePassDescriptor<'a> {
    pub label: Label<'a>,
    pub timestamp_writes: Option<ComputePassTimestampWrites<'a>>,
}

#[derive(Debug, Clone, Default)]
pub struct CommandEncoderDescriptor<'a> {
    pub label: Label<'a>,
}

pub(crate) struct ComputeDispatchOp {
    pub(crate) pipeline: ComputePipeline,
    pub(crate) bind_groups: Vec<(u32, Option<BindGroup>, Vec<u32>)>,
    pub(crate) immediates: Vec<u8>,
    pub(crate) groups: [u32; 3],
}

#[derive(Clone)]
pub(crate) struct OwnedColorAttachment {
    pub(crate) view: TextureView,
    pub(crate) ops: Operations<Color>,
}

#[derive(Clone)]
pub(crate) struct OwnedDepthAttachment {
    pub(crate) view: TextureView,
    pub(crate) depth_ops: Option<Operations<f32>>,
}

pub(crate) enum RenderCommand {
    SetPipeline(RenderPipeline),
    SetBindGroup {
        index: u32,
        bind_group: BindGroup,
        offsets: Vec<u32>,
    },
    SetVertexBuffer {
        slot: u32,
        buffer: Buffer,
        offset: u64,
    },
    SetIndexBuffer {
        buffer: Buffer,
        format: IndexFormat,
        offset: u64,
    },
    SetImmediates {
        offset: u32,
        data: Vec<u8>,
    },
    SetViewport {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        min_depth: f32,
        max_depth: f32,
    },
    SetScissorRect {
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    },
    Draw {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
    DrawIndexed {
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    },
}

pub(crate) struct RenderPassOp {
    pub(crate) color_attachments: Vec<Option<OwnedColorAttachment>>,
    pub(crate) depth_stencil_attachment: Option<OwnedDepthAttachment>,
    pub(crate) commands: Vec<RenderCommand>,
}

pub(crate) enum RecordedOp {
    ComputePass {
        dispatches: Vec<ComputeDispatchOp>,
        start_ts: Option<(QuerySet, u32)>,
        end_ts: Option<(QuerySet, u32)>,
    },
    RenderPass(RenderPassOp),
    CopyBufferToBuffer {
        src: Buffer,
        src_offset: u64,
        dst: Buffer,
        dst_offset: u64,
        size: u64,
    },
    ClearBuffer {
        dst: Buffer,
        offset: u64,
        size: u64,
    },
    CopyTextureToBuffer {
        src_tex: Texture,
        mip_level: u32,
        origin: Origin3d,
        dst_buf: Buffer,
        dst_offset: u64,
        bytes_per_row: u32,
        rows_per_image: Option<u32>,
        size: Extent3d,
    },
    CopyTextureToTexture {
        src_tex: Texture,
        src_mip: u32,
        src_origin: Origin3d,
        dst_tex: Texture,
        dst_mip: u32,
        dst_origin: Origin3d,
        size: Extent3d,
    },
    WriteTimestamp {
        query_set: QuerySet,
        index: u32,
    },
    ResolveQuerySet {
        query_set: QuerySet,
        first_query: u32,
        query_count: u32,
        destination: Buffer,
        destination_offset: u64,
    },
}

pub struct CommandEncoder {
    pub(crate) device: MetalDevice,
    pub(crate) error_stack: Arc<Mutex<Vec<(ErrorFilter, Option<Error>)>>>,
    pub(crate) ops: Vec<RecordedOp>,
    pub(crate) on_submit_callbacks: Mutex<Vec<Box<dyn FnOnce() + Send>>>,
}

impl CommandEncoder {
    pub(crate) fn record_error(&self, err: Error) {
        let err_filter = match &err {
            Error::OutOfMemory { .. } => ErrorFilter::OutOfMemory,
            Error::Validation { .. } => ErrorFilter::Validation,
            Error::Internal { .. } => ErrorFilter::Internal,
        };
        if let Ok(mut stack) = self.error_stack.lock() {
            if let Some((_, slot)) = stack.iter_mut().rev().find(|(f, _)| *f == err_filter) {
                if slot.is_none() {
                    *slot = Some(err);
                }
            }
        }
    }

    pub fn map_buffer_on_submit<S, F>(
        &self,
        buffer: &Buffer,
        _mode: MapMode,
        _bounds: S,
        callback: F,
    ) where
        S: RangeBounds<BufferAddress>,
        F: FnOnce(Result<(), BufferAsyncError>) + Send + 'static,
    {
        let buf = buffer.clone();
        if let Ok(mut cbs) = self.on_submit_callbacks.lock() {
            cbs.push(Box::new(move || {
                buf.inner.mapped_ready.store(true, Ordering::Release);
                callback(Ok(()));
            }));
        }
    }

    pub fn begin_compute_pass(&mut self, desc: &ComputePassDescriptor<'_>) -> ComputePass<'_> {
        let start_ts = desc.timestamp_writes.as_ref().and_then(|tw| {
            tw.beginning_of_pass_write_index
                .map(|i| (tw.query_set.clone(), i))
        });
        let end_ts = desc
            .timestamp_writes
            .as_ref()
            .and_then(|tw| tw.end_of_pass_write_index.map(|i| (tw.query_set.clone(), i)));
        ComputePass {
            encoder: self,
            current_pipeline: None,
            bind_groups: Vec::with_capacity(4),
            immediates: Vec::new(),
            dispatches: Vec::with_capacity(8),
            start_timestamp: start_ts,
            end_timestamp: end_ts,
        }
    }

    pub fn begin_render_pass(&mut self, desc: &RenderPassDescriptor<'_>) -> RenderPass<'_> {
        let color_attachments = desc
            .color_attachments
            .iter()
            .map(|ca| {
                ca.as_ref().map(|c| OwnedColorAttachment {
                    view: c.view.clone(),
                    ops: c.ops,
                })
            })
            .collect();
        let depth_stencil_attachment =
            desc.depth_stencil_attachment
                .as_ref()
                .map(|d| OwnedDepthAttachment {
                    view: d.view.clone(),
                    depth_ops: d.depth_ops,
                });
        RenderPass {
            encoder: self,
            color_attachments,
            depth_stencil_attachment,
            commands: Vec::with_capacity(32),
        }
    }

    pub fn copy_buffer_to_buffer(
        &mut self,
        source: &Buffer,
        source_offset: BufferAddress,
        destination: &Buffer,
        destination_offset: BufferAddress,
        copy_size: BufferAddress,
    ) {
        self.ops.push(RecordedOp::CopyBufferToBuffer {
            src: source.clone(),
            src_offset: source_offset,
            dst: destination.clone(),
            dst_offset: destination_offset,
            size: copy_size,
        });
    }

    pub fn clear_buffer(
        &mut self,
        buffer: &Buffer,
        offset: BufferAddress,
        size: Option<BufferAddress>,
    ) {
        let sz = size.unwrap_or_else(|| buffer.size().saturating_sub(offset));
        self.ops.push(RecordedOp::ClearBuffer {
            dst: buffer.clone(),
            offset,
            size: sz,
        });
    }

    pub fn copy_texture_to_buffer(
        &mut self,
        source: TexelCopyTextureInfo<'_>,
        destination: TexelCopyBufferInfo<'_>,
        copy_size: Extent3d,
    ) {
        self.ops.push(RecordedOp::CopyTextureToBuffer {
            src_tex: source.texture.clone(),
            mip_level: source.mip_level,
            origin: source.origin,
            dst_buf: destination.buffer.clone(),
            dst_offset: destination.layout.offset,
            bytes_per_row: destination
                .layout
                .bytes_per_row
                .unwrap_or(copy_size.width * source.texture.format.bytes_per_pixel()),
            rows_per_image: destination.layout.rows_per_image,
            size: copy_size,
        });
    }

    pub fn copy_texture_to_texture(
        &mut self,
        source: TexelCopyTextureInfo<'_>,
        destination: TexelCopyTextureInfo<'_>,
        copy_size: Extent3d,
    ) {
        self.ops.push(RecordedOp::CopyTextureToTexture {
            src_tex: source.texture.clone(),
            src_mip: source.mip_level,
            src_origin: source.origin,
            dst_tex: destination.texture.clone(),
            dst_mip: destination.mip_level,
            dst_origin: destination.origin,
            size: copy_size,
        });
    }

    pub fn write_timestamp(&mut self, query_set: &QuerySet, query_index: u32) {
        self.ops.push(RecordedOp::WriteTimestamp {
            query_set: query_set.clone(),
            index: query_index,
        });
    }

    pub fn resolve_query_set(
        &mut self,
        query_set: &QuerySet,
        query_range: Range<u32>,
        destination: &Buffer,
        destination_offset: BufferAddress,
    ) {
        self.ops.push(RecordedOp::ResolveQuerySet {
            query_set: query_set.clone(),
            first_query: query_range.start,
            query_count: query_range.end.saturating_sub(query_range.start),
            destination: destination.clone(),
            destination_offset,
        });
    }

    pub fn finish(self) -> CommandBuffer {
        let on_submit_callbacks = self
            .on_submit_callbacks
            .into_inner()
            .unwrap_or_default();
        CommandBuffer {
            ops: self.ops,
            on_submit_callbacks,
        }
    }
}

pub struct CommandBuffer {
    pub(crate) ops: Vec<RecordedOp>,
    pub(crate) on_submit_callbacks: Vec<Box<dyn FnOnce() + Send>>,
}

pub struct ComputePass<'a> {
    encoder: &'a mut CommandEncoder,
    current_pipeline: Option<ComputePipeline>,
    bind_groups: Vec<(u32, Option<BindGroup>, Vec<u32>)>,
    immediates: Vec<u8>,
    dispatches: Vec<ComputeDispatchOp>,
    start_timestamp: Option<(QuerySet, u32)>,
    end_timestamp: Option<(QuerySet, u32)>,
}

impl<'a> ComputePass<'a> {
    #[inline]
    pub fn set_pipeline(&mut self, pipeline: &ComputePipeline) {
        self.current_pipeline = Some(pipeline.clone());
    }

    #[inline]
    pub fn set_bind_group<'b>(
        &mut self,
        index: u32,
        bind_group: impl Into<Option<&'b BindGroup>>,
        offsets: &[DynamicOffset],
    ) {
        let bind_group = bind_group.into();
        if let Some(entry) = self.bind_groups.iter_mut().find(|(i, _, _)| *i == index) {
            entry.1 = bind_group.cloned();
            entry.2.clear();
            entry.2.extend_from_slice(offsets);
        } else {
            self.bind_groups
                .push((index, bind_group.cloned(), offsets.to_vec()));
        }
    }

    #[inline]
    pub fn set_immediates(&mut self, offset: u32, data: &[u8]) {
        let end = offset as usize + data.len();
        if self.immediates.len() < end {
            self.immediates.resize(end, 0);
        }
        self.immediates[offset as usize..end].copy_from_slice(data);
    }

    #[inline]
    pub fn dispatch_workgroups(&mut self, x: u32, y: u32, z: u32) {
        if let Some(pipe) = &self.current_pipeline {
            self.dispatches.push(ComputeDispatchOp {
                pipeline: pipe.clone(),
                bind_groups: self.bind_groups.clone(),
                immediates: self.immediates.clone(),
                groups: [x, y, z],
            });
        } else {
            let msg = "dispatch_workgroups called without an active ComputePipeline".to_string();
            self.encoder.record_error(Error::Validation {
                source: Arc::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    msg.clone(),
                )),
                description: msg,
            });
        }
    }
}

impl<'a> Drop for ComputePass<'a> {
    fn drop(&mut self) {
        let dispatches = std::mem::take(&mut self.dispatches);
        let start_ts = self.start_timestamp.take();
        let end_ts = self.end_timestamp.take();
        if !dispatches.is_empty() || start_ts.is_some() || end_ts.is_some() {
            self.encoder.ops.push(RecordedOp::ComputePass {
                dispatches,
                start_ts,
                end_ts,
            });
        }
    }
}

pub struct RenderPass<'a> {
    encoder: &'a mut CommandEncoder,
    color_attachments: Vec<Option<OwnedColorAttachment>>,
    depth_stencil_attachment: Option<OwnedDepthAttachment>,
    commands: Vec<RenderCommand>,
}

impl<'a> RenderPass<'a> {
    #[inline]
    pub fn set_pipeline(&mut self, pipeline: &RenderPipeline) {
        self.commands
            .push(RenderCommand::SetPipeline(pipeline.clone()));
    }

    #[inline]
    pub fn set_bind_group<'b>(
        &mut self,
        index: u32,
        bind_group: impl Into<Option<&'b BindGroup>>,
        offsets: &[DynamicOffset],
    ) {
        if let Some(bg) = bind_group.into() {
            self.commands.push(RenderCommand::SetBindGroup {
                index,
                bind_group: bg.clone(),
                offsets: offsets.to_vec(),
            });
        }
    }

    #[inline]
    pub fn set_vertex_buffer(&mut self, slot: u32, buffer_slice: BufferSlice<'_>) {
        self.commands.push(RenderCommand::SetVertexBuffer {
            slot,
            buffer: buffer_slice.buffer.clone(),
            offset: buffer_slice.offset,
        });
    }

    #[inline]
    pub fn set_index_buffer(&mut self, buffer_slice: BufferSlice<'_>, index_format: IndexFormat) {
        self.commands.push(RenderCommand::SetIndexBuffer {
            buffer: buffer_slice.buffer.clone(),
            format: index_format,
            offset: buffer_slice.offset,
        });
    }

    #[inline]
    pub fn set_immediates(&mut self, offset: u32, data: &[u8]) {
        self.commands.push(RenderCommand::SetImmediates {
            offset,
            data: data.to_vec(),
        });
    }

    #[inline]
    pub fn set_viewport(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        min_depth: f32,
        max_depth: f32,
    ) {
        self.commands.push(RenderCommand::SetViewport {
            x,
            y,
            w,
            h,
            min_depth,
            max_depth,
        });
    }

    #[inline]
    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) {
        self.commands.push(RenderCommand::SetScissorRect {
            x,
            y,
            w: width,
            h: height,
        });
    }

    #[inline]
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.commands.push(RenderCommand::Draw {
            vertices,
            instances,
        });
    }

    #[inline]
    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.commands.push(RenderCommand::DrawIndexed {
            indices,
            base_vertex,
            instances,
        });
    }
}

impl<'a> Drop for RenderPass<'a> {
    fn drop(&mut self) {
        let color_attachments = std::mem::take(&mut self.color_attachments);
        let depth_stencil_attachment = self.depth_stencil_attachment.take();
        let commands = std::mem::take(&mut self.commands);
        self.encoder.ops.push(RecordedOp::RenderPass(RenderPassOp {
            color_attachments,
            depth_stencil_attachment,
            commands,
        }));
    }
}
