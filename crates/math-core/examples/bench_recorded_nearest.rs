//! Reproducible baseline/candidate/production nearest-neighbor comparison.
//! All GPU timings include completion and both full output readbacks; they are
//! host latency measurements, not kernel-only GPU timestamps.
use compute_core::{Binding, ComputeRuntime, GpuArray, Kernel, uniform_f32};
use gpu_compute::{GpuContext, wgpu};
use math_core::{
    NEAREST_NEIGHBOR_COOPERATIVE_WGSL, NEAREST_NEIGHBOR_WGSL, V3,
    gpu::{MathGpuProgram, MathGpuSession, NearestNeighborAlgorithm, PointCloudView},
    nearest_neighbor,
};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

// Exact binary fractions keep this validation dataset's f64 and f32 squared
// distances identical, including ties. GPU-specific adversarial tests exercise
// near ties, overflow, empty targets and partial target workgroups separately.
fn points(count: usize, mut state: u32) -> Vec<f32> {
    (0..count * 3)
        .map(|_| {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            ((state >> 16) & 1023) as f32 / 16.0 - 32.0
        })
        .collect()
}
fn cpu_points(flat: &[f32]) -> Vec<V3> {
    flat.as_chunks::<3>()
        .0
        .iter()
        .map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .collect()
}
enum Dispatch<'a> {
    Raw {
        kernel: Kernel,
        bind: wgpu::BindGroup,
        groups: u32,
    },
    Recorded(MathGpuProgram<'a>),
}
struct Candidate<'a> {
    name: &'static str,
    dispatch: Dispatch<'a>,
    indices: GpuArray<u32>,
    distances: GpuArray<f32>,
}
impl<'a> Candidate<'a> {
    fn raw(
        context: &GpuContext,
        runtime: &ComputeRuntime,
        q: &GpuArray<f32>,
        t: &GpuArray<f32>,
        cooperative: bool,
    ) -> Self {
        let count = q.len() / 3;
        let (name, source, wg) = if cooperative {
            ("cooperative64", NEAREST_NEIGHBOR_COOPERATIVE_WGSL, 64)
        } else {
            ("baseline", NEAREST_NEIGHBOR_WGSL, 256)
        };
        let kernel = Kernel::with_workgroup_size(
            &context.device,
            name,
            source,
            "main",
            &[
                Binding::Uniform,
                Binding::StorageRead,
                Binding::StorageRead,
                Binding::StorageReadWrite,
                Binding::StorageReadWrite,
            ],
            wg,
        )
        .unwrap();
        let indices = runtime.zeros(count).unwrap();
        let distances = runtime.zeros(count).unwrap();
        let params = uniform_f32(
            &context.device,
            &context.queue,
            &[
                f32::from_bits(count as u32),
                f32::from_bits((t.len() / 3) as u32),
                0.,
                0.,
            ],
        );
        let bind = kernel.create_bind_group(
            &context.device,
            &[
                &params,
                q.view().raw(),
                t.view().raw(),
                indices.view().raw(),
                distances.view().raw(),
            ],
        );
        let groups = if cooperative {
            count as u32
        } else {
            kernel.workgroup_count(count as u32)
        };
        Self {
            name,
            dispatch: Dispatch::Raw {
                kernel,
                bind,
                groups,
            },
            indices,
            distances,
        }
    }
    fn recorded(
        runtime: &'a ComputeRuntime,
        session: &'a MathGpuSession,
        q: &GpuArray<f32>,
        t: &GpuArray<f32>,
    ) -> Self {
        let mut plan = session.program(runtime).unwrap();
        let output = plan
            .nearest_neighbors(
                PointCloudView::new(q).unwrap(),
                PointCloudView::new(t).unwrap(),
            )
            .unwrap();
        Self {
            name: "recorded_auto",
            dispatch: Dispatch::Recorded(plan),
            indices: output.indices,
            distances: output.squared_distances,
        }
    }
    fn run(&self, context: &GpuContext, runtime: &ComputeRuntime) -> (Vec<u32>, Vec<f32>) {
        let mut encoder = context.device.create_command_encoder(&Default::default());
        match &self.dispatch {
            Dispatch::Raw {
                kernel,
                bind,
                groups,
            } => kernel.record_dispatch(&mut encoder, bind, *groups),
            Dispatch::Recorded(plan) => plan.record(&mut encoder),
        }
        let mut indices = runtime.record_read(&mut encoder, &self.indices).unwrap();
        let mut distances = runtime.record_read(&mut encoder, &self.distances).unwrap();
        let submission = context.queue.submit([encoder.finish()]);
        indices.submitted(submission.clone());
        distances.submitted(submission);
        (
            indices.wait(Duration::from_secs(30)).unwrap(),
            distances.wait(Duration::from_secs(30)).unwrap(),
        )
    }
    // Reconstruct the former synchronous transport: scalar dispatch, read
    // indices, read distances (three submissions), with cached input buffers.
    fn legacy_sync(&self, context: &GpuContext) -> Vec<(u32, f64)> {
        let Dispatch::Raw {
            kernel,
            bind,
            groups,
        } = &self.dispatch
        else {
            unreachable!()
        };
        kernel.dispatch_bind_group(&context.device, &context.queue, bind, *groups);
        let indices = compute_core::try_read_u32(
            &context.device,
            &context.queue,
            self.indices.view().raw(),
            self.indices.len(),
        )
        .unwrap();
        let distances = compute_core::try_read_f32(
            &context.device,
            &context.queue,
            self.distances.view().raw(),
            self.distances.len(),
        )
        .unwrap();
        indices
            .into_iter()
            .zip(distances)
            .map(|(i, d)| (i, d as f64))
            .collect()
    }
}
fn summary(times: &mut [f64]) -> (f64, f64) {
    times.sort_by(f64::total_cmp);
    (times[times.len() / 2], times[(times.len() - 1) * 9 / 10])
}
fn main() {
    let context = GpuContext::new().expect("GPU required; benchmark never falls back");
    let repeats = std::env::var("NN_BENCH_REPEATS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(15);
    let warmups = std::env::var("NN_BENCH_WARMUPS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(3);
    assert!(repeats >= 3 && warmups >= 1);
    eprintln!(
        "backend={:?}; {warmups} rotated warmups +{repeats} samples; full outputs; milliseconds; host latency",
        context.backend
    );
    println!("queries,targets,mode,implementation,median_ms,p90_ms");
    let runtime = ComputeRuntime::new(&context).unwrap();
    let session = MathGpuSession::new(&context);
    for (n, m) in [(256, 512), (4096, 4096), (16384, 8192)] {
        eprintln!(
            "{n}x{m}: {:?}",
            NearestNeighborAlgorithm::for_shape(context.backend, n, m)
        );
        let query_data = points(n, 13);
        let target_data = points(m, 41);
        let queries = runtime.upload(&query_data).unwrap();
        let targets = runtime.upload(&target_data).unwrap();
        let cq = cpu_points(&query_data);
        let ct = cpu_points(&target_data);
        let reference = nearest_neighbor(&cq, &ct);
        let candidates = [
            Candidate::raw(&context, &runtime, &queries, &targets, false),
            Candidate::raw(&context, &runtime, &queries, &targets, true),
            Candidate::recorded(&runtime, &session, &queries, &targets),
        ];
        for candidate in &candidates {
            let (indices, distances) = candidate.run(&context, &runtime);
            for (i, &(index, distance)) in reference.iter().enumerate() {
                assert_eq!(indices[i], index, "{} index {i}", candidate.name);
                assert_eq!(
                    distances[i] as f64, distance,
                    "{} distance {i}",
                    candidate.name
                );
            }
        }
        assert_eq!(
            session.try_nearest_neighbor(&cq, &ct).unwrap().value,
            reference
        );
        assert_eq!(candidates[0].legacy_sync(&context), reference);
        for mode in ["resident", "upload_inclusive"] {
            let mut timings = vec![Vec::new(); candidates.len()];
            for iteration in 0..warmups + repeats {
                for offset in 0..candidates.len() {
                    let index = (iteration + offset) % candidates.len();
                    let begin = Instant::now();
                    if mode == "upload_inclusive" {
                        runtime.write(&queries, 0, &query_data).unwrap();
                        runtime.write(&targets, 0, &target_data).unwrap();
                    }
                    black_box(candidates[index].run(&context, &runtime));
                    let elapsed = begin.elapsed().as_secs_f64() * 1000.;
                    if iteration >= warmups {
                        timings[index].push(elapsed);
                    }
                }
            }
            for (candidate, times) in candidates.iter().zip(timings.iter_mut()) {
                let (median, p90) = summary(times);
                println!("{n},{m},{mode},{},{median:.6},{p90:.6}", candidate.name);
            }
        }
        let mut sync = [Vec::new(), Vec::new()];
        for iteration in 0..warmups + repeats {
            for offset in 0..2 {
                let index = (iteration + offset) % 2;
                let begin = Instant::now();
                if index == 0 {
                    let q: Vec<f32> = cq.iter().flatten().map(|&v| v as f32).collect();
                    let t: Vec<f32> = ct.iter().flatten().map(|&v| v as f32).collect();
                    runtime.write(&queries, 0, &q).unwrap();
                    runtime.write(&targets, 0, &t).unwrap();
                    black_box(candidates[0].legacy_sync(&context));
                } else {
                    black_box(session.try_nearest_neighbor(&cq, &ct).unwrap().value);
                }
                let elapsed = begin.elapsed().as_secs_f64() * 1000.;
                if iteration >= warmups {
                    sync[index].push(elapsed);
                }
            }
        }
        for (name, times) in ["scalar_three_submissions", "auto_one_submission"]
            .into_iter()
            .zip(sync.iter_mut())
        {
            let (median, p90) = summary(times);
            println!("{n},{m},sync_upload_inclusive,{name},{median:.6},{p90:.6}");
        }
        let mut cpu = Vec::new();
        for iteration in 0..8 {
            let begin = Instant::now();
            black_box(nearest_neighbor(&cq, &ct));
            if iteration >= 1 {
                cpu.push(begin.elapsed().as_secs_f64() * 1000.);
            }
        }
        let (median, p90) = summary(&mut cpu);
        println!("{n},{m},cpu,f64,{median:.6},{p90:.6}");
    }
}
