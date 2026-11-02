use brep_core::{Model, boundary_agreement as b};
use brep_topology::CoedgeTrim;
fn independent_uv() -> Model {
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let domains = m.edges.iter().map(|e| e.curve.domain()).collect::<Vec<_>>();
    for wire in &mut m.loops {
        for c in &mut wire.coedges {
            let a = c.pcurve.control_points[0].clone();
            let z = c.pcurve.control_points[1].clone();
            let mut p = c.pcurve.clone();
            p.control_points = vec![
                a.iter().zip(&z).map(|(a, z)| 2. * a - z).collect(),
                a.iter().zip(&z).map(|(a, z)| 3. * z - 2. * a).collect(),
            ];
            p.knots = vec![2., 2., 6., 6.];
            *c = brep_core::Coedge::new(c.edge, c.reversed, p)
                .with_trim(CoedgeTrim {
                    curve_parameter: domains[c.edge],
                    pcurve_parameter: [3., 4.],
                    periodic_lift: [[0; 2]; 2],
                })
                .unwrap();
        }
    }
    m.validate().unwrap();
    m
}
#[test]
fn exact_brep_uses_retain_original_independent_uv_clocks_and_shared_work() {
    let m = independent_uv();
    let before = m.clone();
    let r = b::verify_exact(&m, 1000000).unwrap();
    assert!(r.all_equal);
    assert!(!r.has_exact_counterexample());
    assert_eq!(r.uses.len(), 24);
    assert!(r.all_joins_exact); // Closure refers to the declared trimmed intervals.
    assert!(!b::verify_exact(&m, r.work - 1).unwrap().all_equal);
    let qualified = b::verify_with_exact(&m, 1, 1000000).unwrap();
    assert!(qualified.agreement.complete);
    assert_eq!(m, before);
    let mut wrong = m;
    let mut c = wrong.edges[0].curve.elevate(3).unwrap();
    c.control_points[1][2] += 2f64.powi(-40);
    wrong.edges[0].curve = c;
    let r = b::verify_exact(&wrong, 1000000).unwrap();
    assert!(!r.all_equal);
    assert!(r.has_exact_counterexample());
}
#[test]
fn explicit_partial_diagnostic_windows_prove_only_the_declared_use_intervals() {
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    for wire in &mut m.loops {
        for c in &mut wire.coedges {
            *c = c
                .clone()
                .with_trim(CoedgeTrim {
                    curve_parameter: [0.125, 0.75],
                    pcurve_parameter: [0.125, 0.75],
                    periodic_lift: [[0; 2]; 2],
                })
                .unwrap();
        }
    }
    let before = m.clone();
    let report = b::verify_exact(&m, 1000000).unwrap();
    assert!(report.all_equal);
    assert!(!report.all_joins_exact); // Diagnostic windows do not cover a closed contour.
    assert_eq!(m, before);
}

#[test]
fn trimmed_uv_gaps_below_sampling_tolerance_are_exactly_refuted() {
    let mut m = independent_uv();
    let wire = m.faces[0].outer;
    m.loops[wire].coedges[0].pcurve.control_points[0][0] += 2f64.powi(-40);
    let report = b::verify_exact(&m, 1000000).unwrap();
    assert!(!report.all_joins_exact);
    assert!(
        report
            .joins
            .iter()
            .any(|(_, _, closed)| *closed == Some(false))
    );
}

#[test]
fn original_multispan_uv_endpoints_close_without_claiming_an_unproved_surface_lift() {
    let mut m = independent_uv();
    for wire in &mut m.loops {
        for c in &mut wire.coedges {
            let a = c.pcurve.control_points[0].clone();
            let z = c.pcurve.control_points[1].clone();
            let middle = a.iter().zip(&z).map(|(a, z)| (a + z) * 0.5).collect();
            c.pcurve.control_points = vec![a, middle, z];
            c.pcurve.weights = vec![1.; 3];
            c.pcurve.knots = vec![2., 2., 4., 6., 6.];
        }
    }
    let before = m.clone();
    m.validate().unwrap();
    let r = b::verify_exact(&m, 1000000).unwrap();
    assert!(r.all_joins_exact);
    assert!(!r.all_equal); // Multi-span surface-lift dispatch is separate from endpoint closure.
    assert!(!r.has_exact_counterexample());
    assert_eq!(m, before);
    let wire = m.faces[0].outer;
    m.loops[wire].coedges[0].pcurve.control_points[1][0] += 2f64.powi(-40);
    assert!(!b::verify_exact(&m, 1000000).unwrap().all_joins_exact);
}

#[test]
fn independently_trimmed_shared_edges_match_rational_multispan_face_boundaries() {
    fn weights(a: &[f64], b: &[f64]) -> [f64; 2] {
        if a.partial_cmp(b) == Some(std::cmp::Ordering::Less) {
            [1., 3.]
        } else {
            [3., 1.]
        }
    }
    let mut m = independent_uv();
    for edge in &mut m.edges {
        edge.curve.weights =
            weights(&edge.curve.control_points[0], &edge.curve.control_points[1]).to_vec();
    }
    for face in &mut m.faces {
        let s = &mut face.surface;
        assert_eq!(s.control_points.len(), 2);
        assert_eq!(s.control_points[0].len(), 2);
        let u = weights(&s.control_points[0][0], &s.control_points[1][0]);
        let v = weights(&s.control_points[0][0], &s.control_points[0][1]);
        let old = s.control_points.clone();
        let middle = (0..2)
            .map(|j| {
                (0..3)
                    .map(|k| (old[0][j][k] * u[0] + old[1][j][k] * u[1]) / (u[0] + u[1]))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        s.control_points = vec![old[0].clone(), middle, old[1].clone()];
        s.weights = vec![
            vec![u[0] * v[0], u[0] * v[1]],
            vec![2. * v[0], 2. * v[1]],
            vec![u[1] * v[0], u[1] * v[1]],
        ];
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
    }
    m.validate().unwrap();
    let before = m.clone();
    let r = b::verify_exact(&m, 1000000).unwrap();
    assert!(r.all_equal && r.all_joins_exact);
    assert_eq!(r.uses.len(), 24);
    assert!(r.uses.iter().all(|u| {
        u.decision
            .as_ref()
            .is_some_and(|d| d.outcome == cad_predicates::BezierIdentity::Equal)
    }));
    assert_eq!(m, before);
    assert!(
        b::verify_with_exact(&m, 1, 1000000)
            .unwrap()
            .agreement
            .complete
    );
    let outer = m.faces[0].outer;
    let edge = m.loops[outer].coedges[0].edge;
    assert_eq!(
        m.loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|c| c.edge == edge)
            .count(),
        2
    );
    m.faces[0].surface.weights[1][0] += 0.125;
    let bad = b::verify_exact(&m, 1000000).unwrap();
    assert!(!bad.all_equal);
    assert!(bad.has_exact_counterexample());
}
