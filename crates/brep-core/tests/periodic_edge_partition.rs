use brep_core::{Model, feature_ops::edge_partition::{self, Limits}};
use brep_topology::CoedgeTrim;
fn cylinder() -> (Model, usize) {
    let mut s = brep_core::cylinder(2., 3.).unwrap();
    let mut carrier = s.faces[0].surface.clone();
    carrier.control_points = (0..4)
        .flat_map(|f| s.faces[f].surface.control_points[..2].iter().cloned())
        .collect();
    carrier.weights = (0..4)
        .flat_map(|f| s.faces[f].surface.weights[..2].iter().cloned())
        .collect();
    carrier.control_points.extend_from_within(..2);
    carrier.weights.extend_from_within(..2);
    carrier.knots_u = vec![-1., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 5., 5.];
    carrier.periodic_u = true;
    carrier.validate().unwrap();
    for f in 0..4 {
        let shift = match f {
            0 => 8.,
            3 => -8.,
            _ => 0.,
        };
        let wire = s.faces[f].outer;
        for c in &mut s.loops[wire].coedges {
            for p in &mut c.pcurve.control_points {
                p[0] += f as f64 + shift;
            }
        }
        s.faces[f].surface = carrier.clone();
    }
    let e = s
        .edges
        .iter()
        .position(|e| {
            let a = s.vertices[e.vertices[0]].point;
            let b = s.vertices[e.vertices[1]].point;
            a[0] == 2. && a[1] == 0. && b[0] == 2. && b[1] == 0. && a[2] != b[2]
        })
        .unwrap();
    for f in 0..4 {
        let wire = s.faces[f].outer;
        for c in &mut s.loops[wire].coedges {
            if c.edge == e {
                let mut curve = [0.17, 0.83];
                let mut pc = [0.17, 0.83];
                if c.reversed {
                    curve.reverse();
                    pc.reverse();
                }
                let mut lifts = [[0; 2]; 2];
                for i in 0..2 {
                    lifts[i][0] = (c.pcurve.evaluate(pc[i]).unwrap().point[0] / 4.).floor() as i32;
                }
                *c = c
                    .clone()
                    .with_trim(CoedgeTrim {
                        curve_parameter: curve,
                        pcurve_parameter: pc,
                        periodic_lift: lifts,
                    })
                    .unwrap();
            }
        }
    }
    s.validate().unwrap();
    (s, e)
}
fn volume(s: &Model) -> f64 {
    brep_core::analysis::mass_properties(s, 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3
}
#[test]
fn shared_periodic_shores_keep_independent_positive_and_negative_lifts() {
    let (s, e) = cylinder();
    let before = s.clone();
    let p =
        edge_partition::split(&s, e, &[0.25, 0.7], "periodic-shores", Limits::default()).unwrap();
    assert_eq!(p.relations().len(), 2);
    assert!(
        p.relations()
            .iter()
            .any(|r| r.original.periodic_lift[0][0] > 0)
    );
    assert!(
        p.relations()
            .iter()
            .any(|r| r.original.periodic_lift[0][0] < 0)
    );
    for r in p.relations() {
        for fragment in &r.fragments {
            let c = p.model().loops[p.model().faces[r.face].outer]
                .coedges
                .iter()
                .find(|c| c.edge == fragment.edge)
                .unwrap();
            let t = c.trim().unwrap();
            for i in 0..2 {
                assert_eq!(
                    t.periodic_lift[i][0],
                    (c.pcurve.evaluate(t.pcurve_parameter[i]).unwrap().point[0] / 4.).floor()
                        as i32
                );
            }
            assert!(fragment.original_curve_distance_upper <= s.tolerance_mm);
        }
    }
    assert!(p.reverify().unwrap().charts_injective);
    assert!((volume(p.model()) - 12. * std::f64::consts::PI).abs() < 1e-5);
    assert_eq!(s, before);
}
#[test]
fn periodic_authored_lift_mismatch_and_work_exhaustion_refuse_atomically() {
    let (s, e) = cylinder();
    let before = s.clone();
    assert!(
        edge_partition::split(
            &s,
            e,
            &[0.5],
            "work",
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut bad = s.clone();
    for w in &mut bad.loops {
        for c in &mut w.coedges {
            if c.edge == e {
                let mut t = c.trim().unwrap();
                t.periodic_lift[0][0] += 1;
                *c = c.clone().with_trim(t).unwrap();
            }
        }
    }
    assert!(edge_partition::split(&bad, e, &[0.5], "bad-lift", Limits::default()).is_err());
    assert_eq!(s, before);
}

fn seam_crossing_box() -> (Model, usize, usize) {
    let mut s = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let f = 5;
    let wire = s.faces[f].outer;
    let edge = s.loops[wire].coedges[0].edge;
    let profile = [[0., 0.5], [0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.5]];
    let carrier = nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., 0., 1., 2., 3., 4., 5., 6.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: profile
            .iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
            .collect(),
        weights: vec![vec![1., 1.]; 6],
        periodic_u: true,
        periodic_v: false,
    };
    carrier.validate().unwrap();
    s.faces[f].surface = carrier;
    for c in &mut s.loops[wire].coedges {
        for p in &mut c.pcurve.control_points {
            p[0] = 4. + 2. * p[0];
        }
        if c.edge == edge {
            *c = c
                .clone()
                .with_trim(CoedgeTrim {
                    curve_parameter: [0.1, 0.9],
                    pcurve_parameter: [0.1, 0.9],
                    periodic_lift: [[0, 0], [1, 0]],
                })
                .unwrap();
        }
    }
    s.validate().unwrap();
    (s, edge, f)
}
#[test]
fn a_single_complete_face_can_cross_the_actual_periodic_seam() {
    let (s, e, f) = seam_crossing_box();
    let before = s.clone();
    let p =
        edge_partition::split(&s, e, &[0.25, 0.5, 0.75], "cross-seam", Limits::default()).unwrap();
    assert_eq!(p.model().faces[f].surface, s.faces[f].surface);
    let relation = &p.relations()[0];
    assert_eq!(relation.fragments.len(), 4);
    assert_eq!(relation.original.periodic_lift, [[0, 0], [1, 0]]);
    assert!(
        relation
            .fragments
            .iter()
            .any(|r| r.transferred.periodic_lift[0][0] == 0)
    );
    assert!(
        relation
            .fragments
            .iter()
            .any(|r| r.transferred.periodic_lift[1][0] == 1)
    );
    assert!(p.reverify().unwrap().charts_injective);
    assert!((volume(p.model()) - 1.).abs() < 1e-6);
    assert_eq!(s, before);
}

#[test]
fn approximate_exterior_knot_repetition_cannot_substitute_for_exact_seam_identity() {
    let (mut s, e, f) = seam_crossing_box();
    s.faces[f].surface.knots_u[0] = (-1f64).next_up();
    s.faces[f].surface.validate().unwrap();
    s.validate().unwrap();
    let before = s.clone();
    let err = match edge_partition::split(&s, e, &[0.5], "unproved-seam", Limits::default()) {
        Ok(_) => panic!("rounded periodic storage must not receive an exact seam proof"),
        Err(e) => e,
    };
    assert!(err.message.contains("seam identity"), "{err:?}");
    assert_eq!(s, before);
}
