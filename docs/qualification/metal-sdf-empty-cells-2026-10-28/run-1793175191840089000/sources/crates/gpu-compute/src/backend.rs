use crate::GpuContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    Noop,
    Vulkan,
    Metal,
    Dx12,
    Gl,
    WebGpu,
    Cuda,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackendReport {
    pub kind: BackendKind,
    pub label: &'static str,
    pub native_api: bool,
    pub browser_api: bool,
    pub metal: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubgroupReport {
    pub supported: bool,
    pub min_size: u32,
    pub max_size: u32,
}

/// Physical and logical GPU memory architecture:
/// - [`MemoryArchitecture::Uma`] (Unified Memory Architecture, e.g. Apple Silicon Metal):
///   CPU and GPU share a single coherent physical DRAM pool (`StorageModeShared`).
///   Host-to-device uploads map primary buffers directly (`MAP_WRITE | STORAGE`) with zero PCIe staging copies.
/// - [`MemoryArchitecture::Duma`] (Discrete / Decoupled Memory Architecture, e.g. NVIDIA CUDA / PCIe Vulkan / DX12 / WebGPU):
///   CPU RAM and GPU VRAM are physically or logically separated across a bus/staging boundary.
///   Uploads use explicit staging ring buffers (`queue.write_buffer` / `cudaMemcpyAsync`) and device-local VRAM (`StorageModePrivate`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryArchitecture {
    /// Unified Memory Architecture (Apple Silicon Metal `StorageModeShared`, zero-copy CPU/GPU DRAM).
    Uma,
    /// Discrete / Decoupled Memory Architecture (dedicated VRAM + PCIe/staging transfer).
    Duma,
    /// Accelerated Storage Direct / Apple Silicon Direct / GPUDirect Storage (`newBufferWithBytesNoCopy:` / `cuFile`).
    Asd,
}

/// Strategy for uploading, storing, and reading back GPU buffers under [`MemoryArchitecture::Uma`], [`MemoryArchitecture::Duma`], and [`MemoryArchitecture::Asd`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryArchitectureProfile {
    pub architecture: MemoryArchitecture,
    pub label: &'static str,
    pub unified_physical_ram: bool,
    pub zero_copy_primary_upload: bool,
    pub requires_explicit_staging_ring: bool,
    pub preferred_upload_strategy: &'static str,
    pub preferred_readback_strategy: &'static str,
}

impl MemoryArchitecture {
    pub const fn profile(self) -> MemoryArchitectureProfile {
        match self {
            Self::Uma => MemoryArchitectureProfile {
                architecture: Self::Uma,
                label: "UMA (Unified Memory Architecture)",
                unified_physical_ram: true,
                zero_copy_primary_upload: true,
                requires_explicit_staging_ring: false,
                preferred_upload_strategy: "direct_mapped_primary_buffer",
                preferred_readback_strategy: "shared_coherent_map_read",
            },
            Self::Duma => MemoryArchitectureProfile {
                architecture: Self::Duma,
                label: "dUMA (Discrete Memory Architecture)",
                unified_physical_ram: false,
                zero_copy_primary_upload: false,
                requires_explicit_staging_ring: true,
                preferred_upload_strategy: "staging_ring_to_device_local_vram",
                preferred_readback_strategy: "dma_copy_to_staging_then_map",
            },
            Self::Asd => MemoryArchitectureProfile {
                architecture: Self::Asd,
                label: "ASD (Accelerated Storage Direct / Zero-Copy Page Mapping)",
                unified_physical_ram: true,
                zero_copy_primary_upload: true,
                requires_explicit_staging_ring: false,
                preferred_upload_strategy: "page_aligned_no_copy_direct_storage",
                preferred_readback_strategy: "zero_copy_page_mapped_view",
            },
        }
    }
}

/// Hardware execution topology across the 5 fundamental GPU architecture primitives
/// plus the memory architecture (`UMA` vs `dUMA`):
/// 1. Scalar execution unit (`ALU / Execution Pipeline` on Metal vs `CUDA Core / Streaming Processor`)
/// 2. 32-thread lockstep execution unit (`SIMD-group (32 threads)` on Metal vs `Warp (32 threads)`)
/// 3. Thread block / Threadgroup (`max_threads_per_block` up to 1024)
/// 4. On-chip programmable SRAM (`threadgroup memory` 32 KB vs `__shared__ memory` 48 KB)
/// 5. Matrix Coprocessor (`simdgroup_matrix (8x8)` on Apple Silicon vs `Tensor Cores (wmma 16x16)`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwareExecutionTopology {
    pub memory_architecture: MemoryArchitecture,
    pub scalar_unit_label: &'static str,
    pub simd_width: u32,
    pub simd_group_label: &'static str,
    pub max_threads_per_block: u32,
    pub local_sram_label: &'static str,
    pub default_local_sram_bytes: u32,
    pub matrix_coprocessor_label: &'static str,
    pub matrix_tile_shape: [u32; 3],
    pub optimal_1d_workgroup_size: u32,
    pub optimal_2d_tile_size: [u32; 2],
}

impl HardwareExecutionTopology {
    /// Estimates matrix-multiply occupancy `(tile_count, total_simd_groups, local_sram_bytes_per_block)`
    /// for a product `[rows, inner] x [inner, cols] -> [rows, cols]`, mapping onto 32-thread
    /// Warps/SIMD-groups, threadgroups, and `simdgroup_matrix` (8x8) / Tensor Core (16x16) tiles.
    pub const fn estimate_matmul_occupancy(
        &self,
        rows: u32,
        inner: u32,
        cols: u32,
    ) -> (u32, u32, u32) {
        if rows == 0 || inner == 0 || cols == 0 {
            return (0, 0, 0);
        }
        let tm = if self.matrix_tile_shape[0] > 0 {
            self.matrix_tile_shape[0]
        } else {
            8
        };
        let tn = if self.matrix_tile_shape[1] > 0 {
            self.matrix_tile_shape[1]
        } else {
            8
        };
        let tk = if self.matrix_tile_shape[2] > 0 {
            self.matrix_tile_shape[2]
        } else {
            8
        };
        let tiles_m = rows.div_ceil(tm);
        let tiles_n = cols.div_ceil(tn);
        let tile_count = tiles_m.saturating_mul(tiles_n);

        let block_x = if self.optimal_2d_tile_size[0] > 0 {
            self.optimal_2d_tile_size[0]
        } else {
            16
        };
        let block_y = if self.optimal_2d_tile_size[1] > 0 {
            self.optimal_2d_tile_size[1]
        } else {
            16
        };
        let groups_x = cols.div_ceil(block_x);
        let groups_y = rows.div_ceil(block_y);
        let total_blocks = groups_x.saturating_mul(groups_y);
        let threads_per_block = block_x.saturating_mul(block_y);
        let simd_width = if self.simd_width > 0 {
            self.simd_width
        } else {
            32
        };
        let simd_groups_per_block = threads_per_block.div_ceil(simd_width);
        let total_simd_groups = total_blocks.saturating_mul(simd_groups_per_block);

        // Two f32 tiles in local SRAM per threadblock: A_tile (block_y x tk) + B_tile (tk x block_x)
        let sram_floats = block_y
            .saturating_mul(tk)
            .saturating_add(tk.saturating_mul(block_x));
        let raw_sram_bytes = sram_floats.saturating_mul(4);
        let local_sram_bytes_per_block = if raw_sram_bytes > self.default_local_sram_bytes {
            self.default_local_sram_bytes
        } else {
            raw_sram_bytes
        };

        (tile_count, total_simd_groups, local_sram_bytes_per_block)
    }
}

impl BackendReport {
    pub const fn from_wgpu(backend: wgpu::Backend) -> Self {
        match backend {
            wgpu::Backend::Noop => Self {
                kind: BackendKind::Noop,
                label: "noop",
                native_api: false,
                browser_api: false,
                metal: false,
            },
            wgpu::Backend::Vulkan => Self {
                kind: BackendKind::Vulkan,
                label: "vulkan",
                native_api: true,
                browser_api: false,
                metal: false,
            },
            wgpu::Backend::Metal => Self {
                kind: BackendKind::Metal,
                label: "metal",
                native_api: true,
                browser_api: false,
                metal: true,
            },
            wgpu::Backend::Dx12 => Self {
                kind: BackendKind::Dx12,
                label: "dx12",
                native_api: true,
                browser_api: false,
                metal: false,
            },
            wgpu::Backend::Gl => Self {
                kind: BackendKind::Gl,
                label: "gl",
                native_api: true,
                browser_api: false,
                metal: false,
            },
            wgpu::Backend::BrowserWebGpu => Self {
                kind: BackendKind::WebGpu,
                label: "webgpu",
                native_api: false,
                browser_api: true,
                metal: false,
            },
            wgpu::Backend::Cuda => Self {
                kind: BackendKind::Cuda,
                label: "cuda",
                native_api: true,
                browser_api: false,
                metal: false,
            },
        }
    }

    pub const fn memory_architecture(&self) -> MemoryArchitecture {
        match self.kind {
            BackendKind::Metal if cfg!(all(target_os = "macos", target_arch = "aarch64")) => {
                MemoryArchitecture::Uma
            }
            BackendKind::Metal
            | BackendKind::Vulkan
            | BackendKind::Dx12
            | BackendKind::Cuda
            | BackendKind::Gl
            | BackendKind::WebGpu
            | BackendKind::Noop => MemoryArchitecture::Duma,
        }
    }

    pub const fn memory_profile(&self) -> MemoryArchitectureProfile {
        self.memory_architecture().profile()
    }

    pub const fn execution_topology(&self) -> HardwareExecutionTopology {
        match self.kind {
            BackendKind::Metal => HardwareExecutionTopology {
                memory_architecture: self.memory_architecture(),
                scalar_unit_label: "ALU / Execution Pipeline",
                simd_width: 32,
                simd_group_label: "SIMD-group (32 threads)",
                max_threads_per_block: 1024,
                local_sram_label: "threadgroup memory",
                default_local_sram_bytes: 32768,
                matrix_coprocessor_label: "simdgroup_matrix (8x8)",
                matrix_tile_shape: [8, 8, 8],
                optimal_1d_workgroup_size: 128,
                optimal_2d_tile_size: [32, 8],
            },
            BackendKind::Vulkan | BackendKind::Dx12 | BackendKind::Cuda => HardwareExecutionTopology {
                memory_architecture: MemoryArchitecture::Duma,
                scalar_unit_label: "CUDA Core / Streaming Processor",
                simd_width: 32,
                simd_group_label: "Warp (32 threads)",
                max_threads_per_block: 1024,
                local_sram_label: "__shared__ memory",
                default_local_sram_bytes: 49152,
                matrix_coprocessor_label: "Tensor Cores (wmma 16x16)",
                matrix_tile_shape: [16, 16, 16],
                optimal_1d_workgroup_size: 256,
                optimal_2d_tile_size: [16, 16],
            },
            BackendKind::WebGpu | BackendKind::Gl | BackendKind::Noop => HardwareExecutionTopology {
                memory_architecture: MemoryArchitecture::Duma,
                scalar_unit_label: "CUDA Core / Streaming Processor",
                simd_width: 32,
                simd_group_label: "Warp (32 threads)",
                max_threads_per_block: 1024,
                local_sram_label: "__shared__ memory",
                default_local_sram_bytes: 32768,
                matrix_coprocessor_label: "Tensor Cores (wmma 16x16)",
                matrix_tile_shape: [16, 16, 16],
                optimal_1d_workgroup_size: 256,
                optimal_2d_tile_size: [16, 16],
            },
        }
    }
}

pub const fn backend_label(backend: wgpu::Backend) -> &'static str {
    BackendReport::from_wgpu(backend).label
}

/// Probes the preferred portable compute backend without constructing a
/// domain-specific kernel pipeline.
pub fn available_backend_report() -> Option<BackendReport> {
    static REPORT: std::sync::OnceLock<Option<BackendReport>> = std::sync::OnceLock::new();
    *REPORT.get_or_init(|| GpuContext::new().map(|context| context.backend_report()))
}

/// Reports native WGSL subgroup support for future kernels that can preserve
/// their tie-breaking semantics with subgroup reductions.
pub fn available_subgroup_report() -> Option<SubgroupReport> {
    static REPORT: std::sync::OnceLock<Option<SubgroupReport>> = std::sync::OnceLock::new();
    *REPORT.get_or_init(|| GpuContext::new().map(|context| context.subgroup_report()))
}

/// Probes the active GPU backend's 5-level hardware execution topology.
pub fn available_execution_topology() -> Option<HardwareExecutionTopology> {
    available_backend_report().map(|report| report.execution_topology())
}

/// Recommends a 2D workgroup shape `([wg_x, wg_y], [groups_x, groups_y])` aligned to the
/// 32-thread Warp/SIMD-group for a `width x height` 2D grid.
pub const fn recommend_workgroup_shape_2d(
    backend: wgpu::Backend,
    width: u32,
    height: u32,
) -> ([u32; 2], [u32; 2]) {
    let topo = BackendReport::from_wgpu(backend).execution_topology();
    let tile = topo.optimal_2d_tile_size;
    let gx = if width == 0 {
        0
    } else {
        width.div_ceil(tile[0])
    };
    let gy = if height == 0 {
        0
    } else {
        height.div_ceil(tile[1])
    };
    (tile, [gx, gy])
}

/// Per-backend workgroup-size tuning for compute kernels that template their
/// WGSL source with a `WG` constant. Apple GPUs (Metal) are tile-based
/// deferred renderers whose occupancy is limited by threadgroup memory and
/// per-core execution-unit count; smaller workgroups (aligned to the 32-wide
/// SIMD-group) keep more threadgroups resident and in flight. NVIDIA GPUs —
/// which wgpu drives through Vulkan/DX12 (the dedicated CUDA path is
/// `crate::cuda`) — have deep multi-warp schedulers per SM and benefit
/// from larger workgroups that hide memory latency with more warps in
/// flight, so every non-Metal backend keeps the larger default.
///
/// `metal_size` and `default_size` should both be powers of two; callers
/// that reduce across the workgroup (e.g. tree reductions halving the
/// stride) depend on that.
pub fn tuned_workgroup_size(backend: wgpu::Backend, metal_size: u32, default_size: u32) -> u32 {
    match backend {
        wgpu::Backend::Metal => metal_size,
        _ => default_size,
    }
}

/// Generates a complete, valid WGSL `@compute @workgroup_size(tile_size, tile_size, 1)` shared-memory
/// tiled matrix multiplication shader (`C[M, N] = A[M, K] * B[K, N]`).
///
/// Uses `var<workgroup> tile_a` and `var<workgroup> tile_b` on-chip SRAM tiles (mapping directly
/// to Apple Silicon Metal `threadgroup` memory and NVIDIA CUDA `__shared__` memory), `workgroupBarrier()`
/// synchronization between tile loads and computation, and `fma()` inner loops.
pub fn generate_wgsl_tiled_matmul_shader(tile_size: u32) -> String {
    let ts = tile_size.clamp(1, 32);
    let tile_elems = ts * ts;
    format!(
        r#"struct MatmulParams {{
    m: u32,
    k: u32,
    n: u32,
    _pad: u32,
}}

@group(0) @binding(0) var<uniform> params: MatmulParams;
@group(0) @binding(1) var<storage, read> matrix_a: array<f32>;
@group(0) @binding(2) var<storage, read> matrix_b: array<f32>;
@group(0) @binding(3) var<storage, read_write> matrix_c: array<f32>;

const TILE_SIZE: u32 = {ts}u;
var<workgroup> tile_a: array<f32, {tile_elems}>;
var<workgroup> tile_b: array<f32, {tile_elems}>;

@compute @workgroup_size({ts}, {ts}, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {{
    let row = global_id.y;
    let col = global_id.x;
    let local_row = local_id.y;
    let local_col = local_id.x;
    let local_idx = local_row * TILE_SIZE + local_col;

    var acc: f32 = 0.0;
    let num_tiles = (params.k + TILE_SIZE - 1u) / TILE_SIZE;

    for (var t: u32 = 0u; t < num_tiles; t = t + 1u) {{
        let a_col = t * TILE_SIZE + local_col;
        if (row < params.m && a_col < params.k) {{
            tile_a[local_idx] = matrix_a[row * params.k + a_col];
        }} else {{
            tile_a[local_idx] = 0.0;
        }}

        let b_row = t * TILE_SIZE + local_row;
        if (b_row < params.k && col < params.n) {{
            tile_b[local_idx] = matrix_b[b_row * params.n + col];
        }} else {{
            tile_b[local_idx] = 0.0;
        }}

        workgroupBarrier();

        for (var i: u32 = 0u; i < TILE_SIZE; i = i + 1u) {{
            acc = fma(tile_a[local_row * TILE_SIZE + i], tile_b[i * TILE_SIZE + local_col], acc);
        }}

        workgroupBarrier();
    }}

    if (row < params.m && col < params.n) {{
        matrix_c[row * params.n + col] = acc;
    }}
}}
"#
    )
}

/// Generates a complete, valid WGSL `@compute @workgroup_size(workgroup_size, 1, 1)` tree-based
/// shared-memory parallel reduction shader with bank-conflict-free sequential addressing.
pub fn generate_wgsl_parallel_reduction_shader(workgroup_size: u32) -> String {
    let wg = workgroup_size.clamp(1, 1024).next_power_of_two().min(1024);
    format!(
        r#"struct ReductionParams {{
    len: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}}

@group(0) @binding(0) var<uniform> params: ReductionParams;
@group(0) @binding(1) var<storage, read> input_data: array<f32>;
@group(0) @binding(2) var<storage, read_write> block_sums: array<f32>;

const WG_SIZE: u32 = {wg}u;
var<workgroup> shared_data: array<f32, {wg}>;

@compute @workgroup_size({wg}, 1, 1)
fn main(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>,
) {{
    let tid = local_id.x;
    let base = group_id.x * (WG_SIZE * 2u) + tid;

    var sum: f32 = 0.0;
    if (base < params.len) {{
        sum = input_data[base];
    }}
    if (base + WG_SIZE < params.len) {{
        sum = sum + input_data[base + WG_SIZE];
    }}
    shared_data[tid] = sum;
    workgroupBarrier();

    var stride: u32 = WG_SIZE / 2u;
    loop {{
        if (stride == 0u) {{
            break;
        }}
        if (tid < stride) {{
            shared_data[tid] = shared_data[tid] + shared_data[tid + stride];
        }}
        workgroupBarrier();
        stride = stride / 2u;
    }}

    if (tid == 0u) {{
        block_sums[group_id.x] = shared_data[0u];
    }}
}}
"#
    )
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn tuned_workgroup_size_selects_metal_variant() {
        assert_eq!(tuned_workgroup_size(wgpu::Backend::Metal, 128, 256), 128);
        assert_eq!(backend_label(wgpu::Backend::Metal), "metal");
        assert_eq!(
            BackendReport::from_wgpu(wgpu::Backend::Metal),
            BackendReport {
                kind: BackendKind::Metal,
                label: "metal",
                native_api: true,
                browser_api: false,
                metal: true,
            }
        );
    }

    #[test]
    fn tuned_workgroup_size_falls_back_to_default_off_metal() {
        assert_eq!(tuned_workgroup_size(wgpu::Backend::Vulkan, 128, 256), 256);
        assert_eq!(tuned_workgroup_size(wgpu::Backend::Dx12, 128, 256), 256);
        assert_eq!(tuned_workgroup_size(wgpu::Backend::Gl, 128, 256), 256);
        assert_eq!(backend_label(wgpu::Backend::Vulkan), "vulkan");
        assert_eq!(backend_label(wgpu::Backend::Dx12), "dx12");
        assert_eq!(backend_label(wgpu::Backend::BrowserWebGpu), "webgpu");
        assert!(BackendReport::from_wgpu(wgpu::Backend::Vulkan).native_api);
        assert!(!BackendReport::from_wgpu(wgpu::Backend::Vulkan).browser_api);
        assert!(BackendReport::from_wgpu(wgpu::Backend::BrowserWebGpu).browser_api);
        assert!(!BackendReport::from_wgpu(wgpu::Backend::Noop).native_api);
    }

    #[test]
    fn available_backend_report_is_well_formed_when_present() {
        if let Some(report) = available_backend_report() {
            assert!(!report.label.is_empty());
            assert_eq!(
                report.label,
                backend_label(match report.kind {
                    BackendKind::Noop => wgpu::Backend::Noop,
                    BackendKind::Vulkan => wgpu::Backend::Vulkan,
                    BackendKind::Metal => wgpu::Backend::Metal,
                    BackendKind::Dx12 => wgpu::Backend::Dx12,
                    BackendKind::Gl => wgpu::Backend::Gl,
                    BackendKind::WebGpu => wgpu::Backend::BrowserWebGpu,
                    BackendKind::Cuda => wgpu::Backend::Cuda,
                })
            );
        }
    }

    #[test]
    fn available_subgroup_report_is_well_formed_when_present() {
        if let Some(report) = available_subgroup_report() {
            assert!(report.min_size <= report.max_size);
            if report.supported {
                assert!(report.min_size > 0);
            }
        }
    }

    #[test]
    fn execution_topology_and_2d_workgroup_recommendations_match_hardware_primitives() {
        let metal_topo = BackendReport::from_wgpu(wgpu::Backend::Metal).execution_topology();
        assert_eq!(metal_topo.memory_architecture, MemoryArchitecture::Uma);
        assert!(metal_topo.memory_architecture.profile().unified_physical_ram);
        assert!(metal_topo.memory_architecture.profile().zero_copy_primary_upload);
        assert!(!metal_topo.memory_architecture.profile().requires_explicit_staging_ring);
        assert_eq!(metal_topo.scalar_unit_label, "ALU / Execution Pipeline");
        assert_eq!(metal_topo.simd_width, 32);
        assert_eq!(metal_topo.simd_group_label, "SIMD-group (32 threads)");
        assert_eq!(metal_topo.max_threads_per_block, 1024);
        assert_eq!(metal_topo.local_sram_label, "threadgroup memory");
        assert_eq!(metal_topo.default_local_sram_bytes, 32768);
        assert_eq!(metal_topo.matrix_coprocessor_label, "simdgroup_matrix (8x8)");
        assert_eq!(metal_topo.matrix_tile_shape, [8, 8, 8]);
        assert_eq!(metal_topo.optimal_1d_workgroup_size, 128);
        assert_eq!(metal_topo.optimal_2d_tile_size, [32, 8]);

        let vk_topo = BackendReport::from_wgpu(wgpu::Backend::Vulkan).execution_topology();
        assert_eq!(vk_topo.memory_architecture, MemoryArchitecture::Duma);
        assert!(!vk_topo.memory_architecture.profile().unified_physical_ram);
        assert!(!vk_topo.memory_architecture.profile().zero_copy_primary_upload);
        assert!(vk_topo.memory_architecture.profile().requires_explicit_staging_ring);
        assert_eq!(vk_topo.scalar_unit_label, "CUDA Core / Streaming Processor");
        assert_eq!(vk_topo.simd_width, 32);
        assert_eq!(vk_topo.simd_group_label, "Warp (32 threads)");
        assert_eq!(vk_topo.local_sram_label, "__shared__ memory");
        assert_eq!(vk_topo.default_local_sram_bytes, 49152);
        assert_eq!(vk_topo.matrix_coprocessor_label, "Tensor Cores (wmma 16x16)");
        assert_eq!(vk_topo.matrix_tile_shape, [16, 16, 16]);
        assert_eq!(vk_topo.optimal_1d_workgroup_size, 256);
        assert_eq!(vk_topo.optimal_2d_tile_size, [16, 16]);

        let (tile_m, groups_m) = recommend_workgroup_shape_2d(wgpu::Backend::Metal, 100, 50);
        assert_eq!(tile_m, [32, 8]);
        assert_eq!(groups_m, [4, 7]);
        assert_eq!((tile_m[0] * tile_m[1]) % 32, 0);

        let (tile_v, groups_v) = recommend_workgroup_shape_2d(wgpu::Backend::Vulkan, 100, 50);
        assert_eq!(tile_v, [16, 16]);
        assert_eq!(groups_v, [7, 4]);
        assert_eq!((tile_v[0] * tile_v[1]) % 32, 0);

        if let Some(active) = available_execution_topology() {
            assert_eq!(active.simd_width, 32);
            assert_eq!(active.max_threads_per_block, 1024);
        }

        let (m_tiles, m_simds, m_sram) = metal_topo.estimate_matmul_occupancy(64, 64, 64);
        assert_eq!(m_tiles, 64); // (64/8) * (64/8) = 64 8x8 simdgroup_matrix tiles
        assert!(m_simds > 0);
        assert!(m_sram > 0 && m_sram <= metal_topo.default_local_sram_bytes);

        let (v_tiles, v_simds, v_sram) = vk_topo.estimate_matmul_occupancy(64, 64, 64);
        assert_eq!(v_tiles, 16); // (64/16) * (64/16) = 16 16x16 Tensor Core tiles
        assert!(v_simds > 0);
        assert!(v_sram > 0 && v_sram <= vk_topo.default_local_sram_bytes);
        assert_eq!(metal_topo.estimate_matmul_occupancy(0, 64, 64), (0, 0, 0));
    }

    #[test]
    fn wgsl_tiled_matmul_and_parallel_reduction_shader_generators_emit_valid_kernels() {
        let matmul_wgsl = generate_wgsl_tiled_matmul_shader(16);
        assert!(matmul_wgsl.contains("@compute @workgroup_size(16, 16, 1)"));
        assert!(matmul_wgsl.contains("var<workgroup> tile_a: array<f32, 256>;"));
        assert!(matmul_wgsl.contains("var<workgroup> tile_b: array<f32, 256>;"));
        assert!(matmul_wgsl.contains("workgroupBarrier();"));
        assert!(matmul_wgsl.contains("fma("));

        let reduce_wgsl = generate_wgsl_parallel_reduction_shader(256);
        assert!(reduce_wgsl.contains("@compute @workgroup_size(256, 1, 1)"));
        assert!(reduce_wgsl.contains("var<workgroup> shared_data: array<f32, 256>;"));
        assert!(reduce_wgsl.contains("workgroupBarrier();"));
        assert!(reduce_wgsl.contains("shared_data[tid] + shared_data[tid + stride]"));
    }
}
