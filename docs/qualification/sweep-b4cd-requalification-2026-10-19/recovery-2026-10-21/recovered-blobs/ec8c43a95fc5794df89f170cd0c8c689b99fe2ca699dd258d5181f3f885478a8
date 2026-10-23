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
    match gpu_compute::available_backend_report() {
        Some(report) => println!(
            "gpu-compute wgpu: label={} native={} browser={} metal={}",
            report.label, report.native_api, report.browser_api, report.metal
        ),
        None => println!("gpu-compute wgpu: unavailable"),
    }

    #[cfg(feature = "gpu")]
    match math_core::gpu::backend_report() {
        Some(report) => println!(
            "math-core wgpu: label={} native={} browser={} metal={}",
            report.label, report.native_api, report.browser_api, report.metal
        ),
        None => println!("math-core wgpu: unavailable"),
    }

    #[cfg(not(feature = "gpu"))]
    println!("wgpu: feature disabled");

    #[cfg(feature = "cuda")]
    match gpu_compute::cuda::available_device_report() {
        Some(report) => println!(
            "gpu-compute cuda: name=\"{}\" multiprocessors={} native={}",
            report.name, report.multiprocessors, report.native_cuda
        ),
        None => println!("gpu-compute cuda: unavailable"),
    }

    #[cfg(feature = "cuda")]
    match math_core::cuda::device_report() {
        Some(report) => println!(
            "math-core cuda: name=\"{}\" multiprocessors={} native={}",
            report.name, report.multiprocessors, report.native_cuda
        ),
        None => println!("math-core cuda: unavailable"),
    }

    #[cfg(not(feature = "cuda"))]
    println!("cuda: feature disabled");
}
