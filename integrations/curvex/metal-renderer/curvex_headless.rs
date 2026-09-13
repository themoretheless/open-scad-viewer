//! Headless capture of the real Curvex paint pipeline, used unchanged in both builds.
use curvex::shape::{
    Color, Contour, FillRule, FillStyle, Gradient, PathSegment, Shape, ShapeData, Vec2,
};
use curvex::ui::canvas::CanvasTransform;
use curvex::ui::shape_render::draw_shape_cached;
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
fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shape: &Shape,
    zoom: f32,
    enabled: bool,
    dithering: bool,
    format: wgpu::TextureFormat,
) -> (Vec<u8>, usize) {
    let ctx = egui::Context::default();
    let size = (110. * zoom) as u32;
    let mut renderer = egui_wgpu::Renderer::new(
        device,
        format,
        egui_wgpu::RendererOptions {
            dithering,
            ..Default::default()
        },
    );
    if enabled {
        assert!(curvex_metal_renderer::egui_integration::install(
            &ctx,
            &mut renderer,
            device,
            format,
            1,
            dithering
        ));
    }
    let mut doc = curvex::document::Document::new("GPU parity".into(), 110., 110.);
    doc.add_shape(shape.clone());
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
    draw_shape_cached(
        &painter,
        shape,
        &CanvasTransform::new(Pos2::ZERO, egui::Vec2::ZERO, zoom),
        Some(&doc),
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
    let mut encoder = device.create_command_encoder(&Default::default());
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [size, size],
        pixels_per_point: 1.,
    };
    let mut commands = renderer.update_buffers(device, queue, &mut encoder, &jobs, &screen);
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
    (pixels, callbacks)
}
fn main() {
    block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let mut report = Vec::new();
        let mut total_callbacks = 0;
        for shape in cases() {
            for zoom in [0.5, 2., 8.] {
                for dithering in [false, true] {
                    for format in [
                        wgpu::TextureFormat::Rgba8Unorm,
                        wgpu::TextureFormat::Bgra8Unorm,
                    ] {
                        let (before, _) =
                            capture(&device, &queue, &shape, zoom, false, dithering, format);
                        let (after, callbacks) =
                            capture(&device, &queue, &shape, zoom, true, dithering, format);
                        total_callbacks += callbacks;
                        let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
                        let max_diff = before
                            .iter()
                            .zip(&after)
                            .map(|(a, b)| a.abs_diff(*b))
                            .max()
                            .unwrap();
                        report.push(json!({"format":format!("{format:?}"),"case":shape.name,"zoom":zoom,"dithering":dithering,"callbacks":callbacks,"changed_channels":changed,"max_channel_difference":max_diff}));
                    }
                }
            }
        }
        println!(
            "{}",
            json!({"adapter":format!("{:?}",adapter.get_info()),"cases":report,"total_callbacks":total_callbacks})
        );
        assert!(total_callbacks > 0, "GPU optimization was never exercised");
        assert!(
            report.iter().all(|r| r["max_channel_difference"] == 0),
            "GPU parity failed"
        );
    });
}
