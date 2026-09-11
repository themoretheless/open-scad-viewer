//! Temporary lattice benchmark (may be removed after qualification).
use geometry_bridge::mesh_shell;
use polygon_core::solid::primitives::cube;
use std::time::Instant;

fn main() {
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
                if x < 2 { edges.push([id(x, y, z), id(x + 1, y, z)]); }
                if y < 2 { edges.push([id(x, y, z), id(x, y + 1, z)]); }
                if z < 2 { edges.push([id(x, y, z), id(x, y, z + 1)]); }
            }
        }
    }
    println!("nodes={} edges={}", nodes.len(), edges.len());
    for organic in [false, true] {
        let reference = mesh_shell::lattice(&mesh, nodes.clone(), edges.clone(), 3.5, 3.5, 1.5, organic, false, 0., false).unwrap();
        let mut cpu_times = Vec::new();
        let mut gpu_times = Vec::new();
        let mut cpu_tris = 0;
        let mut gpu_tris = 0;
        for i in 0..6 {
            let gpu = i % 2 == 1;
            let start = Instant::now();
            let out = mesh_shell::lattice_accelerated(&mesh, nodes.clone(), edges.clone(), 3.5, 3.5, 1.5, organic, false, 0., false,
                if gpu { sdf_core::Acceleration::Gpu } else { sdf_core::Acceleration::Cpu }).unwrap();
            let ms = start.elapsed().as_secs_f64() * 1000.;
            (if gpu { &mut gpu_times } else { &mut cpu_times }).push(ms);
            if gpu { gpu_tris = out.mesh.indices.len() / 3 } else { cpu_tris = out.mesh.indices.len() / 3 }
            let dv = (out.report.signed_volume_mm3 - reference.report.signed_volume_mm3).abs() / reference.report.signed_volume_mm3;
            println!("  organic={organic} gpu={gpu}: volume delta {:.6}%", dv * 100.);
            assert!(dv < 0.001, "volume diverges beyond 0.1%");
        }
        cpu_times.sort_by(f64::total_cmp);
        gpu_times.sort_by(f64::total_cmp);
        println!(
            "organic={organic}: cpu {:.0}ms gpu {:.0}ms | triangles cpu={cpu_tris} gpu={gpu_tris} ({:+.2}%)",
            cpu_times[cpu_times.len() / 2],
            gpu_times[gpu_times.len() / 2],
            (gpu_tris as f64 - cpu_tris as f64) / cpu_tris as f64 * 100.
        );
    }
}
