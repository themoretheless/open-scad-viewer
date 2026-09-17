//! CPU field-sampling microbenchmarks via rbench (use a release build).
//!
//! ```sh
//! cargo run --release -p sdf-core --example bench_sdf_cpu -- --profile quick
//! ```
//!
//! `sdf-core` compiles under the workspace size profile, so these cases track
//! the cost of the leaf math and mesh-distance loops the extraction path
//! spends its time in. Both grids stay inside `check_grid_budget`.
use rbench::{DropPolicy, Suite};
use sdf_core::{Field, Grid, polygonize};

/// Outward closed UV-sphere mesh (shared pole vertices) with a controlled
/// triangle count.
fn uv_sphere(radius: f64, rings: usize, sectors: usize) -> geometry_ops::Triangles {
    let mut positions = vec![0., 0., radius];
    for r in 1..rings {
        let phi = std::f64::consts::PI * r as f64 / rings as f64;
        for s in 0..sectors {
            let theta = 2. * std::f64::consts::PI * s as f64 / sectors as f64;
            positions.push(radius * phi.sin() * theta.cos());
            positions.push(radius * phi.sin() * theta.sin());
            positions.push(radius * phi.cos());
        }
    }
    positions.extend_from_slice(&[0., 0., -radius]);
    let north = 0;
    let south = positions.len() / 3 - 1;
    let ring = |r: usize, s: usize| 1 + (r - 1) * sectors + s % sectors;
    let mut indices = Vec::new();
    for s in 0..sectors {
        indices.extend_from_slice(&[north, ring(1, s), ring(1, s + 1)]);
        indices.extend_from_slice(&[south, ring(rings - 1, s + 1), ring(rings - 1, s)]);
    }
    for r in 1..rings - 1 {
        for s in 0..sectors {
            let (a, b) = (ring(r, s), ring(r, s + 1));
            let (c, d) = (ring(r + 1, s), ring(r + 1, s + 1));
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    geometry_ops::Triangles { positions, indices }
}

fn main() -> rbench::Result<()> {
    let csg = Field::Difference {
        a: Box::new(Field::SmoothUnion {
            a: Box::new(Field::Sphere {
                center: [0., 0., 0.],
                radius: 10.,
            }),
            b: Box::new(Field::Box {
                center: [8., 0., 0.],
                half_size: [6., 6., 6.],
            }),
            radius: 3.,
        }),
        b: Box::new(Field::Torus {
            center: [0., 0., 0.],
            major_radius: 9.,
            minor_radius: 2.5,
        }),
    };
    let csg_grid = Grid {
        min: [-12., -12., -12.],
        max: [16., 12., 12.],
        cells: [48, 48, 48],
    };
    let mesh = uv_sphere(10., 17, 34);
    let triangles = mesh.indices.len() / 3;
    let mesh_field = Field::from_triangles(mesh.clone(), false).unwrap();
    let signed_field = Field::from_triangles(mesh, true).unwrap();
    let mesh_grid = Grid {
        min: [-12., -12., -12.],
        max: [12., 12., 12.],
        cells: [16, 16, 16],
    };

    let mut suite = Suite::new("sdf-core");
    suite
        .bench_with_input(
            "polygonize/csg",
            || (csg.clone(), csg_grid.clone()),
            |(field, grid)| polygonize(field, grid).unwrap().indices.len(),
            DropPolicy::InsideTiming,
        )
        .parameter("cells", 48.);
    suite
        .bench_with_input(
            "polygonize/mesh_unsigned",
            || (mesh_field.clone(), mesh_grid.clone()),
            |(field, grid)| polygonize(field, grid).unwrap().indices.len(),
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", triangles as f64);
    suite
        .bench_with_input(
            "polygonize/mesh_signed",
            || (signed_field.clone(), mesh_grid.clone()),
            |(field, grid)| polygonize(field, grid).unwrap().indices.len(),
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", triangles as f64);
    suite.main()
}
