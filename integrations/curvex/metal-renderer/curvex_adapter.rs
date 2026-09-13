// Included into shape_render.rs by prepare_gpu.py. The CPU snapshot cache uses
// geometry allocation identity plus premultiplied color. Weak ownership prevents
// stale pointer reuse; a new tessellation or recolor creates a new GPU identity.
struct OsvGpuSnapshot {
    source: std::rc::Weak<dyn std::any::Any>,
    mesh: std::sync::Arc<curvex_metal_renderer::cache::MeshData>,
    age: u64,
}
#[derive(Default)]
struct OsvGpuSnapshots {
    entries: std::collections::HashMap<(usize, u32), OsvGpuSnapshot>,
    ages: std::collections::BTreeMap<u64, (usize, u32)>,
    clock: u64,
    bytes: u64,
}
thread_local! {
    static OSV_GPU_SNAPSHOTS: RefCell<OsvGpuSnapshots> = RefCell::new(OsvGpuSnapshots::default());
}
fn osv_try_gpu_solid(
    painter: &Painter,
    geometry: &Rc<osv_geometry::render::tess::VertexBuffers<Pos2, u32>>,
    color: egui::Color32,
    ct: &CanvasTransform,
) -> bool {
    if !curvex_metal_renderer::egui_integration::is_enabled(painter.ctx()) {
        return false;
    }
    let bytes = geometry.vertices.len() as u64 * 12 + geometry.indices.len() as u64 * 4;
    const BUDGET: u64 = 32 * 1024 * 1024;
    if bytes > BUDGET || geometry.vertices.is_empty() || geometry.indices.is_empty() {
        return false;
    }
    let mesh = osv_gpu_snapshot(
        geometry,
        u32::from_le_bytes(color.to_array()),
        bytes,
        || {
            curvex_metal_renderer::cache::MeshData::new(
                geometry
                    .vertices
                    .iter()
                    .map(|p| curvex_metal_renderer::Vertex {
                        position: [p.x, p.y],
                        color: color.to_array(),
                    })
                    .collect(),
                geometry.indices.clone(),
            )
            .ok()
        },
    );
    let Some(mesh) = mesh else {
        return false;
    };
    curvex_metal_renderer::egui_integration::paint(
        painter,
        mesh,
        curvex_metal_renderer::Camera {
            screen: [1., 1.],
            origin: [ct.origin.x, ct.origin.y],
            offset: [ct.offset.x, ct.offset.y],
            zoom: ct.zoom,
            dithering: true,
        },
    )
}

fn osv_gpu_snapshot<T: std::any::Any>(
    source: &Rc<T>,
    discriminator: u32,
    bytes: u64,
    build: impl FnOnce() -> Option<std::sync::Arc<curvex_metal_renderer::cache::MeshData>>,
) -> Option<std::sync::Arc<curvex_metal_renderer::cache::MeshData>> {
    const BUDGET: u64 = 32 * 1024 * 1024;
    if bytes > BUDGET {
        return None;
    }
    let key = (Rc::as_ptr(source) as usize, discriminator);
    OSV_GPU_SNAPSHOTS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.clock == u64::MAX {
            *cache = OsvGpuSnapshots::default();
        }
        let age = cache.clock;
        cache.clock += 1;
        if let Some(entry) = cache.entries.get(&key) {
            if entry.source.upgrade().is_some() {
                let mesh = entry.mesh.clone();
                let old_age = entry.age;
                cache.ages.remove(&old_age);
                cache.ages.insert(age, key);
                cache.entries.get_mut(&key).unwrap().age = age;
                return Some(mesh);
            }
        }
        let mesh = build()?;
        if let Some(old) = cache.entries.remove(&key) {
            cache.bytes -= old.mesh.byte_size();
            cache.ages.remove(&old.age);
        }
        while cache.bytes + bytes > BUDGET || cache.entries.len() >= 8192 {
            let (_, old_key) = cache.ages.pop_first()?;
            let old = cache.entries.remove(&old_key)?;
            cache.bytes -= old.mesh.byte_size();
        }
        cache.bytes += bytes;
        cache.ages.insert(age, key);
        cache.entries.insert(
            key,
            OsvGpuSnapshot {
                source: Rc::downgrade(&(source.clone() as Rc<dyn std::any::Any>)),
                mesh: mesh.clone(),
                age,
            },
        );
        Some(mesh)
    })
}

fn osv_try_gpu_gradient(
    painter: &Painter,
    sampled: &Rc<osv_geometry::attribute_mesh::SampledMesh>,
    ct: &CanvasTransform,
) -> bool {
    if !curvex_metal_renderer::egui_integration::is_enabled(painter.ctx()) {
        return false;
    }
    let bytes = sampled.positions.len() as u64 * 12 + sampled.indices.len() as u64 * 4;
    let mesh = osv_gpu_snapshot(sampled, 0, bytes, || {
        curvex_metal_renderer::cache::MeshData::new(
            sampled
                .positions
                .iter()
                .zip(&sampled.values)
                .map(|(p, v)| curvex_metal_renderer::Vertex {
                    position: [p[0] as f32, p[1] as f32],
                    color: v.map(|x| x.round().clamp(0., 255.) as u8),
                })
                .collect(),
            sampled.indices.clone(),
        )
        .ok()
    });
    let Some(mesh) = mesh else {
        return false;
    };
    curvex_metal_renderer::egui_integration::paint(
        painter,
        mesh,
        curvex_metal_renderer::Camera {
            screen: [1., 1.],
            origin: [ct.origin.x, ct.origin.y],
            offset: [ct.offset.x, ct.offset.y],
            zoom: ct.zoom,
            dithering: true,
        },
    )
}

/// Payload diagnostics; budgets exclude map/node and driver allocation overhead.
pub fn osv_gpu_cache_payloads() -> (usize, usize, usize, u64) {
    let geometry = RENDER_CACHE.with(|cache| {
        let c = cache.borrow();
        (c.entries.len(), c.bytes)
    });
    let snapshots = OSV_GPU_SNAPSHOTS.with(|cache| {
        let c = cache.borrow();
        (c.entries.len(), c.bytes)
    });
    (geometry.0, geometry.1, snapshots.0, snapshots.1)
}
