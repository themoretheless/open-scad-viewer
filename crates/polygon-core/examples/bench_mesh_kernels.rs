//! Mesh kernel microbenchmarks via rbench (use a release build).
//!
//! ```sh
//! cargo run --release -p polygon-core --example bench_mesh_kernels -- --profile quick
//! ```
//!
//! Workload: a UV sphere with stride-6 (position + normal) f32 vertices at the
//! host import budget scale (~100k triangles), exercised through the same
//! entry points the `geometry-bridge` ABI reaches.
use polygon_core::solid::bvh::build_mesh_bvh;
use polygon_core::solid::bvh_query::{Query, raycast};
use polygon_core::solid::edges::extract_semantic_edges;
use rbench::{DropPolicy, Suite};
use std::collections::BTreeSet;

const STRIDE: usize = 6;

fn sphere(rings: usize, segments: usize) -> (Vec<f32>, Vec<u32>) {
    let mut vertices = Vec::with_capacity((rings + 1) * (segments + 1) * STRIDE);
    for i in 0..=rings {
        let theta = std::f64::consts::PI * i as f64 / rings as f64;
        for j in 0..=segments {
            let phi = 2. * std::f64::consts::PI * j as f64 / segments as f64;
            let n = [
                theta.sin() * phi.cos(),
                theta.cos(),
                theta.sin() * phi.sin(),
            ];
            for k in 0..3 {
                vertices.push((n[k] * 10.) as f32);
            }
            for k in 0..3 {
                vertices.push(n[k] as f32);
            }
        }
    }
    let mut indices = Vec::with_capacity(rings * segments * 6);
    let row = (segments + 1) as u32;
    for i in 0..rings as u32 {
        for j in 0..segments as u32 {
            let a = i * row + j;
            let b = a + row;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    (vertices, indices)
}

fn rays(count: usize) -> Vec<([f64; 3], [f64; 3])> {
    (0..count)
        .map(|i| {
            let t = i as f64 / count as f64;
            let angle = t * std::f64::consts::TAU * 7.;
            let origin = [30. * angle.cos(), 20. * (t - 0.5), 30. * angle.sin()];
            let direction = [-origin[0], -origin[1] * 0.5, -origin[2]];
            (origin, direction)
        })
        .collect()
}

fn main() -> rbench::Result<()> {
    let (vertices, indices) = sphere(200, 250);
    let triangles = indices.len() / 3;
    let mut suite = Suite::new("polygon-core");
    suite
        .bench_with_input(
            "bvh/build",
            || (vertices.clone(), indices.clone()),
            |(vertices, indices)| build_mesh_bvh(vertices, indices, STRIDE, 8),
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", triangles as f64);
    suite
        .bench_with_input(
            "bvh/raycast",
            || {
                (
                    build_mesh_bvh(&vertices, &indices, STRIDE, 8),
                    vertices.clone(),
                    indices.clone(),
                    rays(256),
                    BTreeSet::new(),
                )
            },
            |(bvh, vertices, indices, rays, excluded)| {
                let mut hits = 0usize;
                for &(origin, direction) in rays.iter() {
                    let query = Query {
                        origin,
                        direction,
                        min_t: 0.,
                        max_t: f64::INFINITY,
                        local_from_world: None,
                        excluded: &*excluded,
                    };
                    if let Ok(Some(_)) = raycast(bvh, vertices, indices, STRIDE, &query) {
                        hits += 1;
                    }
                }
                hits
            },
            DropPolicy::InsideTiming,
        )
        .parameter("rays", 256.);
    suite
        .bench_with_input(
            "edges/semantic",
            || (vertices.clone(), indices.clone()),
            |(vertices, indices)| {
                extract_semantic_edges(vertices, indices, &[], &[], true, 30f64.to_radians().cos())
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", triangles as f64);
    suite.main()
}
