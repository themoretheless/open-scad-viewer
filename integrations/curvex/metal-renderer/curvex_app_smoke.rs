//! Boots the real app factory with isolated persistence, then exercises its
//! installed renderer through a small foreground qualification shape.
use curvex::{
    document::Document,
    shape::{Color, Contour, FillStyle, PathSegment, Shape, ShapeData, Vec2},
    ui::{canvas::CanvasTransform, shape_render::draw_shape_cached},
};
use eframe::App as _;
struct Smoke {
    inner: curvex::app::CurvexApp,
    state: eframe::egui_wgpu::RenderState,
    doc: Document,
    shape: Shape,
    passes: u32,
    output: std::path::PathBuf,
    enabled: bool,
}
impl eframe::App for Smoke {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.inner.ui(ui, frame);
        let ctx = ui.ctx();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("GPU smoke overlay"),
        ));
        draw_shape_cached(
            &painter,
            &self.shape,
            &CanvasTransform::new(egui::pos2(80., 120.), egui::Vec2::ZERO, 2.),
            Some(&self.doc),
        );
        self.passes += 1;
        let stats =
            curvex_metal_renderer::egui_integration::cache_stats(&self.state.renderer.read());
        if self.passes >= 30 && (!self.enabled || stats.is_some_and(|s| s.0.uploads > 0))
            || self.passes >= 300
        {
            std::fs::write(&self.output,serde_json::to_vec_pretty(&serde_json::json!({"ui_passes":self.passes,"gpu_cache_enabled":self.enabled,"uploads":stats.map(|s|s.0.uploads).unwrap_or(0),"hits":stats.map(|s|s.0.hits).unwrap_or(0),"target_format":format!("{:?}",self.state.target_format),"adapter":format!("{:?}",self.state.adapter.get_info()),"pixels_per_point":ctx.pixels_per_point()})).unwrap()).unwrap();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}
fn main() -> eframe::Result {
    let output = std::path::PathBuf::from(std::env::args().nth(1).expect("report path"));
    eframe::run_native(
        "Curvex Metal production smoke",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: egui::ViewportBuilder::default().with_inner_size([900., 700.]),
            persist_window: false,
            persistence_path: Some("/private/tmp/curvex-metal-production-smoke.ron".into()),
            ..Default::default()
        },
        Box::new(move |cc| {
            let inner = curvex::app::CurvexApp::new(cc);
            let enabled = curvex_metal_renderer::egui_integration::is_enabled(&cc.egui_ctx);
            let mut shape = Shape::new(
                "Metal smoke".into(),
                ShapeData::Compound {
                    contours: vec![Contour {
                        start: Vec2::new(0., 0.),
                        segments: vec![
                            PathSegment::Line {
                                to: Vec2::new(70., 0.),
                            },
                            PathSegment::Line {
                                to: Vec2::new(70., 50.),
                            },
                            PathSegment::Line {
                                to: Vec2::new(0., 50.),
                            },
                        ],
                    }],
                },
            );
            shape.fill = Some(FillStyle::Solid {
                color: Color::new(20, 160, 230, 255),
            });
            shape.stroke.width = 0.;
            let mut doc = Document::new("Smoke overlay".into(), 100., 100.);
            doc.add_shape(shape.clone());
            Ok(Box::new(Smoke {
                inner,
                state: cc.wgpu_render_state.clone().expect("wgpu state"),
                doc,
                shape,
                passes: 0,
                output,
                enabled,
            }))
        }),
    )
}
