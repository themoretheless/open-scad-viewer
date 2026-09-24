//! Headless rasterizer smoke tests. Skipped (with a notice) on machines
//! without an adapter, matching the compute-kernel test convention.

use raster_core::gpu_compute::GpuContext;
use raster_core::rasterizer::Rasterizer;
use raster_core::uniform::{ObjectUniform, SceneUniform};
use raster_core::wgpu;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;

/// Orthographic box over [-1, 1]² with the eye at z = 1; column-major
/// translation that brings the z = 0 model plane into the view volume.
fn test_scene() -> SceneUniform {
    let mut view_projection = [0.0f32; 16];
    view_projection[0] = 1.0;
    view_projection[5] = 1.0;
    view_projection[10] = 1.0;
    view_projection[15] = 1.0;
    view_projection[14] = -1.0; // translate z by -1 (eye at +1)
    SceneUniform::new(view_projection, [0.0, 0.0, 1.0, 1.0])
}

/// A triangle on the z = 0 plane; the morph source shifts it +0.3 in x, so
/// weight 0 shows the shifted (source) shape and weight 1 the vertex-buffer
/// (target) shape.
fn triangle() -> (Vec<f32>, Vec<u32>, Vec<f32>) {
    let target = [(-0.6f32, -0.4f32), (0.6, -0.4), (0.0, 0.6)];
    let mut vertices = Vec::with_capacity(18);
    let mut morph = Vec::with_capacity(9);
    for (x, y) in target {
        vertices.extend_from_slice(&[x, y, 0.0, 0.0, 0.0, 1.0]);
        morph.extend_from_slice(&[x + 0.3, y, 0.0]);
    }
    (vertices, vec![0, 1, 2], morph)
}

fn pixel(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
    let start = ((y * WIDTH + x) * 4) as usize;
    [bytes[start], bytes[start + 1], bytes[start + 2], bytes[start + 3]]
}

fn red_at(bytes: &[u8], x: u32, y: u32) -> bool {
    let [r, _g, b, _a] = pixel(bytes, x, y);
    r > 100 && r > b + 60
}

fn background_at(bytes: &[u8], x: u32, y: u32) -> bool {
    pixel(bytes, x, y) == [13, 15, 20, 255]
}

fn colored_uniform(r: f32, g: f32, b: f32) -> ObjectUniform {
    let mut uniform = ObjectUniform::default();
    uniform.color = [r, g, b, 1.0];
    uniform.style = [1.0, 0.0, 0.0, 0.0];
    uniform
}

#[test]
fn morph_weight_moves_the_triangle_on_the_gpu() {
    let Some(context) = GpuContext::new() else {
        eprintln!("raster-core: no GPU adapter available, skipping render test");
        return;
    };
    eprintln!("raster-core: rendering on {}", context.backend_label());
    let mut rasterizer = Rasterizer::new(context, wgpu::TextureFormat::Rgba8Unorm);
    rasterizer.set_scene(&test_scene());

    let (vertices, indices, morph) = triangle();
    let mesh = rasterizer.create_mesh(&vertices, &indices, &colored_uniform(1.0, 0.0, 0.0), Some(&morph));

    // Weight 0 shows the slot-1 source positions (shifted right).
    rasterizer.set_morph_weight(&mesh, 0.0);
    let at_source = rasterizer.render_to_rgba(WIDTH, HEIGHT, &[&mesh]);
    assert!(red_at(&at_source, 55, 40), "source-only area is covered at weight 0");
    assert!(background_at(&at_source, 16, 40), "target-only area stays background at weight 0");

    // Weight 1 shows the vertex-buffer target positions.
    rasterizer.set_morph_weight(&mesh, 1.0);
    let at_target = rasterizer.render_to_rgba(WIDTH, HEIGHT, &[&mesh]);
    assert!(red_at(&at_target, 16, 40), "target-only area is covered at weight 1");
    assert!(background_at(&at_target, 55, 40), "source-only area returns to background at weight 1");

    // Halfway blend: midpoint covered, both extreme-only areas uncovered.
    rasterizer.set_morph_weight(&mesh, 0.5);
    let at_half = rasterizer.render_to_rgba(WIDTH, HEIGHT, &[&mesh]);
    assert!(red_at(&at_half, 36, 40), "mid blend covers the midpoint");
    assert!(background_at(&at_half, 16, 40), "left edge retreats at half blend");
    assert!(background_at(&at_half, 55, 40), "right edge retreats at half blend");
}

#[test]
fn mesh_without_morph_source_draws_its_target_vertices() {
    let Some(context) = GpuContext::new() else {
        eprintln!("raster-core: no GPU adapter available, skipping render test");
        return;
    };
    let mut rasterizer = Rasterizer::new(context, wgpu::TextureFormat::Rgba8Unorm);
    rasterizer.set_scene(&test_scene());

    let (vertices, indices, _morph) = triangle();
    // No morph source: slot 1 binds the zero dummy, and the uploaded morph
    // weight stays 0, so the target vertices draw unchanged. (Writing a
    // nonzero weight without a morph source would blend toward the origin —
    // callers must not do that; the browser renderer enforces the same
    // invariant by resetting the weight when a morph retires.)
    let mesh = rasterizer.create_mesh(&vertices, &indices, &colored_uniform(0.0, 0.0, 1.0), None);
    let first = rasterizer.render_to_rgba(WIDTH, HEIGHT, &[&mesh]);
    let second = rasterizer.render_to_rgba(WIDTH, HEIGHT, &[&mesh]);
    assert_eq!(first, second, "repeated renders are deterministic");
    assert!(pixel(&first, 16, 40)[2] > 100, "target triangle covers the left area in blue");
    assert!(background_at(&first, 55, 40), "source-only area stays background");
}
