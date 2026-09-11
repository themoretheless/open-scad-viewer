//! Temporary GPU vs CPU sampling benchmark (may be removed after qualification).
use sdf_core::{polygonize, polygonize_accelerated, Acceleration, Field, Grid};
use std::time::Instant;

/// Outward UV-sphere mesh with a controlled triangle count.
fn uv_sphere(center: [f64; 3], radius: f64, rings: usize, sectors: usize) -> geometry_ops::Triangles {
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

fn main() {
    let field = Field::SmoothUnion {
        a: Box::new(Field::Sphere { center: [0., 0., 0.], radius: 10. }),
        b: Box::new(Field::Box { center: [8., 0., 0.], half_size: [6., 6., 6.] }),
        radius: 3.,
    };
    let grid = Grid { min: [-12., -12., -12.], max: [16., 12., 12.], cells: [64, 64, 64] };
    let mut cpu_times = Vec::new();
    let mut gpu_times = Vec::new();
    for i in 0..6 {
        let (acceleration, label) = if i % 2 == 0 { (Acceleration::Cpu, "cpu") } else { (Acceleration::Gpu, "gpu") };
        let start = Instant::now();
        let mesh = polygonize_accelerated(&field, &grid, acceleration).unwrap();
        let ms = start.elapsed().as_secs_f64() * 1000.;
        (if label == "cpu" { &mut cpu_times } else { &mut gpu_times }).push(ms);
        if i == 0 {
            let reference = polygonize(&field, &grid).unwrap();
            println!("triangles cpu={} alt={}", reference.indices.len() / 3, mesh.indices.len() / 3);
        }
    }
    cpu_times.sort_by(f64::total_cmp);
    gpu_times.sort_by(f64::total_cmp);
    println!("primitive field: cpu median {:.1}ms gpu median {:.1}ms",
        cpu_times[cpu_times.len() / 2], gpu_times[gpu_times.len() / 2]);

    // Mesh-distance field (the dominant production case; budget-capped grid).
    let mesh = uv_sphere([0., 0., 0.], 10., 17, 34);
    let triangles = mesh.indices.len() / 3;
    let field = Field::from_triangles(mesh, false).unwrap();
    let grid = Grid { min: [-12., -12., -12.], max: [12., 12., 12.], cells: [16, 16, 16] };
    let mut cpu_times = Vec::new();
    let mut gpu_times = Vec::new();
    for i in 0..4 {
        let acceleration = if i % 2 == 0 { Acceleration::Cpu } else { Acceleration::Gpu };
        let start = Instant::now();
        let out = polygonize_accelerated(&field, &grid, acceleration).unwrap();
        let ms = start.elapsed().as_secs_f64() * 1000.;
        if i == 0 {
            let reference = polygonize(&field, &grid).unwrap();
            assert_eq!(reference.indices, out.indices, "cpu baseline matches");
        }
        (if i % 2 == 0 { &mut cpu_times } else { &mut gpu_times }).push(ms);
    }
    cpu_times.sort_by(f64::total_cmp);
    gpu_times.sort_by(f64::total_cmp);
    println!("mesh field ({} tris, 16^3): cpu median {:.1}ms gpu median {:.1}ms",
        triangles, cpu_times[cpu_times.len() / 2], gpu_times[gpu_times.len() / 2]);
}
