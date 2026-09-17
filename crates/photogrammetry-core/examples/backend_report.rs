fn main() {
    #[cfg(feature = "gpu")]
    match photogrammetry_core::gpu::backend_report() {
        Some(report) => println!(
            "photogrammetry-core wgpu: label={} native={} browser={} metal={}",
            report.label, report.native_api, report.browser_api, report.metal
        ),
        None => println!("photogrammetry-core wgpu: unavailable"),
    }

    #[cfg(not(feature = "gpu"))]
    println!("photogrammetry-core wgpu: feature disabled");

    #[cfg(feature = "cuda")]
    match photogrammetry_core::gpu::cuda_device_report() {
        Some(report) => println!(
            "photogrammetry-core cuda: name=\"{}\" multiprocessors={} native={}",
            report.name, report.multiprocessors, report.native_cuda
        ),
        None => println!("photogrammetry-core cuda: unavailable"),
    }

    #[cfg(not(feature = "cuda"))]
    println!("photogrammetry-core cuda: feature disabled");
}
