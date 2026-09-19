//! Mesh CSG scaling microbenchmarks via rbench (use a release build).
//!
//! ```sh
//! cargo run --release -p polygon-core --example bench_boolean -- --profile quick
//! ```
//!
//! Workloads mirror the OpenSCAD idioms that first exposed the BSP kernel's
//! scaling limits (docs/design/csg-scaling-2026-09-19.md): a plate with many
//! drilled holes whose cutters are taller than the plate, a union of many
//! separated bodies, and dense separated spheres above the BSP admission cap.
//! Every case validates the result volume so a faster wrong answer fails.
use polygon_core::Mesh;
use polygon_core::solid::boolean::{Operation, Options, boolean, difference_many, union_many};
use polygon_core::solid::primitives::{cube, cylinder, prism_boolean, sphere};
use rbench::{DropPolicy, Suite};

fn translated(mesh: &Mesh, offset: [f64; 3]) -> Mesh {
    mesh.transform([
        [1., 0., 0., offset[0]],
        [0., 1., 0., offset[1]],
        [0., 0., 1., offset[2]],
        [0., 0., 0., 1.],
    ])
    .unwrap()
}

/// `count` cylinders on a grid inside an 80 mm square, taller than the plate.
fn hole_cutters(count: usize, segments: usize) -> Vec<Mesh> {
    let side = (count as f64).sqrt().ceil() as usize;
    let step = 80. / side as f64;
    let radius = 3_f64.min(step / 2. - 0.5);
    let hole = cylinder(12., radius, radius, segments, true).unwrap();
    (0..count)
        .map(|i| {
            let x = -40. + step / 2. + (i % side) as f64 * step;
            let y = -40. + step / 2. + (i / side) as f64 * step;
            translated(&hole, [x, y, 0.])
        })
        .collect()
}

fn volume(mesh: &Mesh) -> f64 {
    mesh.inspect().unwrap().signed_volume_mm3
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 1e-9 + 1e-9,
        "{label}: {actual} vs {expected}"
    );
}

/// Pairwise step used by the bridge: prism arrangement first, BSP otherwise.
fn pairwise(op: Operation) -> impl FnMut(&Mesh, &Mesh) -> polygon_core::Result<Mesh> {
    move |a, b| {
        let name = match op {
            Operation::Union => "union",
            Operation::Difference => "difference",
            Operation::Intersection => "intersection",
        };
        // Same policy as geometry-bridge: the arrangement is a shortcut; a
        // refused profile triangulation hands the step to the BSP Boolean.
        if let Ok(Some(m)) = prism_boolean(a, b, name) {
            return Ok(m);
        }
        Ok(boolean(a, b, op, &Options::default())?.mesh)
    }
}

fn main() -> rbench::Result<()> {
    // Preserve the historical 36-hole workload for comparisons. Expanded
    // 64/100-hole profiles are covered by bench_profile_triangulation and the
    // production OpenSCAD CSG scaling benchmark.
    let plate = cube([86., 86., 8.], true).unwrap();
    let holes = hole_cutters(36, 32);
    let hole_area = 16. * (std::f64::consts::PI / 16.).sin() * 9.;
    let drilled_volume = 86. * 86. * 8. - 36. * hole_area * 8.;
    let ball = sphere(30., 128).unwrap();
    let balls: Vec<Mesh> = (0..3)
        .map(|i| translated(&ball, [70. * i as f64, 0., 0.]))
        .collect();
    let ball_volume = volume(&ball);
    let a = cube([2., 2., 2.], false).unwrap();
    let b = translated(&a, [1., 1., 1.]);

    let mut suite = Suite::new("polygon-core/boolean");
    // One cutter per step is not benchmarked: when the prism arrangement
    // refuses a cap, the BSP fallback re-fragments the whole cap on every step
    // and the stitch budget is exhausted long before 36 steps.
    for (batch, name) in [
        (8_usize, "difference_many/plate-36-holes/batch-8"),
        (36, "difference_many/plate-36-holes/batch-36"),
    ] {
        suite
            .bench_with_input(
                name,
                || (plate.clone(), holes.clone()),
                move |(plate, holes)| {
                    let out =
                        difference_many(plate, holes, &mut pairwise(Operation::Difference), batch)
                            .unwrap();
                    assert_close(volume(&out), drilled_volume, "drilled plate");
                    out.indices.len()
                },
                DropPolicy::InsideTiming,
            )
            .parameter("cutters", 36.);
    }
    suite
        .bench_with_input(
            "union_many/36-separated-cylinders",
            || holes.clone(),
            |holes| {
                let out = union_many(holes, &mut pairwise(Operation::Union)).unwrap();
                assert_close(volume(&out), 36. * hole_area * 12., "joined cylinders");
                out.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("operands", 36.);

    suite
        .bench_with_input(
            "union_many/3-separated-spheres-above-bsp-cap",
            || balls.clone(),
            |balls| {
                let out = union_many(balls, &mut pairwise(Operation::Union)).unwrap();
                assert_close(volume(&out), 3. * ball_volume, "separated spheres");
                out.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", (ball.indices.len() / 3 * 3) as f64);
    suite
        .bench_with_input(
            "boolean/separated-spheres-pairwise-fast-path",
            || (balls[0].clone(), balls[1].clone()),
            |(a, b)| {
                let out = boolean(a, b, Operation::Union, &Options::default()).unwrap();
                assert_close(volume(&out.mesh), 2. * ball_volume, "pairwise separated");
                out.mesh.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", (ball.indices.len() / 3 * 2) as f64);

    suite
        .bench_with_input(
            "boolean/overlapping-boxes-bsp",
            || (a.clone(), b.clone()),
            |(a, b)| {
                let out = boolean(a, b, Operation::Union, &Options::default()).unwrap();
                assert_close(volume(&out.mesh), 15., "overlapping boxes");
                out.mesh.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 24.);
    suite.main()
}
