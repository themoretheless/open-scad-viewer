//! Compare the unchanged raw-moment plan with explicit centered covariance.
//! Run with --release --features gpu. Output includes every measured sample.
mod stats_baseline;
use compute_core::{ComputeBatch, ComputeRuntime, GpuArray};
use gpu_compute::{GpuContext, GpuTimer};
use math_compute::gpu::{GpuPointCloudStats, MathGpuProgram, MathGpuSession, PointCloudView};
use math_core::V3;
use std::time::{Duration, Instant};

const PAIRS: [(usize, usize); 6] = [(0, 0), (0, 1), (0, 2), (1, 1), (1, 2), (2, 2)];
const WAIT: Duration = Duration::from_secs(30);
const PATHS: [&str; 3] = ["legacy", "centered_aos256", "centered_vec128"];
fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}
fn cloud(n: usize, offset: f32) -> Vec<f32> {
    (0..n)
        .flat_map(|i| {
            let x = ((i % 17) as f32 - 8.) * 0.25;
            let y = (((i * 7) % 13) as f32 - 6.) * 0.5;
            [
                offset + x,
                -2. * offset + y,
                0.5 * offset + x * 0.5 - y * 0.25,
            ]
        })
        .collect()
}
fn run(
    runtime: &ComputeRuntime,
    plan: &MathGpuProgram<'_>,
    output: &GpuArray<f32>,
    after: Option<&ComputeBatch<'_>>,
    timer: Option<&GpuTimer>,
    upload: Option<(&GpuArray<f32>, &[f32])>,
) -> Result<(f64, f64, Vec<f32>), Box<dyn std::error::Error>> {
    let wall = Instant::now();
    if let Some((array, values)) = upload {
        runtime.write(array, 0, values)?;
    }
    let c = runtime.context();
    let mut encoder = c.device.create_command_encoder(&Default::default());
    let record = |pass: &mut gpu_compute::wgpu::ComputePass<'_>| {
        plan.record_in_pass(pass);
        if let Some(after) = after {
            after.record_in_pass(pass);
        }
    };
    let mut timestamp = if let Some(timer) = timer {
        Some(timer.record_compute(&mut encoder, "point cloud statistics", record)?)
    } else {
        let mut pass = encoder.begin_compute_pass(&gpu_compute::wgpu::ComputePassDescriptor {
            label: Some("point cloud statistics"),
            timestamp_writes: None,
        });
        record(&mut pass);
        None
    };
    let mut read = runtime.record_read(&mut encoder, output)?;
    let submission = c.queue.submit([if let Some(timer) = timer {
        timer.finish(encoder)?
    } else {
        encoder.finish()
    }]);
    read.submitted(submission.clone());
    if let Some(ticket) = &mut timestamp {
        ticket.submitted(submission);
    }
    let value = read.wait(WAIT)?;
    let gpu = if let Some(ticket) = timestamp {
        ticket.wait(WAIT)?.elapsed_ns / 1e6
    } else {
        f64::NAN
    };
    Ok((gpu, wall.elapsed().as_secs_f64() * 1e3, value))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    math_compute::install();
    let c = GpuContext::with_timestamps()?;
    let runtime = ComputeRuntime::new(&c)?;
    let math = MathGpuSession::new(&c);
    let timer = GpuTimer::new(&c)?;
    let frozen = stats_baseline::FrozenCentered::new(&c)?;
    eprintln!(
        "{:?}; 200ms rotated warmup;31 rotated samples per mode; same resident input and full24scalar output; GPU timestamp wall includes post-completion resolve; ordinary wall excludes profiler; uploaded-f32 centered-f64 reference",
        c.backend_report()
    );
    println!("kind,n,offset,path,mode,sample,gpu_ms,wall_ms,max_covariance_abs_error");
    for n in [4097, 1_000_003] {
        for offset in [0., 10_000., 1_000_000.] {
            let data = cloud(n, offset);
            let cpu: Vec<V3> = data
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| p.map(f64::from))
                .collect();
            let reference = math_core::point_cloud_stats(&cpu)?;
            let input = runtime.upload(&data)?;
            let mut legacy = math.program(&runtime)?;
            let old = legacy.point_cloud_stats(PointCloudView::new(&input)?)?;
            let mut stable = math.program(&runtime)?;
            let new = stable.point_cloud_stats_stable(PointCloudView::new(&input)?)?;
            let frozen = frozen.prepare(&runtime, &input, &old.values)?;
            let plans = [&legacy, &legacy, &stable];
            let outputs = [&old.values, &old.values, &new.values];
            let after = [None, Some(&frozen), None];
            let mut errors = [0f64; 3];
            let mut initial = Vec::new();
            for path in 0..3 {
                let (_, _, values) = run(
                    &runtime,
                    plans[path],
                    outputs[path],
                    after[path],
                    None,
                    None,
                )?;
                for (k, &(a, b)) in PAIRS.iter().enumerate() {
                    errors[path] = errors[path].max(
                        (f64::from(values[GpuPointCloudStats::COVARIANCE.start + k])
                            - reference.moments.covariance[a][b])
                            .abs(),
                    );
                }
                initial.push(values);
            }
            for path in 1..3 {
                assert_eq!(&initial[0][..18], &initial[path][..18]);
                assert!(
                    errors[path] < 2e-5,
                    "centered covariance error {}",
                    errors[path]
                );
            }
            let warm = Instant::now();
            while warm.elapsed() < Duration::from_millis(200) {
                for path in 0..3 {
                    run(
                        &runtime,
                        plans[path],
                        outputs[path],
                        after[path],
                        None,
                        None,
                    )?;
                }
            }
            for (mode, use_timer, upload) in [
                ("gpu_profile", true, false),
                ("resident", false, false),
                ("upload", false, true),
            ] {
                let mut times: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
                let mut gpu_times: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
                for sample in 0..31 {
                    for step in 0..3 {
                        let path = (sample + step) % 3;
                        let (gpu, wall, _) = run(
                            &runtime,
                            plans[path],
                            outputs[path],
                            after[path],
                            use_timer.then_some(&timer),
                            upload.then_some((&input, data.as_slice())),
                        )?;
                        times[path].push(wall);
                        gpu_times[path].push(gpu);
                        println!(
                            "sample,{n},{offset},{},{mode},{sample},{gpu:.6},{wall:.6},{:.9}",
                            PATHS[path], errors[path]
                        );
                    }
                }
                for path in 0..3 {
                    println!(
                        "median,{n},{offset},{},{mode},31,{:.6},{:.6},{:.9}",
                        PATHS[path],
                        median(gpu_times[path].clone()),
                        median(times[path].clone()),
                        errors[path]
                    );
                }
            }
            let mut cpu_times = Vec::new();
            for _ in 0..5 {
                let start = Instant::now();
                std::hint::black_box(math_core::point_cloud_stats(std::hint::black_box(&cpu))?);
                cpu_times.push(start.elapsed().as_secs_f64() * 1e3);
            }
            println!(
                "median,{n},{offset},cpu,centered_f64,5,NaN,{:.6},0",
                median(cpu_times)
            );
            // Both synchronous methods include upload, plan/dispatch, readback
            // and construction of the public summary. Kernel warmup is excluded.
            for path in 0..2 {
                if path == 0 {
                    math.try_point_cloud_stats(&cpu)?;
                } else {
                    math.try_point_cloud_stats_stable(&cpu)?;
                }
            }
            let mut sync: [Vec<f64>; 2] = std::array::from_fn(|_| Vec::new());
            for sample in 0..15 {
                for step in 0..2 {
                    let path = (sample + step) % 2;
                    let start = Instant::now();
                    let result = if path == 0 {
                        math.try_point_cloud_stats(&cpu)?
                    } else {
                        math.try_point_cloud_stats_stable(&cpu)?
                    };
                    let elapsed = start.elapsed().as_secs_f64() * 1e3;
                    let error = PAIRS
                        .iter()
                        .map(|&(a, b)| {
                            (result.value.moments.covariance[a][b]
                                - reference.moments.covariance[a][b])
                                .abs()
                        })
                        .fold(0f64, f64::max);
                    sync[path].push(elapsed);
                    println!(
                        "sample,{n},{offset},{},session,{sample},NaN,{elapsed:.6},{error:.9}",
                        ["legacy", "centered"][path]
                    );
                }
            }
            for path in 0..2 {
                println!(
                    "median,{n},{offset},{},session,15,NaN,{:.6},NaN",
                    ["legacy", "centered"][path],
                    median(sync[path].clone())
                );
            }
        }
    }
    Ok(())
}
