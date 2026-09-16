//! GPU/CUDA vs CPU sampling benchmark (may be removed after qualification).
//! `cargo run --release -p sdf-core --features cuda --example bench_gpu`.
use sdf_core::{Acceleration, Field, Grid, polygonize, polygonize_accelerated};
use std::time::Instant;

/// Outward UV-sphere mesh with a controlled triangle count.
fn uv_sphere(
    center: [f64; 3],
    radius: f64,
    rings: usize,
    sectors: usize,
) -> geometry_ops::Triangles {
    let mut positions = Vec::new();
    for r in 0..=rings {
        let phi = std::f64::consts::PI * r as f64 / rings as f64;
        for s in 0..sectors {
            let theta = 2. * std::f64::consts::PI * s as f64 / sectors as f64;
            positions.push(center[0] + radius * phi.sin() * theta.cos());
            positions.push(center[1] + radius * phi.sin() * theta.sin());
            positions.push(center[2] + radius * phi.cos());
        }
    }
    let mut indices = Vec::new();
    for r in 0..rings {
        for s in 0..sectors {
            let a = r * sectors + s;
            let b = r * sectors + (s + 1) % sectors;
            let c = (r + 1) * sectors + s;
            let d = (r + 1) * sectors + (s + 1) % sectors;
            if r > 0 {
                indices.extend_from_slice(&[a, c, b]);
            }
            if r + 1 < rings {
                indices.extend_from_slice(&[b, c, d]);
            }
        }
    }
    geometry_ops::Triangles { positions, indices }
}

/// Placements to compare: CPU reference, wgpu shader and (feature `cuda`) the
/// CUDA driver port. `SDF_BENCH_MODES=cpu,cuda` narrows the set.
fn modes() -> Vec<Acceleration> {
    let mut modes = vec![Acceleration::Cpu, Acceleration::Gpu];
    if cfg!(feature = "cuda") {
        modes.push(Acceleration::Cuda);
    }
    if let Ok(filter) = std::env::var("SDF_BENCH_MODES") {
        let wanted: Vec<Acceleration> = filter.split(',').filter_map(Acceleration::parse).collect();
        modes.retain(|mode| wanted.contains(mode));
    }
    modes
}

fn median(times: &mut [f64]) -> f64 {
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

/// Runs every placement `rounds` times (interleaved) and prints the medians.
fn compare(label: &str, field: &Field, grid: &Grid, rounds: usize) {
    let modes = modes();
    let reference = polygonize(field, grid).unwrap();
    let mut times = vec![Vec::new(); modes.len()];
    let mut triangles = vec![0usize; modes.len()];
    for round in 0..rounds {
        for (index, &acceleration) in modes.iter().enumerate() {
            let start = Instant::now();
            let mesh = polygonize_accelerated(field, grid, acceleration).unwrap();
            times[index].push(start.elapsed().as_secs_f64() * 1000.);
            triangles[index] = mesh.indices.len() / 3;
            if round == 0 && acceleration == Acceleration::Cpu {
                assert_eq!(reference.indices, mesh.indices, "cpu baseline matches");
            }
        }
    }
    let summary: Vec<String> = modes
        .iter()
        .zip(times.iter_mut())
        .zip(&triangles)
        .map(|((mode, times), tris)| {
            format!(
                "{} median {:.1}ms ({tris} tris)",
                mode.label(),
                median(times)
            )
        })
        .collect();
    println!("{label}: {}", summary.join(", "));
}

fn main() {
    #[cfg(feature = "cuda")]
    println!(
        "cuda device: {}",
        sdf_core::cuda::device_name().unwrap_or_else(|| "none (falls back to wgpu/cpu)".into())
    );
    let field = Field::SmoothUnion {
        a: Box::new(Field::Sphere {
            center: [0., 0., 0.],
            radius: 10.,
        }),
        b: Box::new(Field::Box {
            center: [8., 0., 0.],
            half_size: [6., 6., 6.],
        }),
        radius: 3.,
    };
    let grid = Grid {
        min: [-12., -12., -12.],
        max: [16., 12., 12.],
        cells: [64, 64, 64],
    };
    compare("primitive field (64^3)", &field, &grid, 3);

    // Mesh-distance field (the dominant production case; budget-capped grid).
    let mesh = uv_sphere([0., 0., 0.], 10., 17, 34);
    let triangles = mesh.indices.len() / 3;
    let field = Field::from_triangles(mesh, false).unwrap();
    let grid = Grid {
        min: [-12., -12., -12.],
        max: [12., 12., 12.],
        cells: [16, 16, 16],
    };
    compare(
        &format!("mesh field ({triangles} tris, 16^3)"),
        &field,
        &grid,
        2,
    );
}
