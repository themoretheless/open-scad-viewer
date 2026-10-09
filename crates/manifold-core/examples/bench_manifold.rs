//! manifold-core kernel microbenchmarks via rbench (use a release build).
//!
//! ```sh
//! cargo run --release -p manifold-core --example bench_manifold -- --profile quick
//! ```
//!
//! Workload: an icosphere (~20k triangles) plus its unwelded and defected
//! variants. Every case validates the result so a faster wrong answer fails.
use manifold_core::{RepairMode, check, metrics, repair, repair_with_mode};
use rbench::{DropPolicy, Suite};

/// Icosphere with `subdivisions` refinement levels (20 * 4^n triangles).
fn icosphere(subdivisions: usize) -> (Vec<f64>, Vec<usize>) {
    let t = (1.0 + 5.0f64.sqrt()) / 2.0;
    #[rustfmt::skip]
    let mut positions: Vec<f64> = vec![
        -1.,  t, 0.,  1.,  t, 0., -1., -t, 0.,  1., -t, 0.,
         0., -1.,  t,  0.,  1.,  t,  0., -1., -t,  0.,  1., -t,
         t,  0., -1.,  t,  0.,  1., -t,  0., -1., -t,  0.,  1.,
    ];
    #[rustfmt::skip]
    let mut indices: Vec<usize> = vec![
        0, 11, 5, 0, 5, 1, 0, 1, 7, 0, 7, 10, 0, 10, 11,
        1, 5, 9, 5, 11, 4, 11, 10, 2, 10, 7, 6, 7, 1, 8,
        3, 9, 4, 3, 4, 2, 3, 2, 6, 3, 6, 8, 3, 8, 9,
        4, 9, 5, 2, 4, 11, 6, 2, 10, 8, 6, 7, 9, 8, 1,
    ];
    let normalize = |p: &mut [f64]| {
        for v in p.as_chunks_mut::<3>().0 {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            v[0] /= len;
            v[1] /= len;
            v[2] /= len;
        }
    };
    normalize(&mut positions);
    for _ in 0..subdivisions {
        use std::collections::HashMap;
        let mut midpoint = HashMap::new();
        let mut next_indices = Vec::with_capacity(indices.len() * 4);
        let mut mid = |a: usize, b: usize, positions: &mut Vec<f64>| -> usize {
            *midpoint.entry((a.min(b), a.max(b))).or_insert_with(|| {
                let id = positions.len() / 3;
                positions.extend_from_slice(&[
                    (positions[a * 3] + positions[b * 3]) / 2.,
                    (positions[a * 3 + 1] + positions[b * 3 + 1]) / 2.,
                    (positions[a * 3 + 2] + positions[b * 3 + 2]) / 2.,
                ]);
                normalize(positions);
                id
            })
        };
        for t in indices.as_chunks::<3>().0 {
            let (a, b, c) = (t[0], t[1], t[2]);
            let ab = mid(a, b, &mut positions);
            let bc = mid(b, c, &mut positions);
            let ca = mid(c, a, &mut positions);
            next_indices.extend_from_slice(&[a, ab, ca, b, bc, ab, c, ca, bc, ab, bc, ca]);
        }
        indices = next_indices;
    }
    (positions, indices)
}

/// Face soup: every triangle carries its own copy of its three corners.
fn unwelded(positions: &[f64], indices: &[usize]) -> (Vec<f64>, Vec<usize>) {
    let mut p = Vec::with_capacity(indices.len() * 3);
    let mut i = Vec::with_capacity(indices.len());
    for &corner in indices {
        i.push(p.len() / 3);
        p.extend_from_slice(&positions[corner * 3..corner * 3 + 3]);
    }
    (p, i)
}

fn main() -> rbench::Result<()> {
    let (p, i) = icosphere(3); // 1280 triangles, 642 vertices
    let (p_big, i_big) = icosphere(4); // 5120 triangles, 2562 vertices
    let (soup_p, soup_i) = unwelded(&p, &i);
    // Defected: unwelded + one duplicated triangle (3+-use edges) + one
    // removed triangle (boundary hole).
    let (dp, mut di) = unwelded(&p, &i);
    let duplicate: Vec<usize> = di[6..9].to_vec();
    di.extend_from_slice(&duplicate);
    di.drain(0..3);

    let mut suite = Suite::new("manifold-core/kernels");
    suite
        .bench_with_input(
            "check/icosphere-1280",
            || (p.clone(), i.clone()),
            |(p, i)| {
                let report = check(p, i);
                assert!(report.is_manifold());
                report.triangle_count
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 1280.);
    suite
        .bench_with_input(
            "check/icosphere-5120",
            || (p_big.clone(), i_big.clone()),
            |(p, i)| {
                let report = check(p, i);
                assert!(report.is_manifold());
                report.triangle_count
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 5120.);
    suite
        .bench_with_input(
            "repair-conservative/unwelded-icosphere-1280",
            || (soup_p.clone(), soup_i.clone()),
            |(p, i)| {
                let out = repair(p, i, 0.0);
                assert!(out.is_manifold());
                assert_eq!(out.positions.len(), 642 * 3);
                out.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 1280.);
    suite
        .bench_with_input(
            "repair-full/defected-icosphere-1280",
            || (dp.clone(), di.clone()),
            |(p, i)| {
                let out = repair_with_mode(p, i, 0.0, RepairMode::Full);
                assert!(out.report.residual.non_manifold_edges.is_empty());
                out.indices.len()
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 1281.);
    suite
        .bench_with_input(
            "metrics/icosphere-5120",
            || (p_big.clone(), i_big.clone()),
            |(p, i)| {
                let m = metrics(p, i);
                assert!(m.watertight);
                assert_eq!(m.components[0].genus, Some(0));
                m.triangle_count
            },
            DropPolicy::InsideTiming,
        )
        .parameter("triangles", 5120.);

    suite.main()
}
