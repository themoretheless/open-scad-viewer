//! Temporary lattice benchmark (may be removed after qualification).
use geometry_bridge::mesh_shell;
use polygon_core::solid::primitives::cube;
use sdf_core::Acceleration;
use std::time::Instant;

#[derive(Clone, Copy)]
struct Mode {
    name: &'static str,
    acceleration: Acceleration,
}

fn main() {
    #[cfg(feature = "gpu")]
    println!(
        "wgpu backend: {}",
        geometry_bridge::lattice_gpu::backend_label().unwrap_or("none (falls back to cpu)")
    );

    let mesh = cube([40., 40., 40.], false).unwrap();
    let mut nodes = Vec::new();
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                nodes.push([x as f64 * 20.0, y as f64 * 20.0, z as f64 * 20.0]);
            }
        }
    }
    let id = |x: usize, y: usize, z: usize| x * 9 + y * 3 + z;
    let mut edges = Vec::new();
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                if x < 2 {
                    edges.push([id(x, y, z), id(x + 1, y, z)]);
                }
                if y < 2 {
                    edges.push([id(x, y, z), id(x, y + 1, z)]);
                }
                if z < 2 {
                    edges.push([id(x, y, z), id(x, y, z + 1)]);
                }
            }
        }
    }
    println!("nodes={} edges={}", nodes.len(), edges.len());
    let modes = [
        Mode {
            name: "cpu",
            acceleration: Acceleration::Cpu,
        },
        Mode {
            name: "auto",
            acceleration: Acceleration::Auto,
        },
        Mode {
            name: "gpu",
            acceleration: Acceleration::Gpu,
        },
        Mode {
            name: "cuda",
            acceleration: Acceleration::Cuda,
        },
    ];

    for organic in [false, true] {
        let reference = mesh_shell::lattice(
            &mesh,
            nodes.clone(),
            edges.clone(),
            3.5,
            3.5,
            1.5,
            organic,
            false,
            0.,
            false,
        )
        .unwrap();
        let mut summary = Vec::new();
        for mode in &modes {
            let mut times = Vec::new();
            let mut triangles = 0usize;
            let mut volume_delta = 0.;
            for _ in 0..3 {
                let start = Instant::now();
                let out = mesh_shell::lattice_accelerated(
                    &mesh,
                    nodes.clone(),
                    edges.clone(),
                    3.5,
                    3.5,
                    1.5,
                    organic,
                    false,
                    0.,
                    false,
                    mode.acceleration,
                )
                .unwrap();
                times.push(start.elapsed().as_secs_f64() * 1000.);
                triangles = out.mesh.indices.len() / 3;
                volume_delta = (out.report.signed_volume_mm3 - reference.report.signed_volume_mm3)
                    .abs()
                    / reference.report.signed_volume_mm3;
                assert!(
                    volume_delta < 0.001,
                    "{} volume diverges beyond 0.1%",
                    mode.name
                );
            }
            times.sort_by(f64::total_cmp);
            let median = times[times.len() / 2];
            println!(
                "  organic={organic} mode={}: {:.0}ms, triangles={}, volume delta {:.6}%",
                mode.name,
                median,
                triangles,
                volume_delta * 100.
            );
            summary.push((mode.name, median, triangles));
        }
        let cpu = summary[0];
        println!(
            "organic={organic}: baseline {} {:.0}ms, fastest {} {:.0}ms ({:.2}x)",
            cpu.0,
            cpu.1,
            summary.iter().min_by(|a, b| a.1.total_cmp(&b.1)).unwrap().0,
            summary
                .iter()
                .map(|entry| entry.1)
                .min_by(f64::total_cmp)
                .unwrap(),
            cpu.1
                / summary
                    .iter()
                    .map(|entry| entry.1)
                    .min_by(f64::total_cmp)
                    .unwrap()
        );
    }
}
