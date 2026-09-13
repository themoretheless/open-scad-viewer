mod readback;
// Headless Metal probe and readback verification.
fn main() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Metal adapter required");
        let info = adapter.get_info();
        let features = adapter.features();
        let timestamps = features.contains(wgpu::Features::TIMESTAMP_QUERY);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Curvex headless Metal qualification"),
                required_features: if timestamps {
                    wgpu::Features::TIMESTAMP_QUERY
                } else {
                    wgpu::Features::empty()
                },
                ..Default::default()
            })
            .await
            .expect("Metal device required");
        let encoder = device.create_command_encoder(&Default::default());
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .expect("Metal completion");
        println!(
            "{}",
            serde_json::json!({"adapter":info.name,"backend":format!("{:?}",info.backend),"timestamp_queries":timestamps,"timestamp_period_ns":queue.get_timestamp_period(),"submission_completed":true,"readback":readback::verify(&device, &queue)})
        );
    });
}
