use brep_core::planar_cap;
use nurbs_core::curve::Curve;

#[test]
fn general_planar_cap_retains_cubic_boundary_and_proves_trim_region() {
    let p = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let wire: Vec<_> = (0..4)
        .map(|i| {
            let a = p[i];
            let b = p[(i + 1) % 4];
            let d = [b[0] - a[0], b[1] - a[1]];
            Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
                control_points: vec![
                    vec![a[0], a[1], 2.],
                    vec![
                        a[0] + d[0] / 3. + 0.1 * d[1],
                        a[1] + d[1] / 3. - 0.1 * d[0],
                        2.,
                    ],
                    vec![
                        a[0] + 2. * d[0] / 3. + 0.1 * d[1],
                        a[1] + 2. * d[1] / 3. - 0.1 * d[0],
                        2.,
                    ],
                    vec![b[0], b[1], 2.],
                ],
                weights: vec![1.; 4],
                periodic: false,
            }
        })
        .collect();
    let report =
        planar_cap::build_general(&[wire.clone()], 2., 1e-8, 1000, 10000, 100000, 10000).unwrap();
    assert_eq!(report.trim.valid, Some(true));
    assert_eq!(report.trim.winding, vec![Some(1)]);
    assert_eq!(report.cap.unwrap().boundaries, vec![wire.clone()]);
    assert!(report.agreement_cells <= 10000);
    let sheet =
        planar_cap::build_sheet_general(&[wire.clone()], 2., 1e-8, 1000, 10000, 100000, 10000)
            .unwrap();
    let model = sheet.model.unwrap();
    model.validate().unwrap();
    assert_eq!(model.faces.len(), 1);
    assert_eq!(model.edges.len(), 4);
    assert!(model.bodies.is_empty());
    assert!(!model.shells[0].closed);
    for (edge, original) in model.edges.iter().zip(&wire) {
        assert_eq!(&edge.curve, original);
    }
    assert!(
        planar_cap::build_sheet_general(&[wire.clone()], 2., 1e-8, 1, 1, 1, 10000)
            .unwrap()
            .model
            .is_none()
    );
    let reversed = wire.iter().rev().map(|c| c.reverse().unwrap()).collect();
    assert!(
        planar_cap::build_general(&[reversed], 2., 1e-8, 1000, 10000, 100000, 10000)
            .unwrap()
            .cap
            .is_none()
    );
    let unresolved = planar_cap::build_general(&[wire], 2., 1e-8, 1, 1, 1, 10000).unwrap();
    assert!(unresolved.cap.is_none());
}
fn rectangle(bounds: [f64; 4], reverse: bool) -> Vec<Curve> {
    let [a, b, c, d] = bounds;
    let mut p = vec![
        vec![a, c, 2.],
        vec![b, c, 2.],
        vec![b, d, 2.],
        vec![a, d, 2.],
    ];
    if reverse {
        p.reverse();
    }
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap())
        .collect()
}
#[test]
fn planar_cap_preserves_exact_boundary_holes_and_continuous_agreement() {
    let outer = rectangle([0., 4., 0., 4.], false);
    let hole = rectangle([1., 2., 1., 2.], true);
    let input = vec![outer, hole];
    let before = input.clone();
    let caps = planar_cap::build(&input, 2., 1e-8, 1000).unwrap();
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].boundaries.len(), 2);
    assert_eq!(caps[0].boundaries, input);
    assert_eq!(input, before);
    let sheet = planar_cap::build_sheet(&input, 2., 1e-8, 1000).unwrap();
    let report = sheet.validate().unwrap();
    assert!(report.topology_valid);
    assert_eq!(report.boundary_edge_count, 8);
    assert_eq!(sheet.faces.len(), 1);
    assert_eq!(sheet.faces[0].holes.len(), 1);
    assert!(!sheet.shells[0].closed);
    assert!(sheet.bodies.is_empty());
    for row in &caps[0].agreements {
        for p in row {
            assert_eq!(
                p.status,
                nurbs_core::curve_surface_agreement::Status::WithinTolerance
            );
        }
    }
    let disjoint = vec![
        rectangle([0., 1., 0., 1.], false),
        rectangle([3., 4., 0., 1.], false),
    ];
    assert_eq!(
        planar_cap::build(&disjoint, 2., 1e-8, 1000).unwrap().len(),
        2
    );
}
#[test]
fn planar_cap_refuses_gap_nonplanarity_wrong_orientation_and_missing_budget() {
    let mut gap = rectangle([0., 1., 0., 1.], false);
    gap[0].control_points[1][0] += 1e-10;
    assert!(planar_cap::build(&[gap], 2., 1e-8, 1000).is_err());
    let mut nonplanar = rectangle([0., 1., 0., 1.], false);
    nonplanar[0].control_points[1][2] += 1e-10;
    assert!(planar_cap::build(&[nonplanar], 2., 1e-8, 1000).is_err());
    assert!(planar_cap::build(&[rectangle([0., 1., 0., 1.], true)], 2., 1e-8, 1000).is_err());
    assert!(planar_cap::build(&[rectangle([0., 1., 0., 1.], false)], 2., 1e-8, 0).is_err());
}

#[test]
fn thread_section_builds_a_planar_cap_with_retained_boundaries() {
    use nurbs_core::{curve_chain, thread};
    let plane = 1.13;
    let trims = thread::trim_candidates(
        thread::Spec {
            turns: 2,
            ..thread::Spec::default()
        },
        [plane, 1.87],
        1e-10,
    )
    .unwrap();
    let mut curves = Vec::new();
    for candidate in &trims.candidates {
        if candidate
            .exact_uv_vertices
            .iter()
            .filter(|v| {
                matches!(v,
                    thread::ExactUvVertex::EdgePlane { z_limit, .. } if *z_limit == plane
                )
            })
            .count()
            < 2
        {
            continue;
        }
        let edges = candidate
            .planar_edges(plane, 1e-12, 1e-3, 128, 100000)
            .unwrap();
        assert!(edges.all_edges_within_tolerance);
        curves.extend(edges.edges.into_iter().map(|edge| edge.mapped.curve));
    }
    let closed = curve_chain::close(&curves, 1e-8, 1e-8, 65536).unwrap();
    assert!(closed.within_tolerance);
    let shared = closed.curves.unwrap();
    assert_eq!(closed.source.cyclic_chains.len(), 1);
    let mut wire: Vec<_> = closed.source.cyclic_chains[0]
        .iter()
        .map(|&i| shared[i].clone())
        .collect();
    let area: f64 = wire
        .iter()
        .flat_map(|c| c.control_points.windows(2))
        .map(|p| p[0][0] * p[1][1] - p[1][0] * p[0][1])
        .sum();
    if area < 0. {
        wire = wire.iter().rev().map(|c| c.reverse().unwrap()).collect();
    }
    let sheet = planar_cap::build_sheet(&[wire.clone()], plane, 1e-8, 100000).unwrap();
    assert!(sheet.validate().unwrap().topology_valid);
    assert_eq!(sheet.faces.len(), 1);
    assert_eq!(sheet.edges.len(), wire.len());
    assert!(!sheet.shells[0].closed);
    assert!(sheet.bodies.is_empty());
    let caps = planar_cap::build(&[wire.clone()], plane, 1e-8, 100000).unwrap();
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].boundaries, vec![wire]);
    assert!(
        caps[0]
            .surface
            .control_points
            .iter()
            .flatten()
            .all(|p| p[2] == plane)
    );
    assert!(
        caps[0]
            .agreements
            .iter()
            .flatten()
            .all(|r| r.status == nurbs_core::curve_surface_agreement::Status::WithinTolerance)
    );
}

fn bowed_rectangle(bounds: [f64; 4], reverse: bool) -> Vec<Curve> {
    rectangle(bounds, reverse)
        .into_iter()
        .map(|line| {
            let a = &line.control_points[0];
            let b = &line.control_points[1];
            let d = [b[0] - a[0], b[1] - a[1]];
            let mut c = line.elevate(3).unwrap();
            for p in &mut c.control_points[1..3] {
                p[0] += 0.05 * d[1];
                p[1] -= 0.05 * d[0];
            }
            c.weights[1] = 0.8;
            c.weights[2] = 1.2;
            c
        })
        .collect()
}
#[test]
fn general_curved_cap_qualifies_holes_and_refuses_outside_hole() {
    let outer = bowed_rectangle([0., 4., 0., 4.], false);
    let hole = bowed_rectangle([1., 2., 1., 2.], true);
    let loops = vec![outer.clone(), hole];
    let before = loops.clone();
    let report =
        planar_cap::build_sheet_general(&loops, 2., 1e-8, 10000, 100000, 1000000, 10000).unwrap();
    assert_eq!(report.trim.valid, Some(true));
    assert_eq!(report.trim.winding, vec![Some(1), Some(-1)]);
    let model = report.model.unwrap();
    assert_eq!(model.faces.len(), 1);
    assert_eq!(model.faces[0].holes.len(), 1);
    assert_eq!(model.validate().unwrap().boundary_edge_count, 8);
    for (edge, curve) in model.edges.iter().zip(loops.iter().flatten()) {
        assert_eq!(&edge.curve, curve);
    }
    assert_eq!(loops, before);
    let backwards: Vec<Vec<Curve>> = loops
        .iter()
        .map(|wire| wire.iter().rev().map(|c| c.reverse().unwrap()).collect())
        .collect();
    let oriented = planar_cap::build_sheet_general_oriented(
        &backwards, 2., 1e-8, 10000, 100000, 1000000, 10000,
    )
    .unwrap();
    assert!(oriented.orientation_reversed);
    assert_eq!(oriented.trim.winding, vec![Some(-1), Some(1)]);
    let normalized = oriented.model.unwrap();
    assert_eq!(normalized.faces[0].holes.len(), 1);
    assert_eq!(normalized.validate().unwrap().boundary_edge_count, 8);
    for (edge, curve) in normalized.edges.iter().zip(loops.iter().flatten()) {
        assert_eq!(&edge.curve, curve);
    }

    let outside = bowed_rectangle([5., 6., 1., 2.], true);
    let rejected =
        planar_cap::build_sheet_general(&[outer, outside], 2., 1e-8, 10000, 100000, 1000000, 10000)
            .unwrap();
    assert_eq!(rejected.trim.valid, Some(false));
    assert!(rejected.model.is_none());
}
