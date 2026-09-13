//! Headless capture of the real Curvex paint pipeline, used unchanged in both builds.
use curvex::shape::{
    Color, Contour, FillRule, FillStyle, Gradient, PathSegment, Shape, ShapeData, Vec2,
};

use egui::{Pos2, RawInput, Rect};
use serde_json::json;
fn ring(points: &[[f32; 2]]) -> Contour {
    Contour {
        start: Vec2::new(points[0][0], points[0][1]),
        segments: points[1..]
            .iter()
            .map(|p| PathSegment::Line {
                to: Vec2::new(p[0], p[1]),
            })
            .collect(),
    }
}
fn region(name: &str, contours: Vec<Contour>, rule: FillRule) -> Shape {
    let mut s = Shape::new(name.into(), ShapeData::Compound { contours });
    s.fill_rule = rule;
    s.fill = Some(FillStyle::Solid {
        color: Color::new(30, 135, 210, 255),
    });
    s.stroke.width = 0.;
    s
}
fn cases() -> Vec<Shape> {
    let outer = ring(&[[10., 10.], [90., 10.], [90., 90.], [10., 90.]]);
    let inner = ring(&[[30., 30.], [70., 30.], [70., 70.], [30., 70.]]);
    let island = ring(&[[43., 43.], [57., 43.], [57., 57.], [43., 57.]]);
    let donut = region(
        "evenodd nested islands",
        vec![outer.clone(), inner.clone(), island],
        FillRule::EvenOdd,
    );
    let nonzero = region(
        "nonzero same winding",
        vec![outer.clone(), inner.clone()],
        FillRule::Nonzero,
    );
    let bowtie = region(
        "self crossing bowtie",
        vec![ring(&[[10., 10.], [90., 90.], [10., 90.], [90., 10.]])],
        FillRule::EvenOdd,
    );
    let mut gradient = region(
        "linear gradient donut",
        vec![outer.clone(), inner.clone()],
        FillRule::EvenOdd,
    );
    gradient.fill = Some(FillStyle::Gradient {
        gradient: Gradient::default_linear(),
    });
    gradient.opacity = 0.65;
    let mut radial = region(
        "radial concave fill",
        vec![ring(&[
            [10., 10.],
            [90., 10.],
            [90., 45.],
            [45., 45.],
            [45., 90.],
            [10., 90.],
        ])],
        FillRule::Nonzero,
    );
    radial.fill = Some(FillStyle::Gradient {
        gradient: Gradient::default_radial(),
    });
    let mut conic = radial.clone();
    conic.name = "conic concave fill".into();
    conic.fill = Some(FillStyle::Gradient {
        gradient: Gradient::default_conic(),
    });
    let mut repeated = radial.clone();
    repeated.name = "repeated linear fill".into();
    let mut repeated_gradient = Gradient::default_linear();
    repeated_gradient.spread = curvex::shape::GradientSpread::Repeat;
    repeated_gradient.kind = curvex::shape::GradientKind::Linear {
        p1: Vec2::new(0., 0.),
        p2: Vec2::new(0.25, 0.),
    };
    repeated.fill = Some(FillStyle::Gradient {
        gradient: repeated_gradient,
    });
    let mut stroke = Shape::new(
        "gradient cubic stroke".into(),
        ShapeData::BezierPath {
            start: Vec2::new(10., 75.),
            segments: vec![PathSegment::Cubic {
                c1: Vec2::new(15., -20.),
                c2: Vec2::new(85., 120.),
                to: Vec2::new(90., 25.),
            }],
            closed: false,
        },
    );
    stroke.fill = None;
    stroke.stroke.width = 7.;
    stroke.stroke.gradient = Some(Gradient::default_linear());
    let mut ribbon = region("closed gradient stroke", vec![outer], FillRule::Nonzero);
    ribbon.fill = None;
    ribbon.stroke.width = 7.;
    ribbon.stroke.gradient = Some(Gradient::default_linear());
    let mut rectangle = Shape::new(
        "rectangle".into(),
        ShapeData::Rectangle {
            top_left: Vec2::new(10., 10.),
            width: 55.,
            height: 80.,
        },
    );
    rectangle.fill = Some(FillStyle::Solid {
        color: Color::new(25, 150, 180, 255),
    });
    let mut rounded = Shape::new(
        "rounded protrusion".into(),
        ShapeData::Rectangle {
            top_left: Vec2::new(45., 30.),
            width: 48.,
            height: 40.,
        },
    );
    rounded.corner_radii = vec![18.; 4];
    let merged = curvex::boolean_ops::boolean_union_for_bench(&[&rectangle, &rounded]);
    let curves = region(
        "boolean curved protrusion",
        merged
            .into_iter()
            .map(|(start, segments)| Contour { start, segments })
            .collect(),
        FillRule::Nonzero,
    );
    let mut shadowed = curves.clone();
    shadowed.name = "curved outer shadow ghost".into();
    shadowed.shadows.push(curvex::shape::DropShadow {
        dx: 5.,
        dy: 5.,
        color: Color::new(0, 0, 0, 128),
        blur: 0.,
    });
    let mut inset = donut.clone();
    inset.name = "compound inner shadow ghost".into();
    inset.inner_shadows.push(curvex::shape::InnerShadow {
        dx: 7.,
        dy: 3.,
        color: Color::new(0, 0, 0, 128),
        blur: 0.,
    });
    let cases = [
        donut, nonzero, bowtie, gradient, radial, conic, repeated, stroke, ribbon, curves,
        shadowed, inset,
    ];
    cases.to_vec()
}
use eframe::{egui_wgpu, wgpu};
use std::{
    future::Future,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};
struct Unpark(std::thread::Thread);
impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}
fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}
struct Session {
    ctx: egui::Context,
    renderer: egui_wgpu::Renderer,
    doc: std::rc::Rc<curvex::document::Document>,
}
impl Session {
    fn new(device: &wgpu::Device, shapes: &[Shape], enabled: bool) -> Self {
        let ctx = egui::Context::default();
        let mut renderer = egui_wgpu::Renderer::new(
            device,
            wgpu::TextureFormat::Rgba8Unorm,
            egui_wgpu::RendererOptions::default(),
        );
        if enabled {
            assert!(curvex_metal_renderer::egui_integration::install(
                &ctx,
                &mut renderer,
                device,
                wgpu::TextureFormat::Rgba8Unorm,
                1,
                true
            ));
        }
        let side = (shapes.len() as f32).sqrt().ceil() * 110.;
        let mut doc = curvex::document::Document::new("persistent Metal scene".into(), side, side);
        for shape in shapes {
            doc.add_shape(shape.clone());
        }
        Self {
            ctx,
            renderer,
            doc: std::rc::Rc::new(doc),
        }
    }
}
fn frame(
    session: &mut Session,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    _shapes: &[Shape],
    zoom: f32,
    offset: egui::Vec2,
) -> (Vec<u8>, usize, f64, Option<f64>, f64) {
    let Session { ctx, renderer, doc } = session;
    let size = 1024u32;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let cpu_start = std::time::Instant::now();
    ctx.begin_pass(RawInput {
        screen_rect: Some(Rect::from_min_max(
            Pos2::ZERO,
            Pos2::new(size as f32, size as f32),
        )),
        ..Default::default()
    });
    let painter = ctx
        .layer_painter(egui::LayerId::background())
        .with_clip_rect(Rect::from_min_max(
            Pos2::new(3., 5.),
            Pos2::new(size as f32 - 7., size as f32 - 9.),
        ));
    curvex::ui::canvas::draw_canvas(
        &painter,
        doc.as_ref(),
        Rect::from_min_size(Pos2::ZERO, egui::vec2(size as f32, size as f32)),
        offset,
        zoom,
        false,
        10.,
    );
    let output = ctx.end_pass();
    let jobs = ctx.tessellate(output.shapes, output.pixels_per_point);
    let callbacks = jobs
        .iter()
        .filter(|j| matches!(j.primitive, egui::epaint::Primitive::Callback(_)))
        .count();
    for (id, delta) in &output.textures_delta.set {
        renderer.update_texture(device, queue, *id, delta);
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Curvex parity"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let stride = (size * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Curvex readback"),
        size: u64::from(stride) * u64::from(size),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let query = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("frame GPU timestamps"),
        ty: wgpu::QueryType::Timestamp,
        count: 2,
    });
    let resolve = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let times = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [size, size],
        pixels_per_point: 1.,
    };
    let mut commands = renderer.update_buffers(device, queue, &mut encoder, &jobs, &screen);
    encoder.write_timestamp(&query, 0);
    {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Curvex real paint"),
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
        renderer.render(&mut pass.forget_lifetime(), &jobs, &screen);
    }
    encoder.write_timestamp(&query, 1);
    let cpu_ms = cpu_start.elapsed().as_secs_f64() * 1000.;
    commands.push(encoder.finish());
    let draw_started = std::time::Instant::now();
    let rendered = queue.submit(commands);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(rendered),
            timeout: None,
        })
        .unwrap();
    let completed_draw_ms = draw_started.elapsed().as_secs_f64() * 1000.;
    let mut encoder = device.create_command_encoder(&Default::default());
    let mut commands = Vec::new();
    encoder.resolve_query_set(&query, 0..2, &resolve, 0);
    encoder.copy_buffer_to_buffer(&resolve, 0, &times, 0, 16);
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    commands.push(encoder.finish());
    let submission = queue.submit(commands);
    let (ttx, trx) = std::sync::mpsc::channel();
    times
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| ttx.send(r).unwrap());
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range();
    let mut pixels = Vec::new();
    for row in mapped.chunks_exact(stride as usize) {
        pixels.extend_from_slice(&row[..size as usize * 4]);
    }
    drop(mapped);
    buffer.unmap();
    trx.recv().unwrap().unwrap();
    let raw = times.slice(..).get_mapped_range();
    let begin = u64::from_le_bytes(raw[..8].try_into().unwrap());
    let end = u64::from_le_bytes(raw[8..16].try_into().unwrap());
    let gpu_ms = end
        .checked_sub(begin)
        .filter(|&n| begin != 0 && n > 0)
        .map(|n| n as f64 * f64::from(queue.get_timestamp_period()) / 1e6);
    (pixels, callbacks, cpu_ms, gpu_ms, completed_draw_ms)
}

fn median(values: &mut [f64]) -> Option<f64> {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied()
}
fn main() {
    block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
                ..Default::default()
            })
            .await
            .unwrap();
        let mut report = Vec::new();
        for count in [100, 1000, 5000] {
            let templates = cases();
            let shapes: Vec<_> = (0..count)
                .map(|i| {
                    let mut s = templates[i % templates.len()].clone();
                    s.id = curvex::shape::ShapeId::new();
                    let cols = (count as f32).sqrt().ceil() as usize;
                    s.translate((i % cols) as f32 * 110., (i / cols) as f32 * 110.);
                    s
                })
                .collect();
            let mut baseline = Session::new(&device, &shapes, false);
            let mut retained = Session::new(&device, &shapes, true);
            retained.doc = baseline.doc.clone();
            let mut cpu = [vec![], vec![]];
            let mut gpu = [vec![], vec![]];
            let mut completed = [vec![], vec![]];
            let mut raw = Vec::new();
            let mut image_checks = Vec::new();
            for cycle in 0..22 {
                let mut images: [Option<Vec<u8>>; 2] = [None, None];
                for slot in if cycle % 2 == 0 {
                    [0usize, 1]
                } else {
                    [1usize, 0]
                } {
                    let session = if slot == 0 {
                        &mut baseline
                    } else {
                        &mut retained
                    };
                    let (pixels, callbacks, c, g, complete) = frame(
                        session,
                        &device,
                        &queue,
                        &shapes,
                        1000. / ((count as f32).sqrt().ceil() * 110.),
                        egui::Vec2::new(cycle as f32 * 0.25, 0.),
                    );
                    if cycle == 2 && slot == 1 {
                        if let Some(dir) = std::env::args().nth(1) {
                            std::fs::create_dir_all(&dir).unwrap();
                            std::fs::write(
                                std::path::Path::new(&dir).join(format!("scene-{count}.rgba")),
                                &pixels,
                            )
                            .unwrap();
                        }
                    }
                    images[slot] = Some(pixels);
                    if cycle >= 2 {
                        cpu[slot].push(c);
                        if let Some(g) = g {
                            gpu[slot].push(g);
                        }
                        completed[slot].push(complete);
                    }
                    let stats =
                        curvex_metal_renderer::egui_integration::cache_stats(&session.renderer);
                    raw.push(json!({"cpu_cache_payloads":curvex::ui::shape_render::osv_gpu_cache_payloads(),"uploaded_bytes_total":stats.map(|s|s.0.uploaded_bytes).unwrap_or(0),"retained_bytes":stats.map(|s|s.1).unwrap_or(0),"cycle":cycle,"retained":slot==1,"callbacks":callbacks,"cpu_ms":c,"gpu_ms":g,"completed_draw_ms":complete}));
                }
                let before = images[0].as_ref().unwrap();
                let after = images[1].as_ref().unwrap();
                image_checks.push(json!({"cycle":cycle,"max_channel_difference":before.iter().zip(after).map(|(a,b)|a.abs_diff(*b)).max().unwrap(),"changed_channels":before.iter().zip(after).filter(|(a,b)|a!=b).count()}));
            }
            report.push(json!({"shapes":count,"image_checks":image_checks,"completed_baseline_ms":median(&mut completed[0]),"completed_retained_ms":median(&mut completed[1]),"valid_gpu_samples":[gpu[0].len(),gpu[1].len()],"cpu_baseline_ms":median(&mut cpu[0]),"cpu_retained_ms":median(&mut cpu[1]),"gpu_baseline_ms":median(&mut gpu[0]),"gpu_retained_ms":median(&mut gpu[1]),"samples":raw}));
        }
        println!(
            "{}",
            json!({"adapter":format!("{:?}",adapter.get_info()),"workload":"spatial grid of actual Curvex fixtures; complete-document view with camera movement; two warmups and twenty alternating samples","cases":report})
        );
    });
}
