//! **UMA** (Unified Memory Architecture), **dUMA** (Discrete / Decoupled Unified Memory Architecture),
//! and orthogonal **ASD** (Accelerated Storage Direct) hardware topology & transfer planners.
//!
//! Re-exports the single-source-of-truth hardware topology types from [`wgpu::platform`] (DRY principle)
//! and provides higher-level memory transfer planning across physical topology (`UMA` vs `dUMA`)
//! combined with storage/page direct I/O (`StorageDirectCapability` / `ASD`).

pub use wgpu::platform::{
    backend_label, recommend_workgroup_shape_2d, tuned_workgroup_size, BackendKind, BackendReport,
    HardwareExecutionTopology, MemoryArchitecture, MemoryArchitectureProfile,
    StorageDirectCapability, SubgroupReport, TargetPlatform,
};

/// Transfer mode chosen for a specific buffer/storage operation, combining the physical
/// [`MemoryArchitecture`] (`Uma` vs `Duma`) with optional [`StorageDirectCapability`] (`ASD`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModernMemoryTech {
    /// **ASD** (Accelerated Storage Direct / Apple Silicon Direct / GPUDirect Storage / DirectStorage / `VK_EXT_external_memory_host`):
    /// Zero-copy page-aligned storage or `mmap` mapped directly into GPU virtual address space (works on top of both `UMA` and `dUMA`).
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
            MemoryArchitecture::Uma => Self::Uma,
            MemoryArchitecture::Duma => Self::Duma,
        }
    }
}

/// Transfer planner selecting the fastest path by combining the adapter's physical [`MemoryArchitecture`]
/// (`Uma` vs `Duma`) with its orthogonal [`StorageDirectCapability`] (`ASD`) for a given payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnifiedTransferPlan {
    /// Physical RAM/VRAM topology of the adapter (`Uma` or `Duma`).
    pub memory_architecture: MemoryArchitecture,
    /// Orthogonal storage-direct / page-import capability of the backend (`MetalPageNoCopy`, `VulkanExternalMemoryHost`, `NvidiaGpuDirectStorage`, `MicrosoftDirectStorage`, or `None`).
    pub storage_direct: StorageDirectCapability,
    /// Effective transfer technology selected for this payload.
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
        let memory_architecture = report.memory_architecture();
        let storage_direct = report.storage_direct_capability();

        // If the payload is page-aligned and the backend supports ASD (on either UMA or dUMA), use ASD zero-copy page mapping!
        if page_aligned && !matches!(storage_direct, StorageDirectCapability::None) {
            return Self {
                memory_architecture,
                storage_direct,
                tech: ModernMemoryTech::Asd,
                zero_copy_cpu_write: true,
                zero_copy_gpu_read: true,
                uses_page_aligned_no_copy: true,
                uses_rebar_aperture: false,
                staging_ring_chunks: 0,
            };
        }

        match memory_architecture {
            MemoryArchitecture::Uma => Self {
                memory_architecture,
                storage_direct,
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
                    memory_architecture,
                    storage_direct,
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
