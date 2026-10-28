use brep_core::region_replacement::{self as r, CornerCollar, CornerCollars, GraphOptions};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
fn fixture(curved: bool) -> CornerCollars {
    fixture_at(
        curved,
        [
            [0.2, 0.2],
            [0.7, 0.15],
            [0.85, 0.5],
            [0.65, 0.8],
            [0.25, 0.85],
            [0.1, 0.5],
        ],
    )
}
fn fixture_at(curved: bool, vertices: [[f64; 2]; 6]) -> CornerCollars {
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            j as f64 / 2.,
                            if curved {
                                0.2 * f64::from(i == 2) + 0.1 * i as f64 * j as f64 / 4.
                                    - 0.05 * f64::from(j == 2)
                            } else {
                                0.
                            },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let mut boundaries = Vec::new();
    for i in 0..6 {
        let pc = Curve::from_polyline(vec![vertices[i].to_vec(), vertices[(i + 1) % 6].to_vec()])
            .unwrap();
        let direction = [0.001, 0.001];
        let built =
            nurbs_core::surface_collar::construct(&s, &pc, direction, 1e-7, 2000000).unwrap();
        let mut curve = built.surface.iso(Axis::V, 0.).unwrap();
        // Literal source endpoint ownership, without changing the interior.
        curve.control_points[0] = s
            .evaluate(vertices[i][0], vertices[i][1])
            .unwrap()
            .point
            .to_vec();
        *curve.control_points.last_mut().unwrap() = s
            .evaluate(vertices[(i + 1) % 6][0], vertices[(i + 1) % 6][1])
            .unwrap()
            .point
            .to_vec();
        let g2 = nurbs_core::surface_g2_pcurves::inspect_owned_collar(
            &curve,
            &pc,
            &s,
            direction,
            &built.surface,
            [1e-7, 1e-5, 1e-4],
            10000,
            100000,
            2000000,
        )
        .unwrap()
        .owned;
        boundaries.push(CornerCollar {
            curve,
            original_surface: s.clone(),
            original_pcurve: pc,
            direction,
            surface: built.surface,
            g2,
        });
    }
    CornerCollars {
        boundaries,
        cells: 0,
        coefficient_work: 0,
        exact_work: 0,
    }
}
fn options() -> GraphOptions {
    GraphOptions {
        frame: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        degree: 3,
        spans: 1,
        align_junctions: false,
        junction_refinement: 0.,
        stations_per_side: 17,
        regularization: 1e-12,
    }
}
#[test]
fn interior_guide_preserves_exact_reproduction_and_original_sources() {
    let collars = fixture(true);
    let before = collars
        .boundaries
        .iter()
        .map(|c| {
            (
                c.curve.clone(),
                c.original_surface.clone(),
                c.original_pcurve.clone(),
            )
        })
        .collect::<Vec<_>>();
    // This quadratic is represented exactly in the independent fixture.
    let point = collars.boundaries[0]
        .original_surface
        .evaluate(0.5, 0.5)
        .unwrap()
        .point;
    let graph = r::propose_central_graph_guided(
        &collars,
        options(),
        &[r::GraphGuide {
            point,
            weight: 0.01,
        }],
    )
    .unwrap();
    let poles = &graph.surface.control_points;
    let uv = [
        (point[0] - poles[0][0][0]) / (poles.last().unwrap()[0][0] - poles[0][0][0]),
        (point[1] - poles[0][0][1]) / (poles[0].last().unwrap()[1] - poles[0][0][1]),
    ];
    let e = graph.surface.evaluate(uv[0], uv[1]).unwrap();
    assert!(e.point.iter().zip(point).all(|(a, b)| (a - b).abs() < 1e-8));
    assert!(graph.sampled_jet_residual.iter().all(|x| *x < 1e-7));
    for (c, (curve, s, pc)) in collars.boundaries.iter().zip(before) {
        assert_eq!(c.curve, curve);
        assert_eq!(c.original_surface, s);
        assert_eq!(c.original_pcurve, pc);
    }
}
#[test]
fn outside_or_invalid_interior_guides_are_refused() {
    let collars = fixture(false);
    for guide in [
        r::GraphGuide {
            point: [100., 100., 0.],
            weight: 0.01,
        },
        r::GraphGuide {
            point: [0.5, 0.5, 0.],
            weight: f64::NAN,
        },
    ] {
        assert!(r::propose_central_graph_guided(&collars, options(), &[guide]).is_err());
    }
}
#[test]
fn control_net_fairing_retains_planar_reproduction_and_rejects_invalid_gain() {
    let collars = fixture(false);
    let graph = r::propose_central_graph_fair(&collars, options(), &[], 1e-5).unwrap();
    for u in [0., 0.37, 0.81, 1.] {
        for v in [0., 0.23, 0.71, 1.] {
            assert!(graph.surface.evaluate(u, v).unwrap().point[2].abs() < 1e-12);
        }
    }
    assert!(graph.sampled_jet_residual.iter().all(|x| *x < 1e-12));
    assert!(r::propose_central_graph_fair(&collars, options(), &[], f64::NAN).is_err());
    let curved = fixture(true);
    let graph = r::propose_central_graph_fair(&curved, options(), &[], 1e-5).unwrap();
    assert!(graph.sampled_jet_residual.iter().all(|x| *x < 1e-6));
    for (u, v) in [(0.2, 0.3), (0.5, 0.5), (0.8, 0.7)] {
        let p = graph.surface.evaluate(u, v).unwrap().point;
        assert!((p[2] - (0.2 * p[0] * p[0] + 0.1 * p[0] * p[1] - 0.05 * p[1] * p[1])).abs() < 1e-6);
    }
}
#[test]
fn local_refinement_keeps_original_boundaries_and_c2_basis() {
    let collars = fixture(false);
    let graph =
        r::propose_central_graph_refined(&collars, options(), &[], 0., [&[(0.4, 1)], &[(0.6, 1)]])
            .unwrap();
    assert!(graph.surface.knots_u.contains(&0.4) && graph.surface.knots_v.contains(&0.6));
    assert!(graph.sampled_jet_residual.iter().all(|x| *x < 1e-12));
    assert!(
        r::propose_central_graph_refined(&collars, options(), &[], 0., [&[(0.4, 2)], &[]]).is_err()
    );
}
#[test]
fn six_curved_boundaries_recover_independent_quadratic_graph_jets() {
    let collars = fixture(true);
    let graph = r::propose_central_graph(&collars, options()).unwrap();
    assert!(
        graph.sampled_jet_residual.iter().all(|&x| x < 1e-9),
        "{:?}",
        graph.sampled_jet_residual
    );
    // Interior is independently prescribed, rather than a fit training site.
    for u in [0., 0.21, 0.65, 1.] {
        for v in [0., 0.37, 1.] {
            let p = graph.surface.evaluate(u, v).unwrap().point;
            assert!(
                (p[2] - (0.2 * p[0] * p[0] + 0.1 * p[0] * p[1] - 0.05 * p[1] * p[1])).abs() < 1e-9
            );
        }
    }
}
#[test]
fn planar_six_sided_candidate_is_freshly_admitted_and_tampering_refuses() {
    let collars = fixture(false);
    let mut graph = r::propose_central_graph(&collars, options()).unwrap();
    let inspect = |g: &r::CentralGraph| {
        r::inspect_central_graph(
            &collars,
            g,
            [[1, 0, 0], [0, 1, 0]],
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000,
        )
        .unwrap()
    };
    let proof = inspect(&graph);
    assert!(
        proof.admitted,
        "rim {}, interior {}, seams {:?}",
        proof.rim.proven_simple,
        proof.interior.certified,
        proof
            .seams
            .iter()
            .map(|s| (s.positions.proven, s.geometric.decision))
            .collect::<Vec<_>>()
    );
    let correlated = r::inspect_central_graph_correlated(
        &collars,
        &graph,
        [[1, 0, 0], [0, 1, 0]],
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
        100_000_000,
    )
    .unwrap();
    assert!(
        correlated.admitted,
        "Correlated planar six-sided admission failed"
    );
    assert!(correlated.coefficient_work > 0 && correlated.coefficient_work <= 100_000_000);
    let trimmed = r::trim_central_graph(
        &cad_predicates::ToleranceContext::default_valid(),
        &collars,
        &graph,
        1e-8,
        brep_core::trimmed_face_recipe::Limits {
            pairs: 10000,
            region_cells: 100000,
            domain_cells: 100000,
            agreement_cells: 100000,
        },
    )
    .unwrap();
    assert!(trimmed.face.is_some(), "{}", trimmed.reason);
    assert_eq!(trimmed.face.unwrap().edges().len(), 6);
    for row in &mut graph.surface.control_points {
        for p in row {
            p[2] += 0.01;
        }
    }
    graph.sampled_jet_residual = [0.; 6];
    assert!(!inspect(&graph).admitted);
}
#[test]
fn collapsed_frame_and_work_limits_refuse() {
    let collars = fixture(false);
    let mut o = options();
    o.frame[1] = o.frame[0];
    assert!(r::propose_central_graph(&collars, o).is_err());
    o = options();
    o.degree = 25;
    assert!(r::propose_central_graph(&collars, o).is_err());
    o = options();
    o.degree = 24;
    o.spans = 32;
    o.align_junctions = true;
    assert!(r::propose_central_graph(&collars, o).is_err());
}

#[test]
fn changed_original_curvature_cannot_reuse_a_central_candidate() {
    let mut collars = fixture(false);
    let graph = r::propose_central_graph(&collars, options()).unwrap();
    // This quadratic vanishes to first order on the original first seam.
    // Positions and normals still agree; the curvature field no longer does.
    let (a, b, c) = (-0.05, -0.5, 0.11);
    for i in 0..3 {
        for j in 0..3 {
            let x = i as f64 / 2.;
            let y = j as f64 / 2.;
            collars.boundaries[0].original_surface.control_points[i][j][2] = 0.01
                * (c * c
                    + 2. * c * a * x
                    + 2. * c * b * y
                    + a * a * f64::from(i == 2)
                    + b * b * f64::from(j == 2)
                    + 2. * a * b * x * y);
        }
    }
    let proof = r::inspect_central_graph(
        &collars,
        &graph,
        [[1, 0, 0], [0, 1, 0]],
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert!(proof.seams[0].positions.proven);
    assert!(!proof.admitted);
    assert_ne!(
        proof.seams[0].geometric.decision,
        nurbs_core::surface_join::Decision::WithinTolerance
    );
}

#[test]
fn simple_internal_knots_preserve_the_affine_graph_chart() {
    let collars = fixture(true);
    let mut o = options();
    o.spans = 3;
    let graph = r::propose_central_graph(&collars, o).unwrap();
    assert!(
        graph.sampled_jet_residual.iter().all(|&x| x < 1e-8),
        "{:?}",
        graph.sampled_jet_residual
    );
    for u in [0., 0.21, 0.5, 0.65, 1.] {
        for v in [0., 0.37, 0.5, 1.] {
            let p = graph.surface.evaluate(u, v).unwrap().point;
            assert!((p[0] - (0.0625 + 0.825 * u)).abs() < 1e-12);
            assert!((p[1] - (0.115 + 0.77 * v)).abs() < 1e-12);
        }
    }
}

#[test]
fn junction_knots_keep_the_candidate_c2_and_source_endpoints_owned() {
    let collars = fixture(false);
    let mut o = options();
    o.align_junctions = true;
    o.degree = 5;
    o.spans = 2;
    let graph = r::propose_central_graph(&collars, o).unwrap();
    for (knots, n) in [
        (&graph.surface.knots_u, graph.surface.control_points.len()),
        (
            &graph.surface.knots_v,
            graph.surface.control_points[0].len(),
        ),
    ] {
        for &t in &knots[o.degree + 1..n] {
            assert!(knots.iter().filter(|&&k| k == t).count() <= o.degree - 2);
        }
    }
    let proof = r::inspect_central_graph(
        &collars,
        &graph,
        [[1, 0, 0], [0, 1, 0]],
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert!(proof.admitted);
    let mut shifted = graph;
    shifted.pcurves[0].control_points[0][0] += 0.01;
    assert!(
        !r::inspect_central_graph(
            &collars,
            &shifted,
            [[1, 0, 0], [0, 1, 0]],
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .unwrap()
        .admitted
    );
}

#[test]
fn coarsening_tiny_proposal_knots_never_welds_original_endpoints() {
    let collars = fixture_at(
        false,
        [
            [0.2, 0.2],
            [0.7, 0.15],
            [0.85, 0.5],
            [0.65, 0.8],
            [0.65 + 1e-10, 0.85],
            [0.1, 0.5],
        ],
    );
    let before = collars
        .boundaries
        .iter()
        .map(|c| (c.curve.clone(), c.original_pcurve.clone()))
        .collect::<Vec<_>>();
    let mut o = options();
    o.align_junctions = true;
    let graph = r::propose_central_graph(&collars, o).unwrap();
    for knots in [&graph.surface.knots_u, &graph.surface.knots_v] {
        let mut unique = knots.clone();
        unique.dedup();
        assert!(unique.windows(2).all(|p| p[1] - p[0] >= 1e-5));
    }
    assert_ne!(
        graph.pcurves[3].control_points[0][0],
        graph.pcurves[4].control_points[0][0]
    );
    for (c, (curve, pc)) in collars.boundaries.iter().zip(before) {
        assert_eq!(c.curve, curve);
        assert_eq!(c.original_pcurve, pc);
    }
}

#[test]
fn hard_anchor_is_interpolated_even_when_the_unconstrained_target_is_zero() {
    let collars = fixture(false);
    let graph = r::propose_central_graph_constrained(
        &collars,
        options(),
        &[],
        0.,
        [&[], &[]],
        &[[0.5, 0.5, 0.01]],
    )
    .unwrap();
    let p = &graph.surface.control_points;
    let uv = [
        (0.5 - p[0][0][0]) / (p.last().unwrap()[0][0] - p[0][0][0]),
        (0.5 - p[0][0][1]) / (p[0].last().unwrap()[1] - p[0][0][1]),
    ];
    let q = graph.surface.evaluate(uv[0], uv[1]).unwrap().point;
    for (a, b) in q.iter().zip([0.5, 0.5, 0.01]) {
        assert!((a - b).abs() < 1e-10);
    }
    // The anchor is deliberately incompatible with exact planar reproduction.
    // Successful interpolation must not be confused with a G2 receipt.
    assert!(graph.sampled_jet_residual.iter().any(|v| *v > 1e-4));
    assert!(collars.boundaries.iter().all(|c| {
        c.original_surface
            .control_points
            .iter()
            .flatten()
            .all(|p| p[2] == 0.)
    }));
}
#[test]
fn inconsistent_hard_anchors_and_excessive_anchor_count_refuse() {
    let collars = fixture(false);
    for anchors in [
        vec![[0.5, 0.5, 0.01], [0.5, 0.5, 0.02]],
        vec![[0.5, 0.5, 0.]; 33],
        vec![[f64::NAN, 0.5, 0.]],
        vec![[1000., 1000., 0.]],
    ] {
        assert!(
            r::propose_central_graph_constrained(
                &collars,
                options(),
                &[],
                0.,
                [&[], &[]],
                &anchors
            )
            .is_err()
        );
    }
}
