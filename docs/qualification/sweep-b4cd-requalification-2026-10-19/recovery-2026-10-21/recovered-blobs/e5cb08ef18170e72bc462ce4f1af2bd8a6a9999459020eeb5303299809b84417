#[path = "../examples/support/fiber_cut_fixture.rs"]
mod fixture;
use brep_core::face_cut;
use nurbs_core::surface::Surface;

#[test]
fn four_original_roots_form_two_verified_curved_branches_and_three_sectors() {
    let (source, cutter) = pair(false);
    let before = (source.clone(), cutter.clone());
    let out =
        face_cut::partition_fiber_branches_at_face(&source, 0, &cutter, 0, 1e-8, 100000, 1000000)
            .unwrap();
    assert_eq!(out.branches.len(), 2);
    assert_eq!(out.faces.len(), 3);
    assert!(out.rejected_pairs > 0); // Boundary ordering alone would pair incorrectly.
    assert_eq!(out.refusals.len(), out.rejected_pairs);
    assert!(
        out.refusals
            .iter()
            .all(|r| !r.code.is_empty() && !r.message.is_empty())
    );
    assert!(
        out.charged_cells <= 100000
            && out.charged_exact_work <= 1000000
            && out.charged_evaluations <= 4096
    );
    assert!(out.boundary_coverage.iter().all(|(_, r)| r.complete));
    let endpoints = out
        .branches
        .iter()
        .flat_map(|b| b.endpoints)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(endpoints.len(), 4);
    for branch in &out.branches {
        for &edge in &branch.edges {
            assert_eq!(
                out.model
                    .loops
                    .iter()
                    .flat_map(|l| &l.coedges)
                    .filter(|c| c.edge == edge)
                    .count(),
                2
            );
            // Independent analytical residual on actual saved 3D curves.
            for i in 0..=64 {
                let c = &out.model.edges[edge].curve;
                let [a, b] = c.domain();
                let p = c.evaluate(a + (b - a) * i as f64 / 64.).unwrap().point;
                assert!(p[2].abs() < source.tolerance_mm);
                assert!(
                    ((p[1] - 0.25) * (p[1] - 0.75) + 0.01 * p[0] * p[0]).abs()
                        < source.tolerance_mm
                );
            }
        }
    }
    let restored: brep_core::Model =
        value_codec::from_str(&value_codec::to_string(&out.model).unwrap()).unwrap();
    assert_eq!(restored, out.model);
    assert!(
        face_cut::partition_fiber_branches_at_face(&source, 0, &cutter, 0, 1e-8, 32, 1000000)
            .is_err()
    );
    assert_eq!((source, cutter), before);
}

#[test]
fn six_roots_of_a_cubic_cutter_form_three_distinct_owned_branches() {
    let (source, cutter) = pair(true);
    let before = (source.clone(), cutter.clone());
    let out =
        face_cut::partition_fiber_branches_at_face(&source, 0, &cutter, 0, 1e-8, 100000, 1000000)
            .unwrap();
    assert_eq!(out.branches.len(), 3);
    assert_eq!(out.faces.len(), 4);
    assert!(out.branches.iter().any(|b| b.root_fibers > 1));
    assert!(out.boundary_coverage.iter().all(|(_, r)| r.complete));
    assert!(
        out.charged_cells <= 100000
            && out.charged_exact_work <= 1000000
            && out.charged_evaluations <= 4096
    );
    let ends = out
        .branches
        .iter()
        .flat_map(|b| b.endpoints)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ends.len(), 6);
    for branch in &out.branches {
        for &edge in &branch.edges {
            let curve = &out.model.edges[edge].curve;
            let [a, b] = curve.domain();
            for i in 0..=64 {
                let p = curve.evaluate(a + (b - a) * i as f64 / 64.).unwrap().point;
                let residual = (p[1] - 0.2) * (p[1] - 0.5) * (p[1] - 0.8) + 0.005 * p[0] * p[0];
                assert!(p[2].abs() < source.tolerance_mm && residual.abs() < source.tolerance_mm);
            }
        }
    }
    assert_eq!((source, cutter), before);
}

fn pair(cubic: bool) -> (brep_core::Model, brep_core::Model) {
    let source = fixture::sheet(Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    });
    // Original curved cutter: z=(y-0.25)(y-0.75)+0.01*x²,
    // x=-1+3s, y=-1+3t. Both transverse branches cross the source square.
    let square = [1., -2., 4.];
    let y_polynomial = if cubic {
        vec![-3.24, 3.42, -3.42, 3.24]
    } else {
        vec![2.1875, -2.3125, 2.1875]
    };
    let degree_v = y_polynomial.len() - 1;
    let x_gain = if cubic { 0.005 } else { 0.01 };
    let cutter = fixture::sheet(Surface {
        degree_u: 2,
        degree_v,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: [vec![0.; degree_v + 1], vec![1.; degree_v + 1]].concat(),
        control_points: (0..3)
            .map(|i| {
                (0..=degree_v)
                    .map(|j| {
                        vec![
                            -1. + 1.5 * i as f64,
                            -1. + 3. * j as f64 / degree_v as f64,
                            y_polynomial[j] + x_gain * square[i],
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; degree_v + 1]; 3],
        periodic_u: false,
        periodic_v: false,
    });
    (source, cutter)
}
