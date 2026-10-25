//! GPU descriptor matching (feature `gpu`). One thread per candidate pair
//! computes the full 128-component squared distance; workgroup reductions keep
//! the CPU scan's ordering semantics: smaller distance wins, equal distances
//! keep the first index (the CPU scan uses strict `<`). NaN distances never
//! win, as on the CPU. GPU floating-point contraction (fma) means results are
//! NOT bit-identical to the CPU path — this mode is opt-in and qualified
//! separately; CPU remains the deterministic reference.

use super::wgpu;

use super::GpuContext;
use compute_core::{Binding, Kernel};
use gpu_compute::MemoryArchitecture;

/// Apple-silicon workgroup size, measured on an M4 Max (see
/// `math-core/examples/bench_workgroup_tuning.rs`): streaming reductions such
/// as point_bounds/chamfer scale with the workgroup size, and 256 matches or
/// beats 128 at every size tested while 512 adds nothing beyond noise. Every
/// other backend keeps the larger default anyway. See
/// `gpu_compute::tuned_workgroup_size`.
const WG_METAL: u32 = 256;
const WG_DEFAULT: u32 = 256;

/// The qualified matcher template lives in `host_matching` so the browser
/// host-GPU path compiles exactly the shader the native `gpu` feature does
/// (the native kernels use the `match_rows` / `match_cols` entries with zero
/// descriptor offsets; the browser uses `match_pair` over a shared table).
use crate::host_matching::MATCH_WGSL as SHADER_TEMPLATE;

#[derive(Clone, Copy, Debug)]
pub struct RowBest {
    pub j: usize,
    pub d1: f32,
    pub d2: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct ColBest {
    pub i: usize,
    pub d1: f32,
}

struct Buffers {
    row_capacity: usize,
    col_capacity: usize,
    params: wgpu::Buffer,
    descriptors_a: wgpu::Buffer,
    descriptors_b: wgpu::Buffer,
    rows_out: wgpu::Buffer,
    cols_out: wgpu::Buffer,
    /// Present only under `MemoryArchitecture::Duma`; `None` under `MemoryArchitecture::Uma`
    /// where `rows_out` / `cols_out` carry `STORAGE | MAP_READ` in unified RAM.
    read_rows: Option<wgpu::Buffer>,
    read_cols: Option<wgpu::Buffer>,
    bind: wgpu::BindGroup,
}

pub struct GpuMatcher {
    device: wgpu::Device,
    queue: wgpu::Queue,
    memory_architecture: MemoryArchitecture,
    rows_kernel: Kernel,
    cols_kernel: Kernel,
    /// The workgroup size baked into the compiled pipelines, chosen per
    /// backend by `gpu_compute::tuned_workgroup_size`. Exposed for tests
    /// (hence `allow(dead_code)` on non-test builds).
    #[allow(dead_code)]
    workgroup_size: u32,
    buffers: std::cell::RefCell<Option<Buffers>>,
}

impl GpuMatcher {
    pub fn new(context: &GpuContext) -> Self {
        let workgroup_size =
            gpu_compute::tuned_workgroup_size(context.backend, WG_METAL, WG_DEFAULT);
        Self::with_workgroup_size(context, workgroup_size)
    }

    pub fn memory_architecture(&self) -> MemoryArchitecture {
        self.memory_architecture
    }

    /// Builds the pipelines for an explicit workgroup size, bypassing backend
    /// auto-detection. `new` is the production entry point; this is what lets
    /// tests exercise both the Metal-tuned and default-tuned kernel variants
    /// on whatever adapter the test machine actually has.
    fn with_workgroup_size(context: &GpuContext, workgroup_size: u32) -> Self {
        let bindings = [
            Binding::Uniform,
            Binding::StorageRead,
            Binding::StorageRead,
            Binding::StorageReadWrite,
            Binding::StorageReadWrite,
        ];
        let rows_kernel = Kernel::with_workgroup_size(
            &context.device,
            "match_rows",
            SHADER_TEMPLATE,
            "match_rows",
            &bindings,
            workgroup_size,
        )
        .expect("match_rows kernel builds");
        let cols_kernel = Kernel::with_workgroup_size(
            &context.device,
            "match_cols",
            SHADER_TEMPLATE,
            "match_cols",
            &bindings,
            workgroup_size,
        )
        .expect("match_cols kernel builds");
        Self {
            device: context.device.clone(),
            queue: context.queue.clone(),
            memory_architecture: context.memory_architecture(),
            rows_kernel,
            cols_kernel,
            workgroup_size,
            buffers: std::cell::RefCell::new(None),
        }
    }

    #[cfg(test)]
    fn workgroup_size(&self) -> u32 {
        self.workgroup_size
    }

    /// Nearest/second-nearest per row and nearest per column over the full
    /// descriptor grid. Mirrors the CPU scan's tie-breaking; float contraction
    /// may shift borderline distances, so GPU results are not bit-identical.
    pub fn match_descriptors(
        &self,
        da: &[[f32; 128]],
        db: &[[f32; 128]],
    ) -> (Vec<RowBest>, Vec<ColBest>) {
        self.try_match_descriptors(da, db)
            .expect("GPU descriptor readback failed")
    }

    /// Fallible transfer variant for callers that can fall back to CPU matching.
    pub fn try_match_descriptors(
        &self,
        da: &[[f32; 128]],
        db: &[[f32; 128]],
    ) -> Result<(Vec<RowBest>, Vec<ColBest>), gpu_compute::ReadbackError> {
        if da.is_empty() || db.is_empty() {
            // No dispatch overwrites cached outputs for an empty grid. Produce
            // the same missing-neighbor values as CPU and CUDA instead of
            // reading results left by an earlier nonempty call.
            return Ok((
                vec![
                    RowBest {
                        j: usize::MAX,
                        d1: f32::INFINITY,
                        d2: f32::INFINITY
                    };
                    da.len()
                ],
                vec![
                    ColBest {
                        i: usize::MAX,
                        d1: f32::INFINITY
                    };
                    db.len()
                ],
            ));
        }
        let rows = da.len() as u32;
        let cols = db.len() as u32;
        let packed_a = gpu_compute::f32_le_bytes(da.as_flattened());
        let packed_b = gpu_compute::f32_le_bytes(db.as_flattened());
        self.ensure_buffers(da.len(), db.len());
        let row_bytes = (rows as u64) * 12;
        let col_bytes = (cols as u64) * 8;
        let buffers = self.buffers.borrow();
        let buffers = buffers.as_ref().expect("ensure_buffers was just called");
        let mut params_bytes = [0u8; 16];
        params_bytes[0..4].copy_from_slice(&rows.to_ne_bytes());
        params_bytes[4..8].copy_from_slice(&cols.to_ne_bytes());
        self.queue.write_buffer(&buffers.params, 0, &params_bytes);
        if !packed_a.is_empty() {
            self.queue
                .write_buffer(&buffers.descriptors_a, 0, &packed_a);
        }
        if !packed_b.is_empty() {
            self.queue
                .write_buffer(&buffers.descriptors_b, 0, &packed_b);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("match"),
            });
        if rows > 0 && cols > 0 {
            self.rows_kernel
                .record_dispatch(&mut encoder, &buffers.bind, rows);
            self.cols_kernel
                .record_dispatch(&mut encoder, &buffers.bind, cols);
        }
        // Under dUMA, DMA-copy from device-local outputs into host-mappable staging buffers.
        // Under UMA (Apple Silicon Metal StorageModeShared), `rows_out` and `cols_out` are
        // mapped directly in unified memory with zero copy_buffer_to_buffer overhead.
        if let Some(read_rows) = &buffers.read_rows {
            encoder.copy_buffer_to_buffer(
                &buffers.rows_out,
                0,
                read_rows,
                0,
                row_bytes.max(12),
            );
        }
        if let Some(read_cols) = &buffers.read_cols {
            encoder.copy_buffer_to_buffer(
                &buffers.cols_out,
                0,
                read_cols,
                0,
                col_bytes.max(8),
            );
        }
        self.queue.submit([encoder.finish()]);

        // The shared transport releases each mapping before returning, so these
        // allocations can be reused by the next call without unmapping.
        let rows_target = buffers.read_rows.as_ref().unwrap_or(&buffers.rows_out);
        let cols_target = buffers.read_cols.as_ref().unwrap_or(&buffers.cols_out);
        let rows_out_vec = gpu_compute::try_read_buffer_with(
            &self.device,
            rows_target,
            row_bytes as usize,
            |bytes| {
                bytes
                    .chunks_exact(12)
                    .map(|c| RowBest {
                        j: match u32::from_le_bytes(c[0..4].try_into().unwrap()) {
                            u32::MAX => usize::MAX,
                            raw => raw as usize,
                        },
                        d1: f32::from_le_bytes(c[4..8].try_into().unwrap()),
                        d2: f32::from_le_bytes(c[8..12].try_into().unwrap()),
                    })
                    .collect()
            },
        )?;
        let cols_out_vec = gpu_compute::try_read_buffer_with(
            &self.device,
            cols_target,
            col_bytes as usize,
            |bytes| {
                bytes
                    .chunks_exact(8)
                    .map(|c| ColBest {
                        i: match u32::from_le_bytes(c[0..4].try_into().unwrap()) {
                            u32::MAX => usize::MAX,
                            raw => raw as usize,
                        },
                        d1: f32::from_le_bytes(c[4..8].try_into().unwrap()),
                    })
                    .collect()
            },
        )?;
        Ok((rows_out_vec, cols_out_vec))
    }

    fn ensure_buffers(&self, row_count: usize, col_count: usize) {
        let stale = match &*self.buffers.borrow() {
            Some(buffers) => buffers.row_capacity < row_count || buffers.col_capacity < col_count,
            None => true,
        };
        if !stale {
            return;
        }
        let device = &self.device;
        let uma = self.memory_architecture == MemoryArchitecture::Uma;
        let row_capacity = row_count.max(1);
        let col_capacity = col_count.max(1);
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_params"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let descriptors_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_da"),
            size: (row_capacity * 128 * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let descriptors_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_db"),
            size: (col_capacity * 128 * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let out_usage = if uma {
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ
        } else {
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC
        };
        let rows_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_rows_out"),
            size: (row_capacity * 12) as u64,
            usage: out_usage,
            mapped_at_creation: false,
        });
        let cols_out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("match_cols_out"),
            size: (col_capacity * 8) as u64,
            usage: out_usage,
            mapped_at_creation: false,
        });
        let (read_rows, read_cols) = if uma {
            (None, None)
        } else {
            (
                Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("match_read_rows"),
                    size: (row_capacity * 12) as u64,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })),
                Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("match_read_cols"),
                    size: (col_capacity * 8) as u64,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })),
            )
        };
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("match_descriptors"),
            layout: self.rows_kernel.bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: descriptors_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: descriptors_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: rows_out.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: cols_out.as_entire_binding(),
                },
            ],
        });
        *self.buffers.borrow_mut() = Some(Buffers {
            row_capacity,
            col_capacity,
            params,
            descriptors_a,
            descriptors_b,
            rows_out,
            cols_out,
            read_rows,
            read_cols,
            bind,
        });
    }
}

thread_local! {
    // The device and pipelines are process-lifetime resources; leaking avoids
    // dropping wgpu objects inside a thread-local destructor.
    static SHARED: std::cell::LazyCell<Option<&'static (GpuContext, GpuMatcher)>> =
        std::cell::LazyCell::new(|| GpuContext::new().map(|c| {
            let matcher = GpuMatcher::new(&c);
            Box::leak(Box::new((c, matcher))) as &'static (GpuContext, GpuMatcher)
        }));
}

/// Matches one descriptor pair-set through a thread-shared GPU context.
/// `None` when no GPU adapter is available or readback fails; callers fall back
/// to the CPU scan.
pub fn match_pair(da: &[[f32; 128]], db: &[[f32; 128]]) -> Option<(Vec<RowBest>, Vec<ColBest>)> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuMatcher)> = cell;
        shared.and_then(|(_, matcher)| matcher.try_match_descriptors(da, db).ok())
    })
}

pub fn match_pair_accelerated(
    da: &[[f32; 128]],
    db: &[[f32; 128]],
    acceleration: crate::Acceleration,
) -> Option<(Vec<RowBest>, Vec<ColBest>)> {
    #[cfg(not(feature = "cuda"))]
    let _ = acceleration;
    #[cfg(feature = "cuda")]
    if acceleration == crate::Acceleration::Cuda
        && let Some(result) = cuda::match_pair_cuda(da, db)
    {
        return Some(result);
    }
    match_pair(da, db)
}

#[cfg(feature = "cuda")]
mod cuda {
    use super::{ColBest, RowBest};
    use gpu_compute::cuda::{CudaDevice, CudaFunction, CudaSlice, LaunchConfig, PushKernelArg};

    const MATCHING_PTX: &str = include_str!("matching.ptx");
    const BLOCK: u32 = 256;

    struct Buffers {
        row_capacity: usize,
        col_capacity: usize,
        descriptors_a: CudaSlice<f32>,
        descriptors_b: CudaSlice<f32>,
        row_j: CudaSlice<u32>,
        row_d1: CudaSlice<f32>,
        row_d2: CudaSlice<f32>,
        col_i: CudaSlice<u32>,
        col_d1: CudaSlice<f32>,
    }

    struct CudaMatcher {
        device: CudaDevice,
        rows_kernel: CudaFunction,
        cols_kernel: CudaFunction,
        buffers: std::cell::RefCell<Option<Buffers>>,
    }

    impl CudaMatcher {
        fn new() -> Option<Self> {
            let device = CudaDevice::new()?;
            let module = device.load_ptx(MATCHING_PTX)?;
            let rows_kernel = module.load_function("match_descriptor_rows").ok()?;
            let cols_kernel = module.load_function("match_descriptor_cols").ok()?;
            Some(Self {
                device,
                rows_kernel,
                cols_kernel,
                buffers: std::cell::RefCell::new(None),
            })
        }

        fn ensure_buffers(&self, rows: usize, cols: usize) -> Option<()> {
            let stale = match &*self.buffers.borrow() {
                Some(b) => b.row_capacity < rows || b.col_capacity < cols,
                None => true,
            };
            if !stale {
                return Some(());
            }
            let row_capacity = rows.max(1);
            let col_capacity = cols.max(1);
            let stream = &self.device.stream;
            let descriptors_a = stream.alloc_zeros::<f32>(row_capacity * 128).ok()?;
            let descriptors_b = stream.alloc_zeros::<f32>(col_capacity * 128).ok()?;
            let row_j = stream.alloc_zeros::<u32>(row_capacity).ok()?;
            let row_d1 = stream.alloc_zeros::<f32>(row_capacity).ok()?;
            let row_d2 = stream.alloc_zeros::<f32>(row_capacity).ok()?;
            let col_i = stream.alloc_zeros::<u32>(col_capacity).ok()?;
            let col_d1 = stream.alloc_zeros::<f32>(col_capacity).ok()?;
            *self.buffers.borrow_mut() = Some(Buffers {
                row_capacity,
                col_capacity,
                descriptors_a,
                descriptors_b,
                row_j,
                row_d1,
                row_d2,
                col_i,
                col_d1,
            });
            Some(())
        }

        fn run(
            &self,
            da: &[[f32; 128]],
            db: &[[f32; 128]],
        ) -> Option<(Vec<RowBest>, Vec<ColBest>)> {
            let rows = da.len();
            let cols = db.len();
            if rows == 0 || cols == 0 {
                return Some((
                    vec![
                        RowBest {
                            j: usize::MAX,
                            d1: f32::INFINITY,
                            d2: f32::INFINITY,
                        };
                        rows
                    ],
                    vec![
                        ColBest {
                            i: usize::MAX,
                            d1: f32::INFINITY,
                        };
                        cols
                    ],
                ));
            }
            self.ensure_buffers(rows, cols)?;
            let stream = &self.device.stream;
            let mut buffers = self.buffers.borrow_mut();
            let b = buffers.as_mut().expect("ensure_buffers was just called");
            let flat_a: Vec<f32> = da.iter().flatten().copied().collect();
            let flat_b: Vec<f32> = db.iter().flatten().copied().collect();
            {
                let mut view = b.descriptors_a.slice_mut(0..flat_a.len());
                stream.memcpy_htod(&flat_a, &mut view).ok()?;
            }
            {
                let mut view = b.descriptors_b.slice_mut(0..flat_b.len());
                stream.memcpy_htod(&flat_b, &mut view).ok()?;
            }
            let rows_u32 = rows as u32;
            let cols_u32 = cols as u32;
            let a_view = b.descriptors_a.slice(0..flat_a.len().max(1));
            let b_view = b.descriptors_b.slice(0..flat_b.len().max(1));
            let mut row_j_view = b.row_j.slice_mut(0..rows.max(1));
            let mut row_d1_view = b.row_d1.slice_mut(0..rows.max(1));
            let mut row_d2_view = b.row_d2.slice_mut(0..rows.max(1));
            let mut row_launch = stream.launch_builder(&self.rows_kernel);
            row_launch
                .arg(&rows_u32)
                .arg(&cols_u32)
                .arg(&a_view)
                .arg(&b_view)
                .arg(&mut row_j_view)
                .arg(&mut row_d1_view)
                .arg(&mut row_d2_view);
            unsafe { row_launch.launch(launch_1d_blocks(rows_u32)) }.ok()?;
            let mut col_i_view = b.col_i.slice_mut(0..cols.max(1));
            let mut col_d1_view = b.col_d1.slice_mut(0..cols.max(1));
            let mut col_launch = stream.launch_builder(&self.cols_kernel);
            col_launch
                .arg(&rows_u32)
                .arg(&cols_u32)
                .arg(&a_view)
                .arg(&b_view)
                .arg(&mut col_i_view)
                .arg(&mut col_d1_view);
            unsafe { col_launch.launch(launch_1d_blocks(cols_u32)) }.ok()?;

            let mut row_j = vec![0u32; rows];
            let mut row_d1 = vec![0f32; rows];
            let mut row_d2 = vec![0f32; rows];
            let mut col_i = vec![0u32; cols];
            let mut col_d1 = vec![0f32; cols];
            stream.memcpy_dtoh(&row_j_view, &mut row_j).ok()?;
            stream.memcpy_dtoh(&row_d1_view, &mut row_d1).ok()?;
            stream.memcpy_dtoh(&row_d2_view, &mut row_d2).ok()?;
            stream.memcpy_dtoh(&col_i_view, &mut col_i).ok()?;
            stream.memcpy_dtoh(&col_d1_view, &mut col_d1).ok()?;

            Some((
                (0..rows)
                    .map(|i| RowBest {
                        j: if row_j[i] == u32::MAX {
                            usize::MAX
                        } else {
                            row_j[i] as usize
                        },
                        d1: row_d1[i],
                        d2: row_d2[i],
                    })
                    .collect(),
                (0..cols)
                    .map(|i| ColBest {
                        i: if col_i[i] == u32::MAX {
                            usize::MAX
                        } else {
                            col_i[i] as usize
                        },
                        d1: col_d1[i],
                    })
                    .collect(),
            ))
        }
    }

    fn launch_1d_blocks(blocks: u32) -> LaunchConfig {
        LaunchConfig {
            grid_dim: (blocks.max(1), 1, 1),
            block_dim: (BLOCK, 1, 1),
            shared_mem_bytes: 0,
        }
    }

    thread_local! {
        static SHARED: std::cell::LazyCell<Option<&'static CudaMatcher>> =
            std::cell::LazyCell::new(|| {
                CudaMatcher::new().map(|matcher| Box::leak(Box::new(matcher)) as &'static CudaMatcher)
            });
    }

    pub(super) fn match_pair_cuda(
        da: &[[f32; 128]],
        db: &[[f32; 128]],
    ) -> Option<(Vec<RowBest>, Vec<ColBest>)> {
        SHARED.with(|cell| {
            let shared: &Option<&CudaMatcher> = cell;
            shared.and_then(|matcher| matcher.run(da, db))
        })
    }
}

#[cfg(test)]
#[path = "matching/tests/tests_integration.rs"]
mod tests;
