//! Portable output-pixel Brown--Conrady rectification. Candidate focal search
//! and cancellation remain in `calibration`; this module only maps and samples
//! an already accepted candidate, returning `None` if a pixel is invalid.

use super::wgpu::util::DeviceExt;
use super::{GpuContext, read_buffer, storage_entry, uniform_entry, wgpu};
use crate::{Image, calibration::Calibration};

const SHADER: &str = r#"
struct Params {
    width: u32, height: u32, _pad0: u32, _pad1: u32,
    focal: f32, fx: f32, fy: f32, cx: f32, cy: f32,
    k1: f32, k2: f32, k3: f32, p1: f32, p2: f32,
}
@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var<storage, read> source: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<u32>;
@group(0) @binding(3) var<storage, read_write> valid: array<atomic<u32>>;

fn channel(pixel: u32, shift: u32) -> f32 {
    return f32((pixel >> shift) & 255u);
}
@compute @workgroup_size(16, 16)
fn rectify(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= p.width || id.y >= p.height) { return; }
    let x = f32(id.x) - f32(p.width) * 0.5;
    let y = f32(id.y) - f32(p.height) * 0.5;
    let xn = x / p.focal;
    let yn = y / p.focal;
    let r2 = xn * xn + yn * yn;
    let radial = 1.0 + r2 * (p.k1 + r2 * (p.k2 + r2 * p.k3));
    let derivative = p.k1 + r2 * (2.0 * p.k2 + 3.0 * r2 * p.k3);
    let cross = 2.0 * xn * yn * derivative + 2.0 * p.p1 * xn + 2.0 * p.p2 * yn;
    let dx = radial + 2.0 * xn * xn * derivative + 2.0 * p.p1 * yn + 6.0 * p.p2 * xn;
    let dy = radial + 2.0 * yn * yn * derivative + 6.0 * p.p1 * yn + 2.0 * p.p2 * xn;
    let determinant = dx * dy - cross * cross;
    let u = p.fx * (xn * radial + 2.0 * p.p1 * xn * yn + p.p2 * (r2 + 2.0 * xn * xn)) + p.cx;
    let v = p.fy * (yn * radial + p.p1 * (r2 + 2.0 * yn * yn) + 2.0 * p.p2 * xn * yn) + p.cy;
    // Ordered comparisons also reject NaN; the finite upper bound rejects infinity.
    if (!(abs(u) <= 3.402823e38 && abs(v) <= 3.402823e38 && abs(determinant) <= 3.402823e38 && dx > 1e-6 && dy > 1e-6 &&
          determinant > 1e-6 && u >= 0.0 && v >= 0.0 &&
          u <= f32(p.width - 1u) && v <= f32(p.height - 1u))) {
        atomicAnd(&valid[0], 0u);
        return;
    }
    let ix = u32(floor(u));
    let iy = u32(floor(v));
    let nx = min(ix + 1u, p.width - 1u);
    let ny = min(iy + 1u, p.height - 1u);
    let a = u - f32(ix);
    let b = v - f32(iy);
    let c00 = source[iy * p.width + ix];
    let c10 = source[iy * p.width + nx];
    let c01 = source[ny * p.width + ix];
    let c11 = source[ny * p.width + nx];
    var packed = 0u;
    for (var shift = 0u; shift < 24u; shift += 8u) {
        let value = round((1.0 - b) * ((1.0 - a) * channel(c00, shift) + a * channel(c10, shift)) +
                          b * ((1.0 - a) * channel(c01, shift) + a * channel(c11, shift)));
        packed |= u32(clamp(value, 0.0, 255.0)) << shift;
    }
    output[id.y * p.width + id.x] = packed;
}
"#;

/// Dispatches the portable shader. A missing adapter, device failure, or any
/// invalid mapped pixel returns `None`, so the caller uses the CPU reference.
pub fn render(image: &Image, calibration: &Calibration, focal: f64) -> Option<Vec<u8>> {
    shared().and_then(|(context, renderer)| renderer.render(context, image, calibration, focal))
}

struct Renderer {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

impl Renderer {
    fn new(context: &GpuContext) -> Self {
        let module = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("calibration_rectification"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("calibration_rectification"),
                entries: &[
                    uniform_entry(0),
                    storage_entry(1, true),
                    storage_entry(2, false),
                    storage_entry(3, false),
                ],
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("calibration_rectification"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("calibration_rectification"),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("rectify"),
                compilation_options: Default::default(),
                cache: None,
            });
        Self { pipeline, layout }
    }

    fn render(
        &self,
        context: &GpuContext,
        image: &Image,
        c: &Calibration,
        focal: f64,
    ) -> Option<Vec<u8>> {
        let pixels = image.width.checked_mul(image.height)?;
        let source: Vec<u32> = image
            .rgb
            .chunks_exact(3)
            .map(|v| u32::from(v[0]) | u32::from(v[1]) << 8 | u32::from(v[2]) << 16)
            .collect();
        let params = [
            image.width as u32,
            image.height as u32,
            0,
            0,
            (focal as f32).to_bits(),
            (c.fx as f32).to_bits(),
            (c.fy as f32).to_bits(),
            (c.cx as f32).to_bits(),
            (c.cy as f32).to_bits(),
            (c.k1 as f32).to_bits(),
            (c.k2 as f32).to_bits(),
            (c.k3 as f32).to_bits(),
            (c.p1 as f32).to_bits(),
            (c.p2 as f32).to_bits(),
        ];
        let device = &context.device;
        let source = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rectification_source"),
            contents: bytemuck_u32(&source),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rectification_params"),
            contents: bytemuck_u32(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rectification_output"),
            size: (pixels * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let valid = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rectification_valid"),
            contents: &1u32.to_ne_bytes(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let read_output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rectification_read_output"),
            size: (pixels * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let read_valid = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rectification_read_valid"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("calibration_rectification"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: source.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: valid.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("calibration_rectification"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("calibration_rectification"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(
                (image.width as u32).div_ceil(16),
                (image.height as u32).div_ceil(16),
                1,
            );
        }
        encoder.copy_buffer_to_buffer(&output, 0, &read_output, 0, (pixels * 4) as u64);
        encoder.copy_buffer_to_buffer(&valid, 0, &read_valid, 0, 4);
        context.queue.submit(Some(encoder.finish()));
        let valid = read_buffer(device, &read_valid, 4);
        if valid
            .get(..4)
            .and_then(|v| v.try_into().ok())
            .map(u32::from_ne_bytes)
            != Some(1)
        {
            return None;
        }
        let values = read_buffer(device, &read_output, pixels * 4);
        if values.len() != pixels * 4 {
            return None;
        }
        Some(
            values
                .chunks_exact(4)
                .flat_map(|v| [v[0], v[1], v[2]])
                .collect(),
        )
    }
}

fn bytemuck_u32(values: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(values.as_ptr().cast(), std::mem::size_of_val(values)) }
}

thread_local! {
    static SHARED: std::cell::LazyCell<Option<&'static (GpuContext, Renderer)>> =
        std::cell::LazyCell::new(|| GpuContext::new().map(|context| {
            let renderer = Renderer::new(&context);
            Box::leak(Box::new((context, renderer))) as &'static (GpuContext, Renderer)
        }));
}

fn shared() -> Option<&'static (GpuContext, Renderer)> {
    SHARED.with(|cell| **cell)
}

#[cfg(feature = "cuda")]
pub fn render_cuda(image: &Image, calibration: &Calibration, focal: f64) -> Option<Vec<u8>> {
    use gpu_compute::cuda::{CudaDevice, LaunchConfig, PushKernelArg};
    const PTX: &str = include_str!("rectification.ptx");
    let device = CudaDevice::new()?;
    let module = device.load_ptx(PTX)?;
    let kernel = module.load_function("rectify_brown").ok()?;
    let source: Vec<u32> = image
        .rgb
        .chunks_exact(3)
        .map(|v| u32::from(v[0]) | u32::from(v[1]) << 8 | u32::from(v[2]) << 16)
        .collect();
    let stream = &device.stream;
    let source = device.upload(&source).ok()?;
    let mut output = stream
        .alloc_zeros::<u32>(image.width.checked_mul(image.height)?)
        .ok()?;
    let mut valid = device.upload(&[1u32]).ok()?;
    let width = image.width as u32;
    let height = image.height as u32;
    let values = [
        focal as f32,
        calibration.fx as f32,
        calibration.fy as f32,
        calibration.cx as f32,
        calibration.cy as f32,
        calibration.k1 as f32,
        calibration.k2 as f32,
        calibration.k3 as f32,
        calibration.p1 as f32,
        calibration.p2 as f32,
    ];
    let mut launch = stream.launch_builder(&kernel);
    launch
        .arg(&width)
        .arg(&height)
        .arg(&values[0])
        .arg(&values[1])
        .arg(&values[2])
        .arg(&values[3])
        .arg(&values[4])
        .arg(&values[5])
        .arg(&values[6])
        .arg(&values[7])
        .arg(&values[8])
        .arg(&values[9])
        .arg(&source)
        .arg(&mut output)
        .arg(&mut valid);
    unsafe {
        launch.launch(LaunchConfig {
            grid_dim: (width.div_ceil(16), height.div_ceil(16), 1),
            block_dim: (16, 16, 1),
            shared_mem_bytes: 0,
        })
    }
    .ok()?;
    let mut host_valid = [0u32];
    stream.memcpy_dtoh(&valid, &mut host_valid).ok()?;
    if host_valid != [1] {
        return None;
    }
    let mut pixels = vec![0u32; image.width * image.height];
    stream.memcpy_dtoh(&output, &mut pixels).ok()?;
    Some(
        pixels
            .into_iter()
            .flat_map(|v| {
                [
                    (v & 255) as u8,
                    ((v >> 8) & 255) as u8,
                    ((v >> 16) & 255) as u8,
                ]
            })
            .collect(),
    )
}
