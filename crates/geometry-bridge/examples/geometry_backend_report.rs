fn main() {
    #[cfg(feature = "gpu")]
    match geometry_bridge::gpu_backend_report() {
        Some(report) => println!(
            "geometry-bridge wgpu: label={} native={} browser={} metal={}",
            report.label, report.native_api, report.browser_api, report.metal
        ),
        None => println!("geometry-bridge wgpu: unavailable"),
    }

    #[cfg(not(feature = "gpu"))]
    println!("geometry-bridge wgpu: feature disabled");

    println!(
        "geometry-bridge cuda: lattice kernels use the portable wgpu backend when CUDA is requested"
    );
}
