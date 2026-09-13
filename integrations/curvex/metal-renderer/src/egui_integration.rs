//! egui adapter: preserves painter order and host clipping, with renderer-owned
//! GPU state. Call `install` for each newly created renderer/device.
use crate::{
    Camera, CameraBinding, ResidentMesh, RetainedPipeline,
    cache::{GeometryCache, MeshData},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
const CONTEXT_KEY: &str = "curvex.retained-gpu.config.v1";
#[derive(Clone, Copy)]
struct Config {
    budget: u64,
    max_buffer: u64,
    dithering: bool,
}
struct Resources {
    pipeline: RetainedPipeline,
    cache: GeometryCache,
    cameras: HashMap<[u32; 8], Arc<CameraBinding>>,
    frame: u64,
}

/// Unsupported targets return false; callers keep their ordinary egui mesh path.
/// The host must supply its exact render target format, MSAA and dithering.
pub fn install(
    ctx: &egui::Context,
    renderer: &mut egui_wgpu::Renderer,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    samples: u32,
    dithering: bool,
) -> bool {
    disable(ctx);
    let Ok(pipeline) = RetainedPipeline::new(device, format, samples.max(1)) else {
        return false;
    };
    let budget = 64 * 1024 * 1024;
    renderer.callback_resources.insert(Resources {
        pipeline,
        cache: GeometryCache::new(budget, 8192),
        cameras: HashMap::new(),
        frame: u64::MAX,
    });
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(CONTEXT_KEY),
            Config {
                budget,
                max_buffer: device.limits().max_buffer_size,
                dithering,
            },
        )
    });
    true
}

/// Explicit opt-out for renderer teardown or host fallback. Existing prepared
/// frame leases remain alive until their callbacks are released.
pub fn disable(ctx: &egui::Context) {
    ctx.data_mut(|data| {
        data.remove::<Config>(egui::Id::new(CONTEXT_KEY));
        data.remove::<LastBatch>(egui::Id::new("curvex.gpu.last-batch"));
        data.remove::<FrameAdmission>(egui::Id::new("curvex.gpu.frame-budget"));
    });
}

#[derive(Clone)]
struct LastBatch {
    frame: u64,
    layer: egui::LayerId,
    index: usize,
    checked_until: usize,
    clip: egui::Rect,
    camera: [u8; 32],
    data: Arc<Mutex<Vec<Arc<MeshData>>>>,
}
#[derive(Clone, Default)]
struct FrameAdmission {
    frame: u64,
    bytes: u64,
}

/// Append-only painter integration: previously submitted shapes must not be
/// rewritten with `Painter::set` after subsequent retained paints are appended.
/// Curvex canvas drawing follows this ordering contract.
pub fn paint(painter: &egui::Painter, mesh: Arc<MeshData>, mut camera: Camera) -> bool {
    let Some(config) = painter
        .ctx()
        .data(|data| data.get_temp::<Config>(egui::Id::new(CONTEXT_KEY)))
    else {
        return false;
    };
    if mesh.byte_size() > config.budget || mesh.byte_size() > config.max_buffer {
        return false;
    }
    camera.dithering = config.dithering;
    let rect = painter.ctx().viewport_rect();
    camera.screen = [rect.width(), rect.height()];
    if camera.bytes().is_err() {
        return false;
    }
    let frame = painter.ctx().cumulative_pass_nr();
    let admitted = painter.ctx().data_mut(|data| {
        let budget = data
            .get_temp_mut_or_default::<FrameAdmission>(egui::Id::new("curvex.gpu.frame-budget"));
        if budget.frame != frame {
            budget.frame = frame;
            budget.bytes = 0;
        }
        if mesh.byte_size() > config.budget.saturating_sub(budget.bytes) {
            return false;
        }
        budget.bytes += mesh.byte_size();
        true
    });
    if !admitted {
        return false;
    }
    let layer = painter.layer_id();
    let next = painter
        .ctx()
        .graphics(|g| g.get(layer).map_or(0, |list| list.next_idx().0));
    let key = camera.bytes().unwrap();
    let previous = painter
        .ctx()
        .data(|data| data.get_temp::<LastBatch>(egui::Id::new("curvex.gpu.last-batch")));
    if let Some(mut previous) = previous
        && previous.frame == frame
            && previous.layer == layer
            && previous.index < next
            && previous.checked_until <= next
            && painter.ctx().graphics(|g| g.get(layer).is_some_and(|list| list.all_entries().skip(previous.checked_until).all(|entry| {
                matches!(&entry.shape, egui::Shape::Noop)
                    || matches!(&entry.shape, egui::Shape::Path(path) if path.fill == egui::Color32::TRANSPARENT && path.stroke.is_empty())
            })))
            && previous.clip == painter.clip_rect()
            && previous.camera == key
        {
            previous.data.lock().unwrap().push(mesh);
            // Keep the frontier monotonic: rescanning from the first callback
            // makes a run of N empty paths quadratic in the document size.
            previous.checked_until = next;
            painter.ctx().data_mut(|store| store.insert_temp(egui::Id::new("curvex.gpu.last-batch"), previous));
            return true;
    }
    let data = Arc::new(Mutex::new(vec![mesh]));
    let index = painter.add(egui_wgpu::Callback::new_paint_callback(
        rect,
        Draw {
            data: data.clone(),
            camera,
            frame,
            prepared: Mutex::new(None),
        },
    ));
    painter.ctx().data_mut(|store| {
        store.insert_temp(
            egui::Id::new("curvex.gpu.last-batch"),
            LastBatch {
                frame,
                layer,
                index: index.0,
                checked_until: index.0 + 1,
                clip: painter.clip_rect(),
                camera: key,
                data,
            },
        )
    });
    true
}
struct Prepared {
    mesh: Arc<ResidentMesh>,
    camera: Arc<CameraBinding>,
}
struct Draw {
    data: Arc<Mutex<Vec<Arc<MeshData>>>>,
    camera: Camera,
    frame: u64,
    prepared: Mutex<Option<Prepared>>,
}
impl egui_wgpu::CallbackTrait for Draw {
    fn prepare(
        &self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let state = resources
            .get_mut::<Resources>()
            .expect("retained renderer lifecycle");
        if state.frame != self.frame {
            state.cameras.clear();
            state.frame = self.frame;
        }
        let mut camera = self.camera;
        camera.screen = [
            screen.size_in_pixels[0] as f32 / screen.pixels_per_point,
            screen.size_in_pixels[1] as f32 / screen.pixels_per_point,
        ];
        let bytes = camera.bytes().expect("validated camera");
        let key = std::array::from_fn(|i| {
            u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap())
        });
        let binding = state
            .cameras
            .entry(key)
            .or_insert_with(|| {
                Arc::new(
                    state
                        .pipeline
                        .camera(device, camera)
                        .expect("validated camera"),
                )
            })
            .clone();
        let mesh = state
            .cache
            .prepare_batch(device, &self.data.lock().unwrap())
            .expect("pre-admitted retained mesh");
        *self.prepared.lock().unwrap() = Some(Prepared {
            mesh,
            camera: binding,
        });
        Vec::new()
    }
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let prepared = self.prepared.lock().unwrap();
        let prepared = prepared.as_ref().expect("callback prepare precedes paint");
        resources
            .get::<Resources>()
            .expect("retained renderer lifecycle")
            .pipeline
            .draw(pass, &prepared.mesh, &prepared.camera);
    }
}

pub fn is_enabled(ctx: &egui::Context) -> bool {
    ctx.data(|data| {
        data.get_temp::<Config>(egui::Id::new(CONTEXT_KEY))
            .is_some()
    })
}

/// Diagnostics for qualification and host telemetry; no GPU readback involved.
pub fn cache_stats(renderer: &egui_wgpu::Renderer) -> Option<(crate::cache::CacheStats, u64)> {
    renderer
        .callback_resources
        .get::<Resources>()
        .map(|state| (state.cache.stats(), state.cache.retained_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (egui::Context, Arc<MeshData>, Camera) {
        let ctx = egui::Context::default();
        ctx.data_mut(|d| {
            d.insert_temp(
                egui::Id::new(CONTEXT_KEY),
                Config {
                    budget: 192,
                    max_buffer: 1024,
                    dithering: false,
                },
            )
        });
        let mesh = MeshData::new(
            vec![
                crate::Vertex {
                    position: [0., 0.],
                    color: [255; 4],
                },
                crate::Vertex {
                    position: [10., 0.],
                    color: [255; 4],
                },
                crate::Vertex {
                    position: [0., 10.],
                    color: [255; 4],
                },
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        (
            ctx,
            mesh,
            Camera {
                screen: [256., 256.],
                origin: [0., 0.],
                offset: [0., 0.],
                zoom: 1.,
                dithering: false,
            },
        )
    }
    fn begin(ctx: &egui::Context) {
        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(256., 256.),
            )),
            ..Default::default()
        });
    }
    #[test]
    fn batches_preserve_visible_barriers_and_clip_changes() {
        let (ctx, mesh, camera) = fixture();
        begin(&ctx);
        let painter = ctx.layer_painter(egui::LayerId::background());
        assert!(paint(&painter, mesh.clone(), camera));
        painter.add(egui::Shape::Noop);
        assert!(paint(&painter, mesh.clone(), camera));
        painter.rect_filled(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(5., 5.)),
            0,
            egui::Color32::RED,
        );
        assert!(paint(&painter, mesh.clone(), camera));
        let clipped = painter.with_clip_rect(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(16., 16.),
        ));
        assert!(paint(&clipped, mesh, camera));
        let output = ctx.end_pass();
        assert_eq!(
            output
                .shapes
                .iter()
                .filter(|s| matches!(s.shape, egui::Shape::Callback(_)))
                .count(),
            3
        );
    }
    #[test]
    fn frame_budget_falls_back_and_resets_without_accepting_invalid_camera() {
        let (ctx, mesh, camera) = fixture();
        begin(&ctx);
        let painter = ctx.layer_painter(egui::LayerId::background());
        assert!(!paint(
            &painter,
            mesh.clone(),
            Camera {
                zoom: f32::NAN,
                ..camera
            }
        ));
        for _ in 0..4 {
            assert!(paint(&painter, mesh.clone(), camera));
        }
        assert!(!paint(&painter, mesh.clone(), camera));
        let _ = ctx.end_pass();
        begin(&ctx);
        assert!(paint(&painter, mesh.clone(), camera));
        disable(&ctx);
        assert!(!paint(&painter, mesh, camera));
        let _ = ctx.end_pass();
    }
}
