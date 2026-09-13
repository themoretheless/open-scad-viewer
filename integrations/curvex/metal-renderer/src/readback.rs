use curvex_metal_renderer::{Camera, ResidentMesh, RetainedPipeline, Vertex};

fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &RetainedPipeline,
    mesh: &ResidentMesh,
    camera: Camera,
) -> Vec<u8> {
    let size = wgpu::Extent3d {
        width: 256,
        height: 256,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Metal verification target"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Metal readback"),
        size: 256 * 256 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let binding = pipeline.camera(device, camera).unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Metal verification draw"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_scissor_rect(11, 17, 231, 225);
        pipeline.draw(&mut pass, mesh, &binding);
    }
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1024),
                rows_per_image: None,
            },
        },
        size,
    );
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let pixels = buffer.slice(..).get_mapped_range().to_vec();
    buffer.unmap();
    pixels
}

pub fn verify(device: &wgpu::Device, queue: &wgpu::Queue) -> serde_json::Value {
    let pipeline = RetainedPipeline::new(device, wgpu::TextureFormat::Rgba8Unorm, 1).unwrap();
    let vertices = [
        Vertex {
            position: [20., 20.],
            color: [128, 0, 0, 128],
        },
        Vertex {
            position: [180., 35.],
            color: [0, 128, 0, 128],
        },
        Vertex {
            position: [80., 180.],
            color: [0, 0, 128, 128],
        },
        Vertex {
            position: [30., 150.],
            color: [60, 30, 10, 96],
        },
        Vertex {
            position: [210., 170.],
            color: [20, 80, 30, 128],
        },
        Vertex {
            position: [160., 50.],
            color: [40, 20, 100, 128],
        },
    ];
    let indices = [0, 1, 2, 3, 4, 5];
    assert!(ResidentMesh::upload(device, &vertices, &[0, 1, 99], 4096).is_err());
    assert!(ResidentMesh::upload(device, &vertices, &indices, 1).is_err());
    let retained = ResidentMesh::upload(device, &vertices, &indices, 4096).unwrap();
    let snapshot =
        curvex_metal_renderer::cache::MeshData::new(vertices.to_vec(), indices.to_vec()).unwrap();
    let mut cache = curvex_metal_renderer::cache::GeometryCache::new(snapshot.byte_size() * 2, 2);
    let lease = cache.prepare(device, &snapshot).unwrap();
    for _ in 0..100 {
        assert!(std::sync::Arc::ptr_eq(
            &lease,
            &cache.prepare(device, &snapshot).unwrap()
        ));
    }
    let edited = curvex_metal_renderer::cache::MeshData::new(
        vertices
            .iter()
            .map(|v| Vertex {
                color: [20, 40, 60, 128],
                ..*v
            })
            .collect(),
        indices.to_vec(),
    )
    .unwrap();
    let replacement = cache.prepare(device, &edited).unwrap();
    assert!(!std::sync::Arc::ptr_eq(&lease, &replacement));
    let third =
        curvex_metal_renderer::cache::MeshData::new(vertices.to_vec(), indices.to_vec()).unwrap();
    cache.prepare(device, &third).unwrap();
    assert_eq!(cache.stats().evictions, 1);
    assert!(cache.retained_bytes() <= snapshot.byte_size() * 2);
    let camera = Camera {
        screen: [256., 256.],
        dithering: false,
        origin: [0., 0.],
        offset: [0., 0.],
        zoom: 1.,
    };
    // Even an evicted mesh remains valid for the prepared frame owning its lease.
    let old_pixels = capture(device, queue, &pipeline, &lease, camera);
    let edit_pixels = capture(device, queue, &pipeline, &replacement, camera);
    assert!(old_pixels != edit_pixels, "edit reused stale GPU colors");
    assert!(old_pixels == capture(device, queue, &pipeline, &retained, camera));
    cache.clear();
    assert_eq!(cache.retained_bytes(), 0);
    assert!(old_pixels == capture(device, queue, &pipeline, &lease, camera));
    cache.prepare(device, &snapshot).unwrap();
    assert_eq!(cache.stats().uploads, 4);
    let cache_evidence = serde_json::json!({"hits":cache.stats().hits,"uploads":cache.stats().uploads,"evictions":cache.stats().evictions,"edit_changes_pixels":true,"evicted_frame_lease_valid":true,"clear_reuploads":true});
    let mut results = vec![];
    for (zoom, offset) in [
        (1., [0., 0.]),
        (0.5, [33., 17.]),
        (2., [-81., -43.]),
        (1.25, [-9., 5.]),
    ] {
        let camera = Camera {
            screen: [256., 256.],
            dithering: false,
            origin: [3., 7.],
            offset,
            zoom,
        };
        let cpu: Vec<_> = vertices
            .iter()
            .map(|v| Vertex {
                position: [
                    camera.origin[0] + offset[0] + v.position[0] * zoom,
                    camera.origin[1] + offset[1] + v.position[1] * zoom,
                ],
                color: v.color,
            })
            .collect();
        let projected = ResidentMesh::upload(device, &cpu, &indices, 4096).unwrap();
        let identity = Camera {
            origin: [0., 0.],
            offset: [0., 0.],
            zoom: 1.,
            ..camera
        };
        let expected = capture(device, queue, &pipeline, &projected, identity);
        let actual = capture(device, queue, &pipeline, &retained, camera);
        let changed = actual.iter().zip(&expected).filter(|(a, b)| a != b).count();
        let nonzero = actual.chunks_exact(4).filter(|p| p[3] != 0).count();
        assert!(nonzero > 1000, "GPU rendered empty output");
        assert_eq!(changed, 0, "camera projection differs");
        results.push(serde_json::json!({"zoom":zoom,"offset":offset,"changed_channels":changed,"nontransparent_pixels":nonzero}));
    }
    serde_json::json!({"camera_readback_cases":results,"cache":cache_evidence,"invalid_indices_rejected":true,"budget_rejected":true,"scope":"Two overlapping colored triangles; CPU vs shader camera projection. Not yet full Curvex parity."})
}
