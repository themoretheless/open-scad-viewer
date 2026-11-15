//! `medium-csg` and `dense-sphere` CSG fixtures via rbench (release build).
//!
//! ```sh
//! cargo run --release -p polygon-core --example bench_csg_fixtures -- --profile quick
//! ```
//!
//! Native counterparts of the `bench:cpu` fixture ids of the same names
//! (`tools/manifold-bench/fixtures.mjs`), evaluated with the bridge policy:
//! prism arrangement first, BSP Boolean otherwise, difference batches of 8.
//! `dense-sphere` is re-scoped to what the kernel accepts today (see ADR
//! 0013): overlapping sphere booleans at `$fn=32` (BSP construction refuses
//! `$fn>=48`) and three separated `$fn=128` spheres (`$fn=192+` exceeds
//! `MAX_MESH_TRIANGLES`). `csg_ladder` prints the full scaling ladder.
//! Every case checks its result volume so a faster wrong answer fails.
//! `tools/manifold-bench/rbench-regression.mjs` gates this suite at 15%.
use polygon_core::Mesh;
use polygon_core::solid::boolean::{Operation, Options, boolean, difference_many, union_many};
use polygon_core::solid::primitives::{cube, cylinder, prism_boolean, sphere};
use rbench::{DropPolicy, Suite};

fn moved(mesh: &Mesh, d: [f64; 3]) -> Mesh {
    mesh.transform([
        [1., 0., 0., d[0]],
        [0., 1., 0., d[1]],
        [0., 0., 1., d[2]],
        [0., 0., 0., 1.],
    ])
    .unwrap()
}

fn volume(mesh: &Mesh) -> f64 {
    mesh.inspect().unwrap().signed_volume_mm3
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 1e-9,
        "{label}: {actual} vs {expected}"
    );
}

fn pairwise(op: Operation) -> impl FnMut(&Mesh, &Mesh) -> polygon_core::Result<Mesh> {
    let name = match op {
        Operation::Union => "union",
        Operation::Difference => "difference",
        Operation::Intersection => "intersection",
    };
    move |a, b| {
        if let Ok(Some(m)) = prism_boolean(a, b, name) {
            return Ok(m);
        }
        Ok(boolean(a, b, op, &Options::default())?.mesh)
    }
}

fn main() -> rbench::Result<()> {
    // medium-csg: 86 x 86 x 8 panel minus 64 cylinders r=3, $fn=32, taller
    // than the panel (the OpenSCAD idiom against z-fighting).
    let panel = cube([86., 86., 8.], true).unwrap();
    let hole = cylinder(12., 3., 3., 32, true).unwrap();
    let holes: Vec<Mesh> = (0..64)
        .map(|i| {
            moved(
                &hole,
                [-35. + (i % 8) as f64 * 10., -35. + (i / 8) as f64 * 10., 0.],
            )
        })
        .collect();
    let hole_area = 144. * (std::f64::consts::PI / 16.).sin();
    let drilled = 86. * 86. * 8. - 64. * hole_area * 8.;

    let ball32 = sphere(30., 32).unwrap();
    let shifted32 = moved(&ball32, [20., 0., 0.]);
    let reference = |op| {
        volume(
            &boolean(&ball32, &shifted32, op, &Options::default())
                .unwrap()
                .mesh,
        )
    };
    let (union_volume, difference_volume) = (
        reference(Operation::Union),
        reference(Operation::Difference),
    );
    let ball128 = sphere(30., 128).unwrap();
    let balls: Vec<Mesh> = (0..3)
        .map(|i| moved(&ball128, [70. * i as f64, 0., 0.]))
        .collect();
    let ball128_volume = volume(&ball128);

    let mut suite = Suite::new("polygon-core/csg-fixtures");
    suite
        .bench_with_input(
            "medium-csg/panel-minus-64-cylinders-fn32",
            || (panel.clone(), holes.clone()),
            |(panel, holes)| {
                let out =
                    difference_many(panel, holes, &mut pairwise(Operation::Difference), 8).unwrap();
                assert_close(volume(&out), drilled, "medium-csg");
                out.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("booleans", 64.);
    for (name, op, expected) in [
        (
            "dense-sphere/overlapping-union-fn32",
            Operation::Union,
            union_volume,
        ),
        (
            "dense-sphere/overlapping-difference-fn32",
            Operation::Difference,
            difference_volume,
        ),
    ] {
        suite
            .bench_with_input(
                name,
                || (ball32.clone(), shifted32.clone()),
                move |(a, b)| {
                    let out = boolean(a, b, op, &Options::default()).unwrap().mesh;
                    assert_close(volume(&out), expected, name);
                    out.indices.len()
                },
                DropPolicy::InsideTiming,
            )
            .parameter("triangles", (2 * ball32.indices.len() / 3) as f64);
    }
    suite
        .bench_with_input(
            "dense-sphere/separated-union-3x-fn128",
            || balls.clone(),
            |balls| {
                let out = union_many(balls, &mut pairwise(Operation::Union)).unwrap();
                assert_close(volume(&out), 3. * ball128_volume, "separated spheres");
                out.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", (3 * ball128.indices.len() / 3) as f64);
    suite.main()
}
