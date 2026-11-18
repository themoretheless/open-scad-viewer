//! Scaling ladder for the native mesh CSG kernel (evidence for ADR 0013).
//!
//! ```sh
//! cargo run --release -p polygon-core --example csg_ladder
//! ```
//!
//! Prints one JSON line per workload: wall time (median of 3 after one
//! warmup), triangle counts, and the refusal message when the kernel refuses.
//! `medium-csg` is the panel minus 64 cylinders of `bench:cpu`, evaluated as
//! the bridge does (prism arrangement first, BSP fallback, batches of 8) and
//! with the BSP path only. `dense-sphere` steps sphere booleans up in `$fn`.
use polygon_core::Mesh;
use polygon_core::solid::boolean::{Operation, Options, boolean, difference_many, union_many};
use polygon_core::solid::primitives::{cube, cylinder, prism_boolean, sphere};
use std::time::Instant;

fn moved(mesh: &Mesh, d: [f64; 3]) -> Mesh {
    mesh.transform([
        [1., 0., 0., d[0]],
        [0., 1., 0., d[1]],
        [0., 0., 1., d[2]],
        [0., 0., 0., 1.],
    ])
    .unwrap()
}

fn op_name(op: Operation) -> &'static str {
    match op {
        Operation::Union => "union",
        Operation::Difference => "difference",
        Operation::Intersection => "intersection",
    }
}

fn pairwise(op: Operation, prism: bool) -> impl FnMut(&Mesh, &Mesh) -> polygon_core::Result<Mesh> {
    move |a, b| {
        if prism && let Ok(Some(m)) = prism_boolean(a, b, op_name(op)) {
            return Ok(m);
        }
        Ok(boolean(a, b, op, &Options::default())?.mesh)
    }
}

fn panel_holes(count: usize, segments: usize) -> (Mesh, Vec<Mesh>) {
    let panel = cube([86., 86., 8.], true).unwrap();
    let hole = cylinder(12., 3., 3., segments, true).unwrap();
    let side = (count as f64).sqrt().ceil() as usize;
    let holes = (0..count)
        .map(|i| {
            moved(
                &hole,
                [
                    -35. + (i % side) as f64 * 10.,
                    -35. + (i / side) as f64 * 10.,
                    0.,
                ],
            )
        })
        .collect();
    (panel, holes)
}

fn run(name: &str, input_triangles: usize, mut work: impl FnMut() -> polygon_core::Result<Mesh>) {
    let mut times = vec![];
    let mut outcome = Ok(0);
    for iteration in 0..4 {
        let start = Instant::now();
        let result = work();
        let ms = start.elapsed().as_secs_f64() * 1e3;
        if iteration > 0 {
            times.push(ms);
        }
        outcome = result.map(|m| m.indices.len() / 3);
        if outcome.is_err() {
            times = vec![ms];
            break;
        }
    }
    times.sort_by(f64::total_cmp);
    let median = times[times.len() / 2];
    match outcome {
        Ok(triangles) => println!(
            "{{\"case\":\"{name}\",\"status\":\"ok\",\"inputTriangles\":{input_triangles},\"outputTriangles\":{triangles},\"medianMs\":{median:.3}}}"
        ),
        Err(error) => println!(
            "{{\"case\":\"{name}\",\"status\":\"refused\",\"inputTriangles\":{input_triangles},\"error\":{:?},\"ms\":{median:.3}}}",
            error.message
        ),
    }
}

fn triangles(meshes: &[&Mesh]) -> usize {
    meshes.iter().map(|m| m.indices.len() / 3).sum()
}

fn main() {
    for count in [16, 36, 64] {
        let (panel, holes) = panel_holes(count, 32);
        let input = triangles(&[&panel]) + holes.iter().map(|h| h.indices.len() / 3).sum::<usize>();
        for (prism, label) in [(true, "bridge"), (false, "bsp-only")] {
            run(
                &format!("medium-csg/panel-minus-{count}-cylinders-fn32/{label}"),
                input,
                || {
                    difference_many(
                        &panel,
                        &holes,
                        &mut pairwise(Operation::Difference, prism),
                        8,
                    )
                },
            );
        }
    }
    for fn_ in [16, 24, 32, 48, 64, 96, 128] {
        let a = sphere(30., fn_).unwrap();
        let b = moved(&a, [20., 0., 0.]);
        let input = triangles(&[&a, &b]);
        run(
            &format!("dense-sphere/overlapping-union-fn{fn_}"),
            input,
            || Ok(boolean(&a, &b, Operation::Union, &Options::default())?.mesh),
        );
        run(
            &format!("dense-sphere/overlapping-difference-fn{fn_}"),
            input,
            || Ok(boolean(&a, &b, Operation::Difference, &Options::default())?.mesh),
        );
    }
    for fn_ in [64, 128, 192, 256] {
        let a = sphere(30., fn_).unwrap();
        let balls: Vec<_> = (0..3)
            .map(|i| moved(&a, [70. * i as f64, 0., 0.]))
            .collect();
        let input = 3 * triangles(&[&a]);
        run(
            &format!("dense-sphere/separated-union-3x-fn{fn_}"),
            input,
            || union_many(&balls, &mut pairwise(Operation::Union, true)),
        );
    }
}
