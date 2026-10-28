#![allow(unsafe_code, unsafe_op_in_unsafe_fn, unused_variables, dead_code)]

//! Modular, zero-overhead cross-platform GPU abstraction layer (`wgpu` drop-in replacement).
//!
//! Organized according to **SOLID**, **DRY**, and **Divide & Conquer** principles:
//! - [`platform`]: Target OS detection (`Windows`, `MacOs`, `Linux`, `Ios`, `Android`, `BrowserWebGpu`),
//!   memory architectures (`Uma`, `Duma`, `Asd`), and 5-level `HardwareExecutionTopology`.
//! - [`flags`]: Bitflags (`Backends`, `Features`, `BufferUsages`, `TextureUsages`, `ShaderStages`, `ColorWrites`).
//! - [`types`]: Core descriptors, limits, adapter info, error scopes, and poll primitives.
//! - [`buffer`]: Zero-copy UMA/ASD `Buffer`, `BufferSlice`, `BufferView`, and `WriteOnly`.
//! - [`texture`]: `Texture`, `TextureView`, `Sampler`, `QuerySet`, and texel copy descriptors.
//! - [`pipeline`]: `ShaderModule`, `BindGroup`, `ComputePipeline`, `RenderPipeline`, and state descriptors.
//! - [`command`]: `CommandEncoder`, `CommandBuffer`, `ComputePass`, `RenderPass`, and Metal encoder passes.
//! - [`metal`], [`vulkan`], [`dx12`], [`cuda`], [`webgpu`]: Hardware backend runtimes.

pub mod buffer;
pub mod command;
pub mod cuda;
pub mod dx12;
pub mod flags;
pub mod metal;
pub mod opengl;
pub mod pipeline;
pub mod platform;
pub mod rtx;
pub mod texture;
pub mod types;
pub mod util;
pub mod vulkan;
pub mod webgpu;

pub use buffer::*;
pub use command::*;
pub use flags::*;
pub use opengl::*;
pub use pipeline::*;
pub use platform::*;
pub use rtx::*;
pub use texture::*;
pub use types::*;

use metal::{
    metal_msg_send, metal_selectors, nsstring_from_str, MTLOrigin, MTLRegion, MTLSize, MetalDevice,
    ObjCObject,
};
use naga::back::msl::{BindingMap, MSL_VERTEX_BUFFER_BASE_SLOT};
use std::ffi::c_void;
use std::future::Future;
use std::num::NonZeroU64;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub type BufferAddress = u64;
pub type BufferSize = NonZeroU64;
pub type DynamicOffset = u32;
pub type Label<'a> = Option<&'a str>;

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
            | Features::SUBGROUP
            | Features::EXPERIMENTAL_RAY_QUERY
            | Features::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE
            | Features::RAY_TRACING_PIPELINE;
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
        let missing_ray_query = desc
            .usage
            .intersects(BufferUsages::BLAS_INPUT | BufferUsages::TLAS_INPUT)
            && !self.features.contains(Features::EXPERIMENTAL_RAY_QUERY);
        let missing_mappable_primary = !self.features.contains(Features::MAPPABLE_PRIMARY_BUFFERS)
            && ((desc.usage.contains(BufferUsages::MAP_READ)
                && !(desc.usage - BufferUsages::MAP_READ - BufferUsages::COPY_DST).is_empty())
                || (desc.usage.contains(BufferUsages::MAP_WRITE)
                    && !(desc.usage - BufferUsages::MAP_WRITE - BufferUsages::COPY_SRC).is_empty()));
        if desc.usage.is_empty()
            || desc.size > self.limits.max_buffer_size
            || missing_ray_query
            || missing_mappable_primary
        {
            let msg = format!(
                "Invalid buffer descriptor: size={}, usage={:?}, enabled_features={:?}",
                desc.size, desc.usage, self.features
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
        .unwrap_or_else(|_| {
            (
                String::new(),
                BindingMap {
                    buffers: Default::default(),
                    textures: Default::default(),
                    samplers: Default::default(),
                    immediate_slot: None,
                    array_length_slots: Default::default(),
                },
            )
        });

        ShaderModule {
            module: Arc::new(module),
            info: Arc::new(info),
            msl_source: Arc::new(msl_source),
            bindings: Arc::new(bindings),
        }
    }

    pub fn create_bind_group_layout(
        &self,
        desc: &BindGroupLayoutDescriptor<'_>,
    ) -> BindGroupLayout {
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

    pub fn create_compute_pipeline(
        &self,
        desc: &ComputePipelineDescriptor<'_>,
    ) -> ComputePipeline {
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
            let vs_fn_ptr = f_fn(
                vs_lib.as_ptr(),
                s.new_function_with_name,
                nsstring_from_str(vs_ep),
            );
            assert!(!vs_fn_ptr.is_null(), "Vertex function `{vs_ep}` not found");
            let vs_fn = ObjCObject(vs_fn_ptr);

            let rpd_ptr = f_init(
                f_alloc(s.cls_mtl_render_pipeline_descriptor, s.alloc),
                s.init,
            );
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
                let fs_fn_ptr = f_fn(
                    fs_lib.as_ptr(),
                    s.new_function_with_name,
                    nsstring_from_str(fs_ep),
                );
                assert!(
                    !fs_fn_ptr.is_null(),
                    "Fragment function `{fs_ep}` not found"
                );
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
                            f_set_u64(
                                att,
                                s.set_source_rgb_blend_factor,
                                blend.color.src_factor.to_mtl(),
                            );
                            f_set_u64(
                                att,
                                s.set_destination_rgb_blend_factor,
                                blend.color.dst_factor.to_mtl(),
                            );
                            f_set_u64(
                                att,
                                s.set_rgb_blend_operation,
                                blend.color.operation.to_mtl(),
                            );
                            f_set_u64(
                                att,
                                s.set_source_alpha_blend_factor,
                                blend.alpha.src_factor.to_mtl(),
                            );
                            f_set_u64(
                                att,
                                s.set_destination_alpha_blend_factor,
                                blend.alpha.dst_factor.to_mtl(),
                            );
                            f_set_u64(
                                att,
                                s.set_alpha_blend_operation,
                                blend.alpha.operation.to_mtl(),
                            );
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
                let dsd_ptr = f_init(
                    f_alloc(s.cls_mtl_depth_stencil_descriptor, s.alloc),
                    s.init,
                );
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
                let f_new_dss: unsafe extern "C" fn(
                    *mut c_void,
                    *mut c_void,
                    *mut c_void,
                ) -> *mut c_void = std::mem::transmute(metal_msg_send as *const c_void);
                let dss_ptr = f_new_dss(
                    self.metal.raw.as_ptr(),
                    s.new_depth_stencil_state,
                    dsd.as_ptr(),
                );
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
            let f_new_tex: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
            ) -> *mut c_void = std::mem::transmute(metal_msg_send as *const c_void);

            let td = f_td(
                s.cls_mtl_texture_descriptor,
                s.tex_desc_2d,
                desc.format.to_mtl_pixel_format(),
                desc.size.width.max(1) as usize,
                desc.size.height.max(1) as usize,
                desc.mip_level_count > 1,
            );
            f_set_u64(
                td,
                s.set_mipmap_level_count,
                desc.mip_level_count.max(1) as u64,
            );
            if desc.sample_count > 1 {
                f_set_u64(td, s.set_sample_count, desc.sample_count as u64);
                f_set_u64(td, s.set_texture_type, 4); // MTLTextureType2DMultisample
            } else if desc.dimension == TextureDimension::D3 {
                f_set_u64(td, s.set_texture_type, 7); // MTLTextureType3D
                f_set_u64(
                    td,
                    s.set_depth,
                    desc.size.depth_or_array_layers.max(1) as u64,
                );
            } else if desc.size.depth_or_array_layers > 1 {
                f_set_u64(td, s.set_texture_type, 3); // MTLTextureType2DArray
                f_set_u64(
                    td,
                    s.set_array_length,
                    desc.size.depth_or_array_layers as u64,
                );
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

            let is_depth = matches!(
                desc.format,
                TextureFormat::Depth32Float | TextureFormat::Depth24Plus
            );
            let storage_mode =
                if is_depth || desc.sample_count > 1 || !self.metal.has_unified_memory {
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
            let f_new_samp: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
            ) -> *mut c_void = std::mem::transmute(metal_msg_send as *const c_void);

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

    /// Creates a Bottom-Level Acceleration Structure (`BLAS`) for hardware RTX / Ray Tracing.
    pub fn create_blas(&self, label: Label<'_>, geometries: Vec<BlasGeometryDescriptor>) -> Blas {
        Blas::new(label, geometries)
    }

    /// Creates a Top-Level Acceleration Structure (`TLAS`) over scene instances for hardware RTX / Ray Tracing.
    pub fn create_tlas(&self, label: Label<'_>, instances: Vec<TlasInstance>) -> Tlas {
        Tlas::new(label, instances)
    }

    /// Queries the hardware RTX & Ray Tracing capabilities of this GPU device.
    pub fn rtx_capabilities(&self) -> RtxCapabilities {
        RtxCapabilities::for_backend(self.info.backend)
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

    pub fn submit<I: IntoIterator<Item = CommandBuffer>>(
        &self,
        command_buffers: I,
    ) -> SubmissionIndex {
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
                                if !active_compute_enc.is_null() {
                                    f_void(active_compute_enc, s.end_encoding);
                                    active_compute_enc = ptr::null_mut();
                                }
                                let sub_cmd_ptr =
                                    f_obj(self.metal.queue.as_ptr(), s.command_buffer);
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
                                let (start_ns, end_ns) =
                                    if t_start_s > 0.0 && t_end_s >= t_start_s {
                                        ((t_start_s * 1e9) as u64, (t_end_s * 1e9) as u64)
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
