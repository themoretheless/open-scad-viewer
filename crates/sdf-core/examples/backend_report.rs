fn main() {
    #[cfg(feature = "gpu")]
    match sdf_core::gpu_backend_report() {
        Some(report) => println!(
            "sdf-core wgpu: label={} native={} browser={} metal={}",
            report.label, report.native_api, report.browser_api, report.metal
        ),
        None => println!("sdf-core wgpu: unavailable"),
    }

    #[cfg(not(feature = "gpu"))]
    println!("sdf-core wgpu: feature disabled");

    #[cfg(feature = "cuda")]
    match sdf_core::cuda::device_report() {
        Some(report) => println!(
            "sdf-core cuda: name=\"{}\" multiprocessors={} native={}",
            report.name, report.multiprocessors, report.native_cuda
        ),
        None => println!("sdf-core cuda: unavailable"),
    }

    #[cfg(not(feature = "cuda"))]
    println!("sdf-core cuda: feature disabled");
}
