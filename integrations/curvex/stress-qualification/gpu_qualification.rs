//! Copy into an isolated migrated Curvex examples/ directory and run --release.
//! Exercises the real draw_shape_cached pipeline in a native wgpu window.
use curvex::{
    document::Document,
    shape::{Color, FillStyle, Gradient, PathSegment, Shape, ShapeData, Vec2},
    ui::{canvas::CanvasTransform, shape_render::draw_shape_cached},
};
use egui::Color32;
use serde_json::{Value, json};
use std::{path::PathBuf, time::Instant};
struct App {
    doc: Document,
    shapes: Vec<Shape>,
    stage: usize,
    frame: usize,
    samples: Vec<Value>,
    report: Vec<Value>,
    previous: Instant,
    output: PathBuf,
    adapter: String,
    captures: usize,
    start: Instant,
}
impl App {
    fn scene(count: usize) -> (Document, Vec<Shape>) {
        let mut doc = Document::new("GPU qualification".into(), 2400., 2400.);
        let cols = (count as f32).sqrt().ceil() as usize;
        let mut shapes = vec![];
        for i in 0..count {
            let x = (i % cols) as f32 * 30.;
            let y = (i / cols) as f32 * 30.;
            let mut s = match i % 4 {
                0 => Shape::new(
                    format!("rectangle {i}"),
                    ShapeData::Rectangle {
                        top_left: Vec2::new(x + 3., y + 3.),
                        width: 22.,
                        height: 22.,
                    },
                ),
                1 => Shape::new(
                    format!("crossing {i}"),
                    ShapeData::BezierPath {
                        start: Vec2::new(x + 3., y + 3.),
                        segments: vec![
                            PathSegment::Line {
                                to: Vec2::new(x + 25., y + 25.),
                            },
                            PathSegment::Line {
                                to: Vec2::new(x + 3., y + 25.),
                            },
                            PathSegment::Line {
                                to: Vec2::new(x + 25., y + 3.),
                            },
                        ],
                        closed: false,
                    },
                ),
                _ => Shape::new(
                    format!("cubic {i}"),
                    ShapeData::BezierPath {
                        start: Vec2::new(x + 3., y + 20.),
                        segments: vec![PathSegment::Cubic {
                            c1: Vec2::new(x + 3., y - 5.),
                            c2: Vec2::new(x + 25., y + 35.),
                            to: Vec2::new(x + 25., y + 10.),
                        }],
                        closed: false,
                    },
                ),
            };
            s.stroke.width = if i % 4 == 1 { 7. } else { 2. };
            s.fill = if i % 4 == 0 {
                Some(FillStyle::Solid {
                    color: Color::new(30, 135, 210, 255),
                })
            } else {
                None
            };
            if i % 4 == 3 {
                s.stroke.gradient = Some(Gradient::default_linear());
            }
            doc.add_shape(s.clone());
            shapes.push(s);
        }
        (doc, shapes)
    }
    fn save_report(&self) {
        std::fs::write(self.output.join("gpu-report.json"),serde_json::to_vec_pretty(&json!({"adapter":self.adapter,"captures":self.captures,"stages":self.report,"elapsed_seconds":self.start.elapsed().as_secs_f64(),"note":"Native eframe/wgpu window; CPU paint and inter-frame intervals, not GPU timestamp queries. No editor input/layout timing."})).unwrap()).unwrap();
    }
}
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let interval = now.duration_since(self.previous).as_secs_f64() * 1000.;
        self.previous = now;
        let ctx = ui.ctx().clone();
        for ev in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot {
                user_data, image, ..
            } = ev
            {
                let tag = user_data
                    .data
                    .as_ref()
                    .and_then(|d| d.downcast_ref::<String>())
                    .cloned()
                    .unwrap();
                let mut bytes =
                    format!("P6\n{} {}\n255\n", image.size[0], image.size[1]).into_bytes();
                for p in &image.pixels {
                    bytes.extend_from_slice(&p.to_array()[..3]);
                }
                std::fs::write(self.output.join(format!("{tag}.ppm")), bytes).unwrap();
                self.captures += 1;
            }
        }
        if self.stage >= 3 {
            if self.captures >= 6 {
                self.save_report();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint();
            }
            return;
        }
        let count = [100, 1000, 5000][self.stage];
        let viewport = ui.max_rect();
        ui.painter().rect_filled(viewport, 0., Color32::WHITE);
        let moving = (40..80).contains(&self.frame);
        let phase = (self.frame as f32 - 40.) / 40.;
        let base =
            (viewport.width().min(viewport.height()) - 20.) / ((count as f32).sqrt().ceil() * 30.);
        let zoom = base * if moving { 1. + phase * 0.25 } else { 1. };
        let pan = if moving {
            egui::vec2(phase * 20., phase * 10.)
        } else {
            egui::Vec2::ZERO
        };
        let ct = CanvasTransform::new(viewport.min + egui::vec2(10., 10.), pan, zoom);
        let start = Instant::now();
        for s in &self.shapes {
            draw_shape_cached(ui.painter(), s, &ct, Some(&self.doc));
        }
        let paint_ms = start.elapsed().as_secs_f64() * 1000.;
        self.samples.push(
            json!({"frame":self.frame,"moving":moving,"paint_ms":paint_ms,"interval_ms":interval}),
        );
        if self.frame == 25 || self.frame == 100 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                format!("scene-{count}-{}", self.frame),
            )));
        }
        self.frame += 1;
        if self.frame >= 110 {
            self.report
                .push(json!({"shapes":count,"samples":std::mem::take(&mut self.samples)}));
            self.stage += 1;
            self.frame = 0;
            if self.stage < 3 {
                (self.doc, self.shapes) = Self::scene([100, 1000, 5000][self.stage]);
            }
            self.save_report();
        }
        ctx.request_repaint();
    }
}
fn main() -> eframe::Result {
    let output = PathBuf::from(std::env::args().nth(1).expect("output directory required"));
    std::fs::create_dir_all(&output).unwrap();
    let (doc, shapes) = App::scene(100);
    eframe::run_native(
        "Curvex GPU qualification",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: egui::ViewportBuilder::default().with_inner_size([1000., 1000.]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let adapter = format!(
                "{:?}",
                cc.wgpu_render_state
                    .as_ref()
                    .expect("wgpu renderer required")
                    .adapter
                    .get_info()
            );
            eprintln!("GPU adapter: {adapter}");
            Ok(Box::new(App {
                doc,
                shapes,
                stage: 0,
                frame: 0,
                samples: vec![],
                report: vec![],
                previous: Instant::now(),
                output,
                adapter,
                captures: 0,
                start: Instant::now(),
            }))
        }),
    )
}
