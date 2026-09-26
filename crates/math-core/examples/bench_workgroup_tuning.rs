//! Workgroup-size tuning benchmark for the domain reduction kernels: runs
//! point_bounds and chamfer at every power-of-two workgroup size the runtime
//! may pick, so the tuned_workgroup_size heuristic can be checked against
//! real shapes instead of the generic elementwise kernels.
//!
//! Run: cargo run --release -p osv-math --features gpu --example bench_workgroup_tuning

use compute_core::gpu_compute::GpuContext;
use compute_core::{Binding, Kernel, read_f32, storage_f32, storage_f32_zeroed, uniform_f32};
use math_core::{CHAMFER_WGSL, POINT_BOUNDS_WGSL, V3};
use std::time::Instant;

const POINTS: usize = 2_000_000;
const QUERIES: usize = 20_000;
const TARGETS: usize = 30_000;
const ITERATIONS: usize = 30;

fn params_u32(count: u32) -> Vec<f32> {
    let mut floats = vec![0.0f32; 4];
    floats[0] = f32::from_le_bytes(count.to_le_bytes());
    floats
}

fn points(n: usize, seed: f64) -> Vec<V3> {
    (0..n)
        .map(|i| {
            let f = i as f64 + seed;
            [
                (f * 0.013).sin() * 200. - 7.,
                (f * 0.017).cos() * 120. + 3.,
                f * 0.00031 - 50.,
            ]
        })
        .collect()
}

fn main() {
    let Some(context) = GpuContext::new() else {
        eprintln!("no GPU adapter available");
        return;
    };
    let device = &context.device;
    let queue = &context.queue;
    println!(
        "workgroup tuning bench: backend {}, {} points, chamfer {}x{}",
        context.backend_label(),
        POINTS,
        QUERIES,
        TARGETS
    );

    // CPU reference for context.
    let cloud = points(POINTS, 0.);
    let start = Instant::now();
    let cpu_bounds = math_core::point_bounds(&cloud);
    let cpu_ms = start.elapsed().as_secs_f64() * 1e3;
    println!("cpu point_bounds:              {cpu_ms:8.2} ms");

    let flat: Vec<f32> = cloud.iter().flatten().map(|&v| v as f32).collect();
    let points_buf = storage_f32(device, queue, &flat);
    let bounds_bindings = [
        Binding::Uniform,
        Binding::StorageRead,
        Binding::StorageReadWrite,
        Binding::StorageReadWrite,
    ];

    println!("\npoint_bounds (12 B/point read):");
    for wg in [64u32, 128, 256, 512] {
        let kernel = Kernel::with_workgroup_size(
            device,
            "point_bounds",
            POINT_BOUNDS_WGSL,
            "main",
            &bounds_bindings,
            wg,
        )
        .unwrap_or_else(|error| panic!("point_bounds@WG{wg}: {error}"));
        let groups = kernel.workgroup_count(POINTS as u32);
        let partials = (groups * 3) as usize;
        let out_min = storage_f32_zeroed(device, queue, partials);
        let out_max = storage_f32_zeroed(device, queue, partials);
        let params = uniform_f32(device, queue, &params_u32(POINTS as u32));
        kernel.dispatch(
            device,
            queue,
            &[&params, &points_buf, &out_min, &out_max],
            POINTS as u32,
        );
        // Sustained: batch the dispatches, flush once.
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            kernel.dispatch(
                device,
                queue,
                &[&params, &points_buf, &out_min, &out_max],
                POINTS as u32,
            );
        }
        let raw_min = read_f32(device, queue, &out_min, partials);
        let _ = raw_min;
        let elapsed = start.elapsed().as_secs_f64() / ITERATIONS as f64;
        let bytes = POINTS as f64 * 12.0;
        // Fold the last partials to prove the kernel still computes.
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for chunk in raw_min.chunks_exact(3).take(groups as usize) {
            for axis in 0..3 {
                min[axis] = min[axis].min(chunk[axis] as f64);
            }
        }
        let raw_max = read_f32(device, queue, &out_max, partials);
        for chunk in raw_max.chunks_exact(3).take(groups as usize) {
            for axis in 0..3 {
                max[axis] = max[axis].max(chunk[axis] as f64);
            }
        }
        for axis in 0..3 {
            let cpu_min = cpu_bounds.as_ref().unwrap().min[axis];
            let cpu_max = cpu_bounds.as_ref().unwrap().max[axis];
            assert!(
                (min[axis] - cpu_min).abs() <= 1e-3 * cpu_min.abs().max(1.0),
                "point_bounds@WG{wg} min[{axis}] mismatch: {min:?} vs cpu {cpu_min}"
            );
            assert!(
                (max[axis] - cpu_max).abs() <= 1e-3 * cpu_max.abs().max(1.0),
                "point_bounds@WG{wg} max[{axis}] mismatch: {max:?} vs cpu {cpu_max}"
            );
        }
        let gbps = bytes / elapsed / 1e9;
        println!(
            "  WG{wg:>3}: {gbps:7.2} GB/s ({:.2} ms), {:.1}x cpu",
            elapsed * 1e3,
            cpu_ms / 1e3 / elapsed
        );
    }

    // Chamfer: per-query brute force over all targets, sum + max reductions.
    // Distinct seeds so the clouds do not coincide (coincident clouds make
    // every directed distance exactly zero, which validates nothing).
    let queries = points(QUERIES, 0.);
    let targets = points(TARGETS, 1_000_000.);
    let cpu_start = Instant::now();
    let cpu_directed = math_core::directed_chamfer_distance(
        &queries,
        &targets,
        math_core::Acceleration::Cpu,
    )
    .expect("cpu directed chamfer");
    let cpu_chamfer_ms = cpu_start.elapsed().as_secs_f64() * 1e3;
    println!(
        "\nchamfer {}x{}: cpu directed mean sq dist {:.4} ({cpu_chamfer_ms:.1} ms)",
        QUERIES,
        TARGETS,
        cpu_directed.mean_squared_distance
    );
    let flat_q: Vec<f32> = queries.iter().flatten().map(|&v| v as f32).collect();
    let flat_t: Vec<f32> = targets.iter().flatten().map(|&v| v as f32).collect();
    let queries_buf = storage_f32(device, queue, &flat_q);
    let targets_buf = storage_f32(device, queue, &flat_t);
    let chamfer_bindings = [
        Binding::Uniform,
        Binding::StorageRead,
        Binding::StorageRead,
        Binding::StorageReadWrite,
        Binding::StorageReadWrite,
    ];
    let chamfer_bytes = QUERIES as f64 * TARGETS as f64 * 12.0;

    println!("chamfer (per-query x all-targets scan):");
    for wg in [64u32, 128, 256, 512] {
        let kernel = Kernel::with_workgroup_size(
            device,
            "chamfer",
            CHAMFER_WGSL,
            "main",
            &chamfer_bindings,
            wg,
        )
        .unwrap_or_else(|error| panic!("chamfer@WG{wg}: {error}"));
        let groups = kernel.workgroup_count(QUERIES as u32);
        let out_sum = storage_f32_zeroed(device, queue, groups as usize);
        let out_max = storage_f32_zeroed(device, queue, groups as usize);
        let mut params = params_u32(QUERIES as u32);
        params[1] = f32::from_le_bytes((TARGETS as u32).to_le_bytes());
        let params = uniform_f32(device, queue, &params);
        kernel.dispatch(
            device,
            queue,
            &[&params, &queries_buf, &targets_buf, &out_sum, &out_max],
            QUERIES as u32,
        );
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            kernel.dispatch(
                device,
                queue,
                &[&params, &queries_buf, &targets_buf, &out_sum, &out_max],
                QUERIES as u32,
            );
        }
        let sums = read_f32(device, queue, &out_sum, groups as usize);
        let elapsed = start.elapsed().as_secs_f64() / ITERATIONS as f64;
        let gbps = chamfer_bytes / elapsed / 1e9;
        let gpu_mean_sq = sums.iter().map(|&v| v as f64).sum::<f64>() / QUERIES as f64;
        assert!(
            (gpu_mean_sq - cpu_directed.mean_squared_distance).abs()
                <= 1e-3 * cpu_directed.mean_squared_distance.max(1.0),
            "chamfer@WG{wg} mean sq mismatch: gpu {gpu_mean_sq} vs cpu {}",
            cpu_directed.mean_squared_distance
        );
        println!(
            "  WG{wg:>3}: {gbps:7.2} GB/s effective ({:.2} ms), mean sq {gpu_mean_sq:.4} (matches cpu)",
            elapsed * 1e3
        );
    }
}
