use crate::GpuContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    Noop,
    Vulkan,
    Metal,
    Dx12,
    Gl,
    WebGpu,
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
        }
    }
}

pub const fn backend_label(backend: wgpu::Backend) -> &'static str {
    BackendReport::from_wgpu(backend).label
}

/// Probes the preferred portable compute backend without constructing a
/// domain-specific kernel pipeline.
pub fn available_backend_report() -> Option<BackendReport> {
    GpuContext::new().map(|context| context.backend_report())
}

/// Reports native WGSL subgroup support for future kernels that can preserve
/// their tie-breaking semantics with subgroup reductions.
pub fn available_subgroup_report() -> Option<SubgroupReport> {
    GpuContext::new().map(|context| context.subgroup_report())
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
}
