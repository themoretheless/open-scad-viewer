use math_core::Acceleration;

fn main() {
    println!("placements:");
    for placement in [
        Acceleration::Cpu,
        Acceleration::Auto,
        Acceleration::Gpu,
        Acceleration::Cuda,
    ] {
        println!(
            "  {:>4}: gpu-capable={}",
            placement.label(),
            placement.is_gpu()
        );
    }

    #[cfg(feature = "gpu")]
    match math_core::gpu::backend_report() {
        Some(report) => println!(
            "wgpu: label={} native={} browser={} metal={}",
            report.label, report.native_api, report.browser_api, report.metal
        ),
        None => println!("wgpu: unavailable"),
    }

    #[cfg(not(feature = "gpu"))]
    println!("wgpu: feature disabled");

    #[cfg(feature = "cuda")]
    match math_core::cuda::device_report() {
        Some(report) => println!(
            "cuda: name=\"{}\" multiprocessors={} native={}",
            report.name, report.multiprocessors, report.native_cuda
        ),
        None => println!("cuda: unavailable"),
    }

    #[cfg(not(feature = "cuda"))]
    println!("cuda: feature disabled");
}
