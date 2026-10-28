#![allow(unsafe_code)]

//! Dedicated GPU, Shader, Compute, CAD Normalization & Modern Storage/Memory Architecture crate.
//!
//! Combines according to **SOLID**, **DRY**, and **Divide & Conquer** principles:
//! 1. **[`asd`] — ASD (Apple Silicon Direct / Accelerated Storage Direct / GPUDirect Storage `cuFile` / DirectStorage)**:
//!    Zero-copy page-aligned memory-mapped storage and direct I/O streaming into GPU buffers (`MTLDevice::newBufferWithBytesNoCopy:` on macOS/iOS, `cuFileRead` on Linux/Windows CUDA, DirectStorage on Windows DX12, external memory fd/host-pointer on Vulkan/Android).
//! 2. **[`uma_duma`] — UMA (Unified Memory Architecture) & dUMA (Discrete Unified Memory Architecture)**:
//!    Single source of truth for hardware execution topology (`HardwareExecutionTopology`), memory tiers (`MemoryArchitecture`, `ModernMemoryTech`), and zero-copy vs ReBAR/DMA transfer planners (`UnifiedTransferPlan`).
//! 3. **[`shader_compiler`] — Multi-Backend Compute & Shaders**:
//!    Unified WGSL -> MSL / SPIR-V / HLSL / CUDA PTX shader compilation, validation, and portable compute kernel generators (`generate_wgsl_tiled_matmul_shader`, `generate_wgsl_parallel_reduction_shader`) across **Windows, macOS, Linux, iOS, Android, and Browser (WebGPU)**.
//! 4. **[`cad_kernels`] & [`cad`] — OpenSCAD & CAD Core Planning & Geometry Normalization**:
//!    Reusable OpenSCAD degree trigonometry, fragment resolution (`$fn`, `$fa`, `$fs`), primitive planners (`cube`, `sphere`, `cylinder`, `polyhedron`, `circle`, `square`), extrusion & slice planners (`linear_extrude`, `rotate_extrude`), `offset`, `resize`, `children` selection, transform matrix normalization, and GPU CAD compute shaders.

pub mod asd;
pub mod cad_kernels;
pub mod children_selection;
pub mod degree_math;
pub mod extrude_slices;
pub mod extrusion_plan;
pub mod fragments;
pub mod indexed_primitive;
pub mod offset_plan;
pub mod primitive_plan;
pub mod resize;
pub mod shader_compiler;
pub mod transform_plan;
pub mod uma_duma;

pub use asd::*;
pub use cad_kernels::*;
pub use shader_compiler::*;
pub use uma_duma::*;
pub use wgpu::opengl::{GlProfile, OpenGlDevice};
pub use wgpu::rtx::{
    Blas, BlasGeometryDescriptor, BlasGeometryKind, RtxCapabilities, RtxTier, Tlas, TlasInstance,
};
pub use wgpu::vulkan::{VkMemoryPropertyFlags, VulkanDevice};

pub mod cad {
    pub use crate::children_selection;
    pub use crate::degree_math::*;
    pub use crate::extrude_slices;
    pub use crate::extrusion_plan::*;
    pub use crate::fragments;
    pub use crate::indexed_primitive::*;
    pub use crate::offset_plan;
    pub use crate::primitive_plan::*;
    pub use crate::resize;
    pub use crate::transform_plan::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asd_page_buffer_aligns_to_16k_and_preserves_data() {
        let payload = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let buf = AsdPageBuffer::from_slice(&payload);
        assert_eq!(buf.page_size(), 16384);
        assert_eq!(buf.aligned_capacity(), 16384);
        assert_eq!(buf.len(), 8);
        assert_eq!(buf.as_slice(), &payload);
    }

    #[test]
    fn unified_transfer_plan_selects_asd_uma_and_duma_appropriately() {
        let asd_plan = UnifiedTransferPlan::for_payload(wgpu::Backend::Metal, 65536, true);
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(asd_plan.tech, ModernMemoryTech::Asd);
            assert!(asd_plan.uses_page_aligned_no_copy);
            assert!(asd_plan.zero_copy_cpu_write);
        }

        let duma_rebar = UnifiedTransferPlan::for_payload(wgpu::Backend::Vulkan, 1024 * 1024, false);
        assert_eq!(duma_rebar.tech, ModernMemoryTech::Duma);
        assert!(duma_rebar.uses_rebar_aperture);
        assert_eq!(duma_rebar.staging_ring_chunks, 1);

        let duma_dma =
            UnifiedTransferPlan::for_payload(wgpu::Backend::Cuda, 64 * 1024 * 1024, false);
        assert_eq!(duma_dma.tech, ModernMemoryTech::Duma);
        assert!(!duma_dma.uses_rebar_aperture);
        assert_eq!(duma_dma.staging_ring_chunks, 4);
    }

    #[test]
    fn multi_target_shader_compiler_emits_msl_spirv_hlsl_ptx_glsl_and_wgsl() {
        let wgsl = generate_wgsl_parallel_reduction_shader(128);
        let artifact = compile_shader_multi_target(&wgsl).expect("valid multi-target compilation");
        assert!(artifact.msl.contains("kernel void"));
        assert!(!artifact.spirv_words.is_empty());
        assert!(artifact.hlsl.contains("[numthreads(128, 1, 1)]"));
        assert!(artifact.cuda_ptx.contains(".target sm_89"));
        assert!(artifact.glsl.contains("#version 460 core"));
        assert!(artifact.glsl.contains("layout(local_size_x = 128"));
        assert!(artifact.wgsl.contains("@compute @workgroup_size(128, 1, 1)"));
    }

    #[test]
    fn opengl_vulkan_and_rtx_raytracing_subsystems_work() {
        let gl = OpenGlDevice::probe_available().expect("OpenGL device profile");
        assert!(gl.supports_compute_shaders);
        assert!(gl.supports_persistent_coherent_mapping);
        assert_ne!(gl.recommended_buffer_storage_flags(), 0);

        let vk = VulkanDevice::probe_available().expect("Vulkan 1.3 device profile");
        assert_eq!(vk.api_version.0, 1);
        assert_eq!(vk.api_version.1, 3);
        assert!(vk.supports_ray_tracing_pipeline);
        assert!(vk.supports_ray_query);
        assert!(vk.supports_external_memory_host_asd);
        assert_eq!(vk.memory_architecture(true), MemoryArchitecture::Asd);

        let blas = Blas::new(
            Some("cad-csg-blas"),
            vec![BlasGeometryDescriptor {
                kind: BlasGeometryKind::Triangles,
                vertex_count: 1024,
                primitive_count: 512,
                opaque: true,
                allow_compaction: true,
            }],
        );
        assert!(blas.compacted_size_bytes >= 512 * 64);
        let tlas = Tlas::new(
            Some("cad-scene-tlas"),
            vec![TlasInstance::identity(blas.device_address, 0)],
        );
        assert_eq!(tlas.instances.len(), 1);

        let rtx_vk = RtxCapabilities::for_backend(wgpu::Backend::Vulkan);
        assert_eq!(rtx_vk.tier, RtxTier::HardwareRtxPipeline);
        assert!(rtx_vk.supports_inline_ray_query);

        let rtx_cuda = RtxCapabilities::for_backend(wgpu::Backend::Cuda);
        assert_eq!(rtx_cuda.tier, RtxTier::HardwareRtxPipeline);

        let rtx_shader = generate_wgsl_rtx_raytracing_shader(8, 8);
        let rtx_artifact =
            compile_shader_multi_target(&rtx_shader).expect("RTX WGSL compiles to all 6 backends");
        assert!(rtx_artifact.msl.contains("kernel void"));
        assert!(rtx_artifact.glsl.contains("layout(local_size_x = 8, local_size_y = 8"));
    }
}
