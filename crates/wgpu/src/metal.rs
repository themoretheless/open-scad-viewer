#![allow(unsafe_code, unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(target_os = "macos")]
#[link(name = "objc")]
unsafe extern "C" {
    pub fn objc_getClass(name: *const c_char) -> *mut c_void;
    pub fn sel_registerName(name: *const c_char) -> *mut c_void;
    pub fn objc_msgSend();
}

#[cfg(target_os = "macos")]
#[link(name = "Metal", kind = "framework")]
unsafe extern "C" {
    pub fn MTLCreateSystemDefaultDevice() -> *mut c_void;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {}

#[cfg(target_os = "macos")]
#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

#[derive(Clone, Copy)]
pub struct Selectors {
    pub release: *mut c_void,
    pub retain: *mut c_void,
    pub utf8_string: *mut c_void,
    pub string_with_utf8: *mut c_void,
    pub localized_description: *mut c_void,
    pub name: *mut c_void,
    pub has_unified_memory: *mut c_void,
    pub recommended_max_working_set_size: *mut c_void,
    pub max_buffer_length: *mut c_void,
    pub new_command_queue: *mut c_void,
    pub new_command_queue_with_max: *mut c_void,
    pub command_buffer: *mut c_void,
    pub command_buffer_unretained: *mut c_void,
    pub compute_command_encoder: *mut c_void,
    pub blit_command_encoder: *mut c_void,
    pub render_command_encoder_with_desc: *mut c_void,
    pub end_encoding: *mut c_void,
    pub commit: *mut c_void,
    pub wait_until_completed: *mut c_void,
    pub status: *mut c_void,
    pub new_buffer_with_length: *mut c_void,
    pub new_buffer_with_bytes: *mut c_void,
    pub new_buffer_with_bytes_no_copy: *mut c_void,
    pub contents: *mut c_void,
    pub length: *mut c_void,
    pub new_library_with_source: *mut c_void,
    pub new_function_with_name: *mut c_void,
    pub new_compute_pipeline_with_function: *mut c_void,
    pub new_render_pipeline_with_desc: *mut c_void,
    pub set_compute_pipeline_state: *mut c_void,
    pub set_buffer: *mut c_void,
    pub set_bytes: *mut c_void,
    pub set_texture: *mut c_void,
    pub set_sampler_state: *mut c_void,
    pub dispatch_threadgroups: *mut c_void,
    pub copy_from_buffer: *mut c_void,
    pub copy_from_texture_to_buffer: *mut c_void,
    pub copy_from_texture_to_texture: *mut c_void,
    pub fill_buffer: *mut c_void,
    pub new_counter_sample_buffer: *mut c_void,
    pub sample_counters_in_buffer: *mut c_void,
    pub resolve_counter_range: *mut c_void,
    pub supports_counter_sampling: *mut c_void,
    // Texture & Render descriptors
    pub tex_desc_2d: *mut c_void,
    pub set_usage: *mut c_void,
    pub set_storage_mode: *mut c_void,
    pub set_mipmap_level_count: *mut c_void,
    pub set_sample_count: *mut c_void,
    pub set_texture_type: *mut c_void,
    pub set_array_length: *mut c_void,
    pub set_depth: *mut c_void,
    pub new_texture_with_desc: *mut c_void,
    pub replace_region: *mut c_void,
    pub get_bytes: *mut c_void,
    pub width: *mut c_void,
    pub height: *mut c_void,
    pub mipmap_level_count: *mut c_void,
    pub pixel_format: *mut c_void,
    // Sampler
    pub alloc: *mut c_void,
    pub init: *mut c_void,
    pub set_min_filter: *mut c_void,
    pub set_mag_filter: *mut c_void,
    pub set_mip_filter: *mut c_void,
    pub set_s_address_mode: *mut c_void,
    pub set_t_address_mode: *mut c_void,
    pub set_r_address_mode: *mut c_void,
    pub set_compare_function: *mut c_void,
    pub new_sampler_state: *mut c_void,
    // Render pipeline & pass
    pub set_vertex_function: *mut c_void,
    pub set_fragment_function: *mut c_void,
    pub color_attachments: *mut c_void,
    pub object_at_indexed_subscript: *mut c_void,
    pub set_pixel_format: *mut c_void,
    pub set_write_mask: *mut c_void,
    pub set_blending_enabled: *mut c_void,
    pub set_source_rgb_blend_factor: *mut c_void,
    pub set_destination_rgb_blend_factor: *mut c_void,
    pub set_rgb_blend_operation: *mut c_void,
    pub set_source_alpha_blend_factor: *mut c_void,
    pub set_destination_alpha_blend_factor: *mut c_void,
    pub set_alpha_blend_operation: *mut c_void,
    pub set_depth_attachment_pixel_format: *mut c_void,
    pub vertex_descriptor: *mut c_void,
    pub attributes: *mut c_void,
    pub layouts: *mut c_void,
    pub set_format: *mut c_void,
    pub set_offset: *mut c_void,
    pub set_buffer_index: *mut c_void,
    pub set_stride: *mut c_void,
    pub set_step_function: *mut c_void,
    pub set_step_rate: *mut c_void,
    // Depth stencil
    pub set_depth_write_enabled: *mut c_void,
    pub set_depth_compare_function: *mut c_void,
    pub new_depth_stencil_state: *mut c_void,
    // Render pass descriptor
    pub render_pass_descriptor: *mut c_void,
    pub depth_attachment: *mut c_void,
    pub set_attachment_texture: *mut c_void,
    pub set_load_action: *mut c_void,
    pub set_store_action: *mut c_void,
    pub set_clear_color: *mut c_void,
    pub set_clear_depth: *mut c_void,
    pub set_slice: *mut c_void,
    pub set_level: *mut c_void,
    // Render command encoder methods
    pub set_render_pipeline_state: *mut c_void,
    pub set_depth_stencil_state: *mut c_void,
    pub set_cull_mode: *mut c_void,
    pub set_front_facing_winding: *mut c_void,
    pub set_depth_bias: *mut c_void,
    pub set_viewport: *mut c_void,
    pub set_scissor_rect: *mut c_void,
    pub set_vertex_buffer: *mut c_void,
    pub set_vertex_bytes: *mut c_void,
    pub set_fragment_buffer: *mut c_void,
    pub set_fragment_bytes: *mut c_void,
    pub set_vertex_texture: *mut c_void,
    pub set_fragment_texture: *mut c_void,
    pub set_vertex_sampler_state: *mut c_void,
    pub set_fragment_sampler_state: *mut c_void,
    pub draw_primitives: *mut c_void,
    pub draw_indexed_primitives: *mut c_void,
    // Classes
    pub cls_ns_string: *mut c_void,
    pub cls_mtl_compile_options: *mut c_void,
    pub cls_mtl_texture_descriptor: *mut c_void,
    pub cls_mtl_sampler_descriptor: *mut c_void,
    pub cls_mtl_render_pipeline_descriptor: *mut c_void,
    pub cls_mtl_depth_stencil_descriptor: *mut c_void,
    pub cls_mtl_render_pass_descriptor: *mut c_void,
    pub cls_mtl_counter_sample_buffer_descriptor: *mut c_void,
    pub cls_mtl_compute_pipeline_descriptor: *mut c_void,
    pub set_compute_function: *mut c_void,
    pub set_thread_group_size_is_multiple: *mut c_void,
    pub new_compute_pipeline_with_descriptor: *mut c_void,
    pub set_fast_math_enabled: *mut c_void,
    pub counter_sets: *mut c_void,
    pub count: *mut c_void,
    pub set_counter_set: *mut c_void,
    pub set_sample_count_counter: *mut c_void,
    pub gpu_start_time: *mut c_void,
    pub gpu_end_time: *mut c_void,
}

unsafe impl Send for Selectors {}
unsafe impl Sync for Selectors {}

static SELECTORS: OnceLock<Selectors> = OnceLock::new();

#[cfg(target_os = "macos")]
fn sel(name: &str) -> *mut c_void {
    let c = CString::new(name).unwrap();
    unsafe { sel_registerName(c.as_ptr()) }
}

#[cfg(target_os = "macos")]
fn cls(name: &str) -> *mut c_void {
    let c = CString::new(name).unwrap();
    unsafe { objc_getClass(c.as_ptr()) }
}

#[cfg(target_os = "macos")]
pub fn selectors() -> &'static Selectors {
    SELECTORS.get_or_init(|| Selectors {
        release: sel("release"),
        retain: sel("retain"),
        utf8_string: sel("UTF8String"),
        string_with_utf8: sel("stringWithUTF8String:"),
        localized_description: sel("localizedDescription"),
        name: sel("name"),
        has_unified_memory: sel("hasUnifiedMemory"),
        recommended_max_working_set_size: sel("recommendedMaxWorkingSetSize"),
        max_buffer_length: sel("maxBufferLength"),
        new_command_queue: sel("newCommandQueue"),
        new_command_queue_with_max: sel("newCommandQueueWithMaxCommandBufferCount:"),
        command_buffer: sel("commandBuffer"),
        command_buffer_unretained: sel("commandBufferWithUnretainedReferences"),
        compute_command_encoder: sel("computeCommandEncoder"),
        blit_command_encoder: sel("blitCommandEncoder"),
        render_command_encoder_with_desc: sel("renderCommandEncoderWithDescriptor:"),
        end_encoding: sel("endEncoding"),
        commit: sel("commit"),
        wait_until_completed: sel("waitUntilCompleted"),
        status: sel("status"),
        new_buffer_with_length: sel("newBufferWithLength:options:"),
        new_buffer_with_bytes: sel("newBufferWithBytes:length:options:"),
        new_buffer_with_bytes_no_copy: sel("newBufferWithBytesNoCopy:length:options:deallocator:"),
        contents: sel("contents"),
        length: sel("length"),
        new_library_with_source: sel("newLibraryWithSource:options:error:"),
        new_function_with_name: sel("newFunctionWithName:"),
        new_compute_pipeline_with_function: sel("newComputePipelineStateWithFunction:error:"),
        new_render_pipeline_with_desc: sel("newRenderPipelineStateWithDescriptor:error:"),
        set_compute_pipeline_state: sel("setComputePipelineState:"),
        set_buffer: sel("setBuffer:offset:atIndex:"),
        set_bytes: sel("setBytes:length:atIndex:"),
        set_texture: sel("setTexture:atIndex:"),
        set_sampler_state: sel("setSamplerState:atIndex:"),
        dispatch_threadgroups: sel("dispatchThreadgroups:threadsPerThreadgroup:"),
        copy_from_buffer: sel(
            "copyFromBuffer:sourceOffset:toBuffer:destinationOffset:size:",
        ),
        copy_from_texture_to_buffer: sel(
            "copyFromTexture:sourceSlice:sourceLevel:sourceOrigin:sourceSize:toBuffer:destinationOffset:destinationBytesPerRow:destinationBytesPerImage:",
        ),
        copy_from_texture_to_texture: sel(
            "copyFromTexture:sourceSlice:sourceLevel:sourceOrigin:sourceSize:toTexture:destinationSlice:destinationLevel:destinationOrigin:",
        ),
        fill_buffer: sel("fillBuffer:range:value:"),
        new_counter_sample_buffer: sel("newCounterSampleBufferWithDescriptor:error:"),
        sample_counters_in_buffer: sel("sampleCountersInBuffer:atSampleIndex:withBarrier:"),
        resolve_counter_range: sel("resolveCounterRange:"),
        supports_counter_sampling: sel("supportsCounterSampling:"),
        tex_desc_2d: sel("texture2DDescriptorWithPixelFormat:width:height:mipmapped:"),
        set_usage: sel("setUsage:"),
        set_storage_mode: sel("setStorageMode:"),
        set_mipmap_level_count: sel("setMipmapLevelCount:"),
        set_sample_count: sel("setSampleCount:"),
        set_texture_type: sel("setTextureType:"),
        set_array_length: sel("setArrayLength:"),
        set_depth: sel("setDepth:"),
        new_texture_with_desc: sel("newTextureWithDescriptor:"),
        replace_region: sel("replaceRegion:mipmapLevel:slice:withBytes:bytesPerRow:bytesPerImage:"),
        get_bytes: sel("getBytes:bytesPerRow:bytesPerImage:fromRegion:mipmapLevel:slice:"),
        width: sel("width"),
        height: sel("height"),
        mipmap_level_count: sel("mipmapLevelCount"),
        pixel_format: sel("pixelFormat"),
        alloc: sel("alloc"),
        init: sel("init"),
        set_min_filter: sel("setMinFilter:"),
        set_mag_filter: sel("setMagFilter:"),
        set_mip_filter: sel("setMipFilter:"),
        set_s_address_mode: sel("setSAddressMode:"),
        set_t_address_mode: sel("setTAddressMode:"),
        set_r_address_mode: sel("setRAddressMode:"),
        set_compare_function: sel("setCompareFunction:"),
        new_sampler_state: sel("newSamplerStateWithDescriptor:"),
        set_vertex_function: sel("setVertexFunction:"),
        set_fragment_function: sel("setFragmentFunction:"),
        color_attachments: sel("colorAttachments"),
        object_at_indexed_subscript: sel("objectAtIndexedSubscript:"),
        set_pixel_format: sel("setPixelFormat:"),
        set_write_mask: sel("setWriteMask:"),
        set_blending_enabled: sel("setBlendingEnabled:"),
        set_source_rgb_blend_factor: sel("setSourceRGBBlendFactor:"),
        set_destination_rgb_blend_factor: sel("setDestinationRGBBlendFactor:"),
        set_rgb_blend_operation: sel("setRgbBlendOperation:"),
        set_source_alpha_blend_factor: sel("setSourceAlphaBlendFactor:"),
        set_destination_alpha_blend_factor: sel("setDestinationAlphaBlendFactor:"),
        set_alpha_blend_operation: sel("setAlphaBlendOperation:"),
        set_depth_attachment_pixel_format: sel("setDepthAttachmentPixelFormat:"),
        vertex_descriptor: sel("vertexDescriptor"),
        attributes: sel("attributes"),
        layouts: sel("layouts"),
        set_format: sel("setFormat:"),
        set_offset: sel("setOffset:"),
        set_buffer_index: sel("setBufferIndex:"),
        set_stride: sel("setStride:"),
        set_step_function: sel("setStepFunction:"),
        set_step_rate: sel("setStepRate:"),
        set_depth_write_enabled: sel("setDepthWriteEnabled:"),
        set_depth_compare_function: sel("setDepthCompareFunction:"),
        new_depth_stencil_state: sel("newDepthStencilStateWithDescriptor:"),
        render_pass_descriptor: sel("renderPassDescriptor"),
        depth_attachment: sel("depthAttachment"),
        set_attachment_texture: sel("setTexture:"),
        set_load_action: sel("setLoadAction:"),
        set_store_action: sel("setStoreAction:"),
        set_clear_color: sel("setClearColor:"),
        set_clear_depth: sel("setClearDepth:"),
        set_slice: sel("setSlice:"),
        set_level: sel("setLevel:"),
        set_render_pipeline_state: sel("setRenderPipelineState:"),
        set_depth_stencil_state: sel("setDepthStencilState:"),
        set_cull_mode: sel("setCullMode:"),
        set_front_facing_winding: sel("setFrontFacingWinding:"),
        set_depth_bias: sel("setDepthBias:slopeScale:clamp:"),
        set_viewport: sel("setViewport:"),
        set_scissor_rect: sel("setScissorRect:"),
        set_vertex_buffer: sel("setVertexBuffer:offset:atIndex:"),
        set_vertex_bytes: sel("setVertexBytes:length:atIndex:"),
        set_fragment_buffer: sel("setFragmentBuffer:offset:atIndex:"),
        set_fragment_bytes: sel("setFragmentBytes:length:atIndex:"),
        set_vertex_texture: sel("setVertexTexture:atIndex:"),
        set_fragment_texture: sel("setFragmentTexture:atIndex:"),
        set_vertex_sampler_state: sel("setVertexSamplerState:atIndex:"),
        set_fragment_sampler_state: sel("setFragmentSamplerState:atIndex:"),
        draw_primitives: sel("drawPrimitives:vertexStart:vertexCount:instanceCount:baseInstance:"),
        draw_indexed_primitives: sel(
            "drawIndexedPrimitives:indexCount:indexType:indexBuffer:indexBufferOffset:instanceCount:baseVertex:baseInstance:",
        ),
        cls_ns_string: cls("NSString"),
        cls_mtl_compile_options: cls("MTLCompileOptions"),
        cls_mtl_texture_descriptor: cls("MTLTextureDescriptor"),
        cls_mtl_sampler_descriptor: cls("MTLSamplerDescriptor"),
        cls_mtl_render_pipeline_descriptor: cls("MTLRenderPipelineDescriptor"),
        cls_mtl_depth_stencil_descriptor: cls("MTLDepthStencilDescriptor"),
        cls_mtl_render_pass_descriptor: cls("MTLRenderPassDescriptor"),
        cls_mtl_counter_sample_buffer_descriptor: cls("MTLCounterSampleBufferDescriptor"),
        cls_mtl_compute_pipeline_descriptor: cls("MTLComputePipelineDescriptor"),
        set_compute_function: sel("setComputeFunction:"),
        set_thread_group_size_is_multiple: sel("setThreadGroupSizeIsMultipleOfThreadExecutionWidth:"),
        new_compute_pipeline_with_descriptor: sel("newComputePipelineStateWithDescriptor:options:reflection:error:"),
        set_fast_math_enabled: sel("setFastMathEnabled:"),
        counter_sets: sel("counterSets"),
        count: sel("count"),
        set_counter_set: sel("setCounterSet:"),
        set_sample_count_counter: sel("setSampleCount:"),
        gpu_start_time: sel("GPUStartTime"),
        gpu_end_time: sel("GPUEndTime"),
    })
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MTLSize {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MTLOrigin {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MTLRegion {
    pub origin: MTLOrigin,
    pub size: MTLSize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NSRange {
    pub location: usize,
    pub length: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MTLClearColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MTLViewport {
    pub origin_x: f64,
    pub origin_y: f64,
    pub width: f64,
    pub height: f64,
    pub znear: f64,
    pub zfar: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MTLScissorRect {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug)]
pub struct ObjCObject(pub *mut c_void);

unsafe impl Send for ObjCObject {}
unsafe impl Sync for ObjCObject {}

impl ObjCObject {
    #[inline]
    pub fn from_retained(ptr: *mut c_void) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self(ptr))
        }
    }

    #[inline]
    pub fn from_unretained(ptr: *mut c_void) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            #[cfg(target_os = "macos")]
            unsafe {
                let s = selectors();
                let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const c_void);
                f(ptr, s.retain);
            }
            Some(Self(ptr))
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }
}

impl Clone for ObjCObject {
    fn clone(&self) -> Self {
        #[cfg(target_os = "macos")]
        unsafe {
            let s = selectors();
            let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            f(self.0, s.retain);
        }
        Self(self.0)
    }
}

impl Drop for ObjCObject {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        if !self.0.is_null() {
            unsafe {
                let s = selectors();
                let f: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                    std::mem::transmute(objc_msgSend as *const c_void);
                f(self.0, s.release);
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn nsstring_from_str(st: &str) -> *mut c_void {
    let s = selectors();
    let c = CString::new(st).unwrap_or_default();
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const c_void);
    f(s.cls_ns_string, s.string_with_utf8, c.as_ptr())
}

#[cfg(target_os = "macos")]
pub unsafe fn nsstring_to_string(ns: *mut c_void) -> String {
    if ns.is_null() {
        return String::new();
    }
    let s = selectors();
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const c_char =
        std::mem::transmute(objc_msgSend as *const c_void);
    let cstr = f(ns, s.utf8_string);
    if cstr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(cstr).to_string_lossy().into_owned()
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn nserror_description(err: *mut c_void) -> String {
    if err.is_null() {
        return "unknown Metal error".to_string();
    }
    let s = selectors();
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const c_void);
    let desc = f(err, s.localized_description);
    nsstring_to_string(desc)
}

#[derive(Clone)]
pub struct MetalDevice {
    pub raw: Arc<ObjCObject>,
    pub queue: Arc<ObjCObject>,
    pub name: String,
    pub has_unified_memory: bool,
    pub max_buffer_length: u64,
    pub recommended_working_set: u64,
    // Caches for zero-overhead pipeline and buffer reuse
    pub compute_pipeline_cache: Arc<Mutex<std::collections::HashMap<u64, Arc<ObjCObject>>>>,
    pub render_pipeline_cache: Arc<Mutex<std::collections::HashMap<u64, (Arc<ObjCObject>, Option<Arc<ObjCObject>>)>>>,
    pub buffer_pool: Arc<Mutex< Vec<(usize, u64, Arc<ObjCObject>, *mut u8)> >>,
    pub last_cmd_buf: Arc<Mutex<Option<(Arc<ObjCObject>, u64)>>>,
    pub(crate) pending_compute_ops: Arc<Mutex<Vec<crate::ComputeDispatchOp>>>,
    pub next_submission_index: Arc<std::sync::atomic::AtomicU64>,
}

unsafe impl Send for MetalDevice {}
unsafe impl Sync for MetalDevice {}

impl MetalDevice {
    #[cfg(target_os = "macos")]
    pub fn create_system_default() -> Option<Self> {
        unsafe {
            let dev_ptr = MTLCreateSystemDefaultDevice();
            let raw = Arc::new(ObjCObject::from_retained(dev_ptr)?);
            let s = selectors();

            let f_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            let f_bool: unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool =
                std::mem::transmute(objc_msgSend as *const c_void);
            let f_u64: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u64 =
                std::mem::transmute(objc_msgSend as *const c_void);
            let f_queue: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);

            let name_ns = f_ptr(raw.as_ptr(), s.name);
            let name = nsstring_to_string(name_ns);
            let has_unified_memory = f_bool(raw.as_ptr(), s.has_unified_memory);
            let max_buffer_length = f_u64(raw.as_ptr(), s.max_buffer_length);
            let recommended_working_set = f_u64(raw.as_ptr(), s.recommended_max_working_set_size);

            let q_ptr = f_queue(raw.as_ptr(), s.new_command_queue_with_max, 512);
            let q_ptr = if q_ptr.is_null() {
                f_ptr(raw.as_ptr(), s.new_command_queue)
            } else {
                q_ptr
            };
            let queue = Arc::new(ObjCObject::from_retained(q_ptr)?);

            Some(Self {
                raw,
                queue,
                name,
                has_unified_memory,
                max_buffer_length: max_buffer_length.max(256 * 1024 * 1024),
                recommended_working_set,
                compute_pipeline_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
                render_pipeline_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
                buffer_pool: Arc::new(Mutex::new(Vec::new())),
                last_cmd_buf: Arc::new(Mutex::new(None)),
                pending_compute_ops: Arc::new(Mutex::new(Vec::with_capacity(32))),
                next_submission_index: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            })
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn create_system_default() -> Option<Self> {
        None
    }

    #[cfg(target_os = "macos")]
    pub fn alloc_buffer(&self, size: usize, options: u64) -> (Arc<ObjCObject>, *mut u8) {
        let aligned_size = ((size.max(16) + 255) & !255).max(256);
        if let Ok(mut pool) = self.buffer_pool.lock() {
            if let Some(pos) = pool
                .iter()
                .position(|(sz, opt, _, _)| *sz == aligned_size && *opt == options)
            {
                let (_, _, buf, ptr) = pool.swap_remove(pos);
                if !ptr.is_null() {
                    unsafe {
                        ptr::write_bytes(ptr, 0, size);
                    }
                }
                return (buf, ptr);
            }
        }
        unsafe {
            let s = selectors();
            let f_buf: unsafe extern "C" fn(*mut c_void, *mut c_void, usize, u64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            let f_contents: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut u8 =
                std::mem::transmute(objc_msgSend as *const c_void);
            let raw_buf = f_buf(self.raw.as_ptr(), s.new_buffer_with_length, aligned_size, options);
            let ptr = if (options & 0xF0) == (2 << 4) {
                // MTLStorageModePrivate has no CPU contents pointer
                ptr::null_mut()
            } else {
                f_contents(raw_buf, s.contents)
            };
            (Arc::new(ObjCObject(raw_buf)), ptr)
        }
    }

    #[cfg(target_os = "macos")]
    pub fn alloc_buffer_with_bytes(&self, bytes: &[u8], options: u64) -> (Arc<ObjCObject>, *mut u8) {
        if bytes.is_empty() {
            return self.alloc_buffer(16, options);
        }
        let aligned_size = ((bytes.len() + 255) & !255).max(256);
        if let Ok(mut pool) = self.buffer_pool.lock() {
            if let Some(pos) = pool
                .iter()
                .position(|(sz, opt, _, ptr)| *sz == aligned_size && *opt == options && !ptr.is_null())
            {
                let (_, _, buf, ptr) = pool.swap_remove(pos);
                unsafe {
                    ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
                }
                return (buf, ptr);
            }
        }
        unsafe {
            let s = selectors();
            let f_buf: unsafe extern "C" fn(*mut c_void, *mut c_void, usize, u64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            let f_contents: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut u8 =
                std::mem::transmute(objc_msgSend as *const c_void);
            let raw_buf = f_buf(self.raw.as_ptr(), s.new_buffer_with_length, aligned_size, options);
            let ptr = f_contents(raw_buf, s.contents);
            if !ptr.is_null() {
                ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            }
            (Arc::new(ObjCObject(raw_buf)), ptr)
        }
    }

    /// ASD (Apple Silicon Direct / Accelerated Storage Direct) zero-copy page-aligned buffer creation
    #[cfg(target_os = "macos")]
    pub unsafe fn alloc_buffer_no_copy(
        &self,
        page_aligned_ptr: *mut c_void,
        page_aligned_len: usize,
        options: u64,
    ) -> Option<(Arc<ObjCObject>, *mut u8)> {
        let s = selectors();
        let f_nocopy: unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut c_void,
            usize,
            u64,
            *mut c_void,
        ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const c_void);
        let raw_buf = f_nocopy(
            self.raw.as_ptr(),
            s.new_buffer_with_bytes_no_copy,
            page_aligned_ptr,
            page_aligned_len,
            options,
            ptr::null_mut(),
        );
        if raw_buf.is_null() {
            return None;
        }
        Some((Arc::new(ObjCObject(raw_buf)), page_aligned_ptr as *mut u8))
    }

    pub fn recycle_buffer(&self, size: usize, options: u64, buf: Arc<ObjCObject>, ptr: *mut u8) {
        if Arc::strong_count(&buf) == 1 && !ptr.is_null() {
            let aligned_size = ((size.max(16) + 255) & !255).max(256);
            if let Ok(mut pool) = self.buffer_pool.lock() {
                if pool.len() < 256 {
                    pool.push((aligned_size, options, buf, ptr));
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn compile_compute_pipeline(
        &self,
        msl_source: &str,
        entry_point: &str,
        wg_total: u32,
    ) -> Result<Arc<ObjCObject>, String> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        msl_source.hash(&mut hasher);
        entry_point.hash(&mut hasher);
        let key = hasher.finish();

        if let Ok(cache) = self.compute_pipeline_cache.lock() {
            if let Some(pso) = cache.get(&key) {
                return Ok(pso.clone());
            }
        }

        unsafe {
            let s = selectors();
            let src_ns = nsstring_from_str(msl_source);
            let mut err: *mut c_void = ptr::null_mut();
            let f_alloc: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            let f_init: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            let f_set_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, bool) =
                std::mem::transmute(objc_msgSend as *const c_void);
            let opts_ptr = f_init(f_alloc(s.cls_mtl_compile_options, s.alloc), s.init);
            let opts = ObjCObject(opts_ptr);
            f_set_bool(opts.as_ptr(), s.set_fast_math_enabled, true);

            let f_lib: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
                *mut c_void,
                *mut *mut c_void,
            ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const c_void);

            let lib_ptr = f_lib(
                self.raw.as_ptr(),
                s.new_library_with_source,
                src_ns,
                opts.as_ptr(),
                &mut err,
            );
            if lib_ptr.is_null() {
                return Err(format!(
                    "MSL compilation failed: {}\n--- MSL SOURCE ---\n{}",
                    nserror_description(err),
                    msl_source
                ));
            }
            let lib = ObjCObject(lib_ptr);

            let fn_ns = nsstring_from_str(entry_point);
            let f_fn: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            let func_ptr = f_fn(lib.as_ptr(), s.new_function_with_name, fn_ns);
            if func_ptr.is_null() {
                return Err(format!(
                    "MSL function `{entry_point}` not found in library\n--- MSL ---\n{msl_source}"
                ));
            }
            let func = ObjCObject(func_ptr);

            let mut pso_err: *mut c_void = ptr::null_mut();
            let f_set_obj: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const c_void);
            let cpd_ptr = f_init(f_alloc(s.cls_mtl_compute_pipeline_descriptor, s.alloc), s.init);
            let cpd = ObjCObject(cpd_ptr);
            f_set_obj(cpd.as_ptr(), s.set_compute_function, func.as_ptr());
            f_set_bool(
                cpd.as_ptr(),
                s.set_thread_group_size_is_multiple,
                wg_total >= 32 && wg_total % 32 == 0,
            );

            let f_pso_desc: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                *mut c_void,
                usize,
                *mut c_void,
                *mut *mut c_void,
            ) -> *mut c_void = std::mem::transmute(objc_msgSend as *const c_void);
            let pso_ptr = f_pso_desc(
                self.raw.as_ptr(),
                s.new_compute_pipeline_with_descriptor,
                cpd.as_ptr(),
                0,
                ptr::null_mut(),
                &mut pso_err,
            );
            if pso_ptr.is_null() {
                return Err(format!(
                    "MTLComputePipelineState creation failed: {}",
                    nserror_description(pso_err)
                ));
            }
            let pso = Arc::new(ObjCObject(pso_ptr));
            if let Ok(mut cache) = self.compute_pipeline_cache.lock() {
                cache.insert(key, pso.clone());
            }
            Ok(pso)
        }
    }

    #[cfg(target_os = "macos")]
    pub fn flush_pending_compute(&self) {
        let ops: Vec<crate::ComputeDispatchOp> = self
            .pending_compute_ops
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default();
        if ops.is_empty() {
            return;
        }
        unsafe {
            let s = selectors();
            let f_obj: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const c_void);
            let f_void: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const c_void);

            let cmd_buf_ptr = f_obj(self.queue.as_ptr(), s.command_buffer);
            if cmd_buf_ptr.is_null() {
                return;
            }
            let cmd_buf = Arc::new(ObjCObject::from_unretained(cmd_buf_ptr).unwrap());
            let enc = f_obj(cmd_buf.as_ptr(), s.compute_command_encoder);
            crate::encode_compute_dispatches(enc, &ops);
            f_void(enc, s.end_encoding);
            f_void(cmd_buf.as_ptr(), s.commit);
            let idx = self.next_submission_index.load(std::sync::atomic::Ordering::Relaxed);
            if let Ok(mut last) = self.last_cmd_buf.lock() {
                *last = Some((cmd_buf, idx));
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn wait_idle(&self) {
        self.flush_pending_compute();
        let cmd_opt = self.last_cmd_buf.lock().ok().and_then(|mut g| g.take());
        if let Some((cmd_buf, _)) = cmd_opt {
            unsafe {
                let s = selectors();
                let f_status: unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize =
                    std::mem::transmute(objc_msgSend as *const c_void);
                let st = f_status(cmd_buf.as_ptr(), s.status);
                // MTLCommandBufferStatusCompleted = 4, Error = 5
                if st < 4 {
                    let f_wait: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                        std::mem::transmute(objc_msgSend as *const c_void);
                    f_wait(cmd_buf.as_ptr(), s.wait_until_completed);
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn is_last_cmd_completed(&self) -> bool {
        if let Ok(pending) = self.pending_compute_ops.lock() {
            if !pending.is_empty() {
                return false;
            }
        }
        let Ok(guard) = self.last_cmd_buf.lock() else {
            return true;
        };
        let Some((cmd_buf, _)) = guard.as_ref() else {
            return true;
        };
        unsafe {
            let s = selectors();
            let f_status: unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize =
                std::mem::transmute(objc_msgSend as *const c_void);
            f_status(cmd_buf.as_ptr(), s.status) >= 4
        }
    }
}

pub use selectors as metal_selectors;
pub use objc_msgSend as metal_msg_send;
