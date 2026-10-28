//! GPU Adapter, Device, Limits, Error Scope, Poll, and Descriptor types.

use crate::{Backends, Features, InstanceFlags, Label};
use std::future::Future;
use std::sync::{Arc, Mutex};

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
    pub(crate) stack: Arc<Mutex<Vec<(ErrorFilter, Option<Error>)>>>,
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
