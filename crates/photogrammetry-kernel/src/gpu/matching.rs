//! GPU descriptor matching (feature `gpu`). One thread per candidate pair
//! computes the full 128-component squared distance; workgroup reductions keep
//! the CPU scan's ordering semantics: smaller distance wins, equal distances
//! keep the first index (the CPU scan uses strict `<`). NaN distances never
//! win, as on the CPU. GPU floating-point contraction (fma) means results are
//! NOT bit-identical to the CPU path — this mode is opt-in and qualified
//! separately; CPU remains the deterministic reference.

use wgpu::util::DeviceExt;

use super::GpuContext;

const SHADER: &str = r#"
struct Params {
    rows: u32,
    cols: u32,
}
struct RowBest {
    j: u32,
    d1: f32,
    d2: f32,
}
struct ColBest {
    i: u32,
    d1: f32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> da: array<f32>;
@group(0) @binding(2) var<storage, read> db: array<f32>;
@group(0) @binding(3) var<storage, read_write> rows_out: array<RowBest>;
@group(0) @binding(4) var<storage, read_write> cols_out: array<ColBest>;

const NONE: u32 = 0xFFFFFFFFu;
const INF: f32 = 3.402823466e+38;
const WG: u32 = 256u;

var<workgroup> sh_d: array<f32, 256>;
var<workgroup> sh_i: array<u32, 256>;
var<workgroup> sh_s: array<f32, 256>;

// True when (a_d, a_i) outranks (b_d, b_i): smaller distance, then smaller index.
fn better(a_d: f32, a_i: u32, b_d: f32, b_i: u32) -> bool {
    return a_d < b_d || (a_d == b_d && a_i < b_i);
}

fn dist(row: u32, col: u32) -> f32 {
    var d = 0.0;
    let ra = row * 128u;
    let rb = col * 128u;
    for (var k = 0u; k < 128u; k++) {
        let v = da[ra + k] - db[rb + k];
        d += v * v;
    }
    return d;
}

@compute @workgroup_size(256)
fn match_rows(@builtin(workgroup_id) wg: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let row = wg.x;
    var best_d = INF;
    var best_j = NONE;
    var second_d = INF;
    var col = lid.x;
    while (col < params.cols) {
        let d = dist(row, col);
        if (d < best_d) {
            second_d = best_d;
            best_d = d;
            best_j = col;
        } else if (d < second_d) {
            second_d = d;
        }
        col += WG;
    }
    sh_d[lid.x] = best_d;
    sh_i[lid.x] = best_j;
    sh_s[lid.x] = second_d;
    workgroupBarrier();
    var stride = WG / 2u;
    while (stride > 0u) {
        if (lid.x < stride) {
            let other = lid.x + stride;
            if (better(sh_d[other], sh_i[other], sh_d[lid.x], sh_i[lid.x])) {
                // other becomes best; old best competes for second
                sh_s[lid.x] = min(min(sh_s[lid.x], sh_s[other]), sh_d[lid.x]);
                sh_d[lid.x] = sh_d[other];
                sh_i[lid.x] = sh_i[other];
            } else {
                sh_s[lid.x] = min(min(sh_s[lid.x], sh_s[other]), sh_d[other]);
            }
        }
        workgroupBarrier();
        stride = stride / 2u;
    }
    if (lid.x == 0u) {
        rows_out[row] = RowBest(sh_i[0], sh_d[0], sh_s[0]);
    }
}

@compute @workgroup_size(256)
fn match_cols(@builtin(workgroup_id) wg: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let col = wg.x;
    var best_d = INF;
    var best_i = NONE;
    var row = lid.x;
    while (row < params.rows) {
        let d = dist(row, col);
        if (d < best_d) {
            best_d = d;
            best_i = row;
        }
        row += WG;
    }
    sh_d[lid.x] = best_d;
    sh_i[lid.x] = best_i;
    workgroupBarrier();
    var stride = WG / 2u;
    while (stride > 0u) {
        if (lid.x < stride) {
            let other = lid.x + stride;
            if (better(sh_d[other], sh_i[other], sh_d[lid.x], sh_i[lid.x])) {
                sh_d[lid.x] = sh_d[other];
                sh_i[lid.x] = sh_i[other];
            }
        }
        workgroupBarrier();
        stride = stride / 2u;
    }
    if (lid.x == 0u) {
        cols_out[col] = ColBest(sh_i[0], sh_d[0]);
    }
}
"#;

pub struct RowBest {
    pub j: usize,
    pub d1: f32,
    pub d2: f32,
}
pub struct ColBest {
    pub i: usize,
    pub d1: f32,
}

pub struct GpuMatcher {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    rows_pipeline: wgpu::ComputePipeline,
    cols_pipeline: wgpu::ComputePipeline,
}

impl GpuMatcher {
    pub fn new(context: &GpuContext) -> Self {
        let device = &context.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("match_descriptors"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("match_descriptors"),
            entries: &[
                super::uniform_entry(0),
                super::storage_entry(1, true),
                super::storage_entry(2, true),
                super::storage_entry(3, false),
                super::storage_entry(4, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("match_descriptors"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let rows_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("match_rows"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("match_rows"),
            compilation_options: Default::default(),
            cache: None,
        });
        let cols_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("match_cols"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("match_cols"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self {
            device: device.clone(),
            queue: context.queue.clone(),
            layout,
            rows_pipeline,
            cols_pipeline,
        }
    }

    /// Nearest/second-nearest per row and nearest per column over the full
    /// descriptor grid. Mirrors the CPU scan's tie-breaking; float contraction
    /// may shift borderline distances, so GPU results are not bit-identical.
    pub fn match_descriptors(&self, da: &[[f32; 128]], db: &[[f32; 128]]) -> (Vec<RowBest>, Vec<ColBest>) {
        let rows = da.len() as u32;
        let cols = db.len() as u32;
        let device = &self.device;

        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params"),
            contents: &[rows.to_ne_bytes(), cols.to_ne_bytes()].concat(),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let packed_a = pack_descriptors(da);
        let packed_b = pack_descriptors(db);
        let buf_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("da"),
            contents: &packed_a,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let buf_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("db"),
            contents: &packed_b,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let row_bytes = (rows as u64) * 12;
        let col_bytes = (cols as u64) * 8;
        let out_rows = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rows_out"),
            size: row_bytes.max(12),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let out_cols = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cols_out"),
            size: col_bytes.max(8),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read_rows = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("read_rows"),
            size: row_bytes.max(12),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let read_cols = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("read_cols"),
            size: col_bytes.max(8),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("match_descriptors"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buf_a.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: buf_b.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: out_rows.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: out_cols.as_entire_binding() },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("match") });
        if rows > 0 && cols > 0 {
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("match_rows"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.rows_pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.dispatch_workgroups(rows, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("match_cols"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.cols_pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.dispatch_workgroups(cols, 1, 1);
            }
        }
        encoder.copy_buffer_to_buffer(&out_rows, 0, &read_rows, 0, row_bytes.max(12));
        encoder.copy_buffer_to_buffer(&out_cols, 0, &read_cols, 0, col_bytes.max(8));
        self.queue.submit([encoder.finish()]);

        let rows_raw = super::read_buffer(device, &read_rows, row_bytes as usize);
        let cols_raw = super::read_buffer(device, &read_cols, col_bytes as usize);

        let rows_out_vec = rows_raw
            .chunks_exact(12)
            .map(|c| RowBest {
                j: match u32::from_ne_bytes(c[0..4].try_into().unwrap()) {
                    u32::MAX => usize::MAX,
                    raw => raw as usize,
                },
                d1: f32::from_ne_bytes(c[4..8].try_into().unwrap()),
                d2: f32::from_ne_bytes(c[8..12].try_into().unwrap()),
            })
            .collect();
        let cols_out_vec = cols_raw
            .chunks_exact(8)
            .map(|c| ColBest {
                i: match u32::from_ne_bytes(c[0..4].try_into().unwrap()) {
                    u32::MAX => usize::MAX,
                    raw => raw as usize,
                },
                d1: f32::from_ne_bytes(c[4..8].try_into().unwrap()),
            })
            .collect();
        (rows_out_vec, cols_out_vec)
    }
}

fn pack_descriptors(data: &[[f32; 128]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 512);
    for descriptor in data {
        for value in descriptor {
            out.extend_from_slice(&value.to_ne_bytes());
        }
    }
    out
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
/// `None` when no GPU adapter is available; callers fall back to the CPU scan.
pub fn match_pair(da: &[[f32; 128]], db: &[[f32; 128]]) -> Option<(Vec<RowBest>, Vec<ColBest>)> {
    SHARED.with(|cell| {
        let shared: &Option<&(GpuContext, GpuMatcher)> = cell;
        shared.map(|(_, matcher)| matcher.match_descriptors(da, db))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptors(seed: u64, n: usize) -> Vec<[f32; 128]> {
        let mut state = seed;
        (0..n)
            .map(|_| {
                let mut d = [0f32; 128];
                for v in &mut d {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    *v = ((state >> 33) as f32 / 2147483648.0).fract();
                }
                d
            })
            .collect()
    }

    fn cpu_reference(da: &[[f32; 128]], db: &[[f32; 128]]) -> (Vec<(usize, f32, f32)>, Vec<(usize, f32)>) {
        let mut best_a = vec![(usize::MAX, f32::INFINITY, f32::INFINITY); da.len()];
        let mut best_b = vec![(usize::MAX, f32::INFINITY); db.len()];
        for (i, x) in da.iter().enumerate() {
            for (j, y) in db.iter().enumerate() {
                let mut d = 0f32;
                for k in 0..128 {
                    let v = x[k] - y[k];
                    d += v * v;
                }
                if d < best_a[i].1 {
                    best_a[i].2 = best_a[i].1;
                    best_a[i].1 = d;
                    best_a[i].0 = j;
                } else if d < best_a[i].2 {
                    best_a[i].2 = d;
                }
                if d < best_b[j].1 {
                    best_b[j] = (i, d);
                }
            }
        }
        (best_a, best_b)
    }

    #[test]
    fn gpu_matching_agrees_with_cpu_on_synthetic_descriptors() {
        let Some((da, db)) = match_pair(&descriptors(7, 300), &descriptors(11, 280)).map(|_| {
            (descriptors(7, 300), descriptors(11, 280))
        }) else {
            eprintln!("no GPU adapter; skipping");
            return;
        };
        let (rows, cols) = match_pair(&da, &db).expect("adapter was available");
        let (ref_a, ref_b) = cpu_reference(&da, &db);
        let mut row_exact = 0;
        let mut row_close = 0;
        for (gpu, cpu) in rows.iter().zip(&ref_a) {
            if gpu.j == cpu.0 && gpu.d1.to_bits() == cpu.1.to_bits() && gpu.d2.to_bits() == cpu.2.to_bits() {
                row_exact += 1;
            }
            // GPU float contraction may shift borderline distances; the selected
            // indices must agree and distances must stay within f32 contraction noise.
            if gpu.j == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.) {
                row_close += 1;
            }
        }
        let col_exact = cols
            .iter()
            .zip(&ref_b)
            .filter(|(gpu, cpu)| gpu.i == cpu.0 && (gpu.d1 - cpu.1).abs() <= 1e-4 * cpu.1.max(1.))
            .count();
        assert_eq!(row_close, ref_a.len(), "row selections must agree with CPU");
        assert_eq!(col_exact, ref_b.len(), "column selections must agree with CPU");
        eprintln!("bit-exact rows: {row_exact}/{}", ref_a.len());
    }
}
