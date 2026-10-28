//! **UMA** (Unified Memory Architecture), **dUMA** (Discrete / Decoupled Unified Memory Architecture),
//! and **ASD** (Accelerated Storage Direct) hardware topology & transfer planners.
//!
//! Re-exports the single-source-of-truth hardware topology types from [`wgpu::platform`] (DRY principle)
//! and provides higher-level memory transfer planning across UMA, dUMA (ReBAR / SAM + pinned DMA), and ASD.

pub use wgpu::platform::{
    backend_label, recommend_workgroup_shape_2d, tuned_workgroup_size, BackendKind, BackendReport,
    HardwareExecutionTopology, MemoryArchitecture, MemoryArchitectureProfile, SubgroupReport,
    TargetPlatform,
};

/// Modern GPU Storage & Memory Architecture modes: **ASD**, **UMA**, and **dUMA**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModernMemoryTech {
    /// **ASD** (Accelerated Storage Direct / Apple Silicon Direct / GPUDirect Storage / DirectStorage):
    /// Zero-copy page-aligned storage / mmap directly mapped into GPU virtual address space.
    Asd,
    /// **UMA** (Unified Memory Architecture):
    /// Shared physical DRAM between CPU and GPU cores with coherent cache / zero staging copies.
    Uma,
    /// **dUMA** (Discrete / Decoupled Unified Memory Architecture):
    /// Discrete VRAM coupled with Resizable BAR (ReBAR / SAM) direct host-visible aperture and async DMA ring buffers.
    Duma,
}

impl From<MemoryArchitecture> for ModernMemoryTech {
    #[inline]
    fn from(arch: MemoryArchitecture) -> Self {
        match arch {
            MemoryArchitecture::Asd => Self::Asd,
            MemoryArchitecture::Uma => Self::Uma,
            MemoryArchitecture::Duma => Self::Duma,
        }
    }
}

impl From<ModernMemoryTech> for MemoryArchitecture {
    #[inline]
    fn from(tech: ModernMemoryTech) -> Self {
        match tech {
            ModernMemoryTech::Asd => Self::Asd,
            ModernMemoryTech::Uma => Self::Uma,
            ModernMemoryTech::Duma => Self::Duma,
        }
    }
}

/// Transfer planner selecting the fastest path among **ASD**, **UMA**, and **dUMA** for a given payload size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnifiedTransferPlan {
    pub tech: ModernMemoryTech,
    pub zero_copy_cpu_write: bool,
    pub zero_copy_gpu_read: bool,
    pub uses_page_aligned_no_copy: bool,
    pub uses_rebar_aperture: bool,
    pub staging_ring_chunks: u32,
}

impl UnifiedTransferPlan {
    /// Computes the optimal transfer plan for `byte_len` bytes on the given `backend`.
    pub const fn for_payload(backend: wgpu::Backend, byte_len: u64, page_aligned: bool) -> Self {
        let report = BackendReport::from_wgpu(backend);
        let arch = report.memory_architecture();
        if page_aligned && matches!(arch, MemoryArchitecture::Uma | MemoryArchitecture::Asd) {
            return Self {
                tech: ModernMemoryTech::Asd,
                zero_copy_cpu_write: true,
                zero_copy_gpu_read: true,
                uses_page_aligned_no_copy: true,
                uses_rebar_aperture: false,
                staging_ring_chunks: 0,
            };
        }
        match arch {
            MemoryArchitecture::Uma | MemoryArchitecture::Asd => Self {
                tech: ModernMemoryTech::Uma,
                zero_copy_cpu_write: true,
                zero_copy_gpu_read: true,
                uses_page_aligned_no_copy: false,
                uses_rebar_aperture: false,
                staging_ring_chunks: 0,
            },
            MemoryArchitecture::Duma => {
                // On discrete GPUs, payloads <= 16 MiB fit directly inside the ReBAR / SAM host-visible VRAM window;
                // larger payloads stream through 4-way double-buffered pinned DMA staging chunks.
                let fits_rebar = byte_len <= 16 * 1024 * 1024;
                Self {
                    tech: ModernMemoryTech::Duma,
                    zero_copy_cpu_write: fits_rebar,
                    zero_copy_gpu_read: true,
                    uses_page_aligned_no_copy: false,
                    uses_rebar_aperture: fits_rebar,
                    staging_ring_chunks: if fits_rebar { 1 } else { 4 },
                }
            }
        }
    }
}
