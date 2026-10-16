use super::*;
#[test]
fn sphere_equator_is_the_only_contact_of_opposite_exact_disk_sides() {
    let model = crate::analytic::sphere(3.).unwrap();
    let before = format!("{model:?}");
    for faces in [[0, 4], [1, 5], [2, 6], [3, 7]] {
        assert!(
            certify_opposite(&model, faces).unwrap().is_some(),
            "{faces:?}"
        );
    }
    let mut changed = model.clone();
    changed.faces[4].surface.control_points[1][1][2] += 1e-12;
    assert!(certify_opposite(&changed, [0, 4]).unwrap().is_none());
    let mut same_side = model.clone();
    for row in &mut same_side.faces[4].surface.control_points {
        for p in row {
            p[2] = -p[2];
        }
    }
    assert!(certify_opposite(&same_side, [0, 4]).unwrap().is_none());
    let rounded = crate::analytic::sphere(2.).unwrap();
    assert!(certify_opposite(&rounded, [0, 4]).unwrap().is_none());
    assert_eq!(format!("{model:?}"), before);
}
#[test]
fn adjacent_cylinder_sides_use_exact_authored_axis_planes() {
    let model = crate::analytic::cylinder(2., 4.).unwrap();
    for faces in [[0, 1], [1, 2], [2, 3], [0, 3]] {
        assert!(
            inspect_opposite_pair(&model, faces).unwrap().is_some(),
            "{faces:?}"
        );
    }
    let a = &model.faces[0].surface;
    let b = &model.faces[1].surface;
    let edge = model.loops[model.faces[0].outer]
        .coedges
        .iter()
        .find(|ca| {
            model.loops[model.faces[1].outer]
                .coedges
                .iter()
                .any(|cb| cb.edge == ca.edge)
        })
        .unwrap();
    let other = model.loops[model.faces[1].outer]
        .coedges
        .iter()
        .find(|cb| cb.edge == edge.edge)
        .unwrap();
    let mut same_side = b.clone();
    for row in &mut same_side.control_points {
        for p in row {
            p[0] = p[0].abs();
        }
    }
    assert!(!opposite_axis_sides(
        a,
        &same_side,
        &edge.pcurve,
        &other.pcurve,
        &model.edges[edge.edge].curve
    ));
}
#[test]
fn collapsed_secondary_boundary_must_be_an_owned_curve_endpoint() {
    let model =
        crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
    for pair in [[0, 1], [10, 11]] {
        let c = certify(&model, pair).unwrap();
        assert_eq!(c.sided_face, pair[0]);
        let mut extra = model.clone();
        // Moving an opposite zero corner within the support plane creates
        // an extra reachable contact rather than an allowed collapsed end.
        let controls = &mut extra.faces[pair[0]].surface.control_points;
        let pole = [0, controls.len() - 1]
            .into_iter()
            .find(|&i| controls[i].iter().all(|p| p == &controls[i][0]))
            .unwrap();
        controls[pole][2][0] += 1e-12;
        assert!(
            !bezier_plane_edge_only(
                &extra.faces[pair[0]].surface,
                [&[0., 0., 6.], &[1., 0., 6.], &[0., 1., 6.]],
                (1, 0)
            ),
            "direct helper pair={pair:?}"
        );
        assert!(
            certify(&extra, pair).is_none(),
            "pair={pair:?} certificate={:?}",
            certify(&extra, pair)
        );
    }
}
#[test]
fn tangent_boundary_control_rows_remain_strictly_separated() {
    let model = crate::analytic::cylinder(2., 4.).unwrap();
    let edge = model.loops[model.faces[0].outer]
        .coedges
        .iter()
        .find(|ca| {
            model.loops[model.faces[1].outer]
                .coedges
                .iter()
                .any(|cb| cb.edge == ca.edge)
        })
        .unwrap();
    let other = model.loops[model.faces[1].outer]
        .coedges
        .iter()
        .find(|cb| cb.edge == edge.edge)
        .unwrap();
    let mut a = model.faces[0].surface.clone();
    let b = &model.faces[1].surface;
    let (fixed, row) = boundary(&a, &edge.pcurve, &model.edges[edge.edge].curve).unwrap();
    let axis = (0..3)
        .find(|&k| {
            model.edges[edge.edge]
                .curve
                .control_points
                .iter()
                .all(|p| p[k] == model.edges[edge.edge].curve.control_points[0][k])
        })
        .unwrap();
    let plane = model.edges[edge.edge].curve.control_points[0][axis];
    for (u, r) in a.control_points.iter_mut().enumerate() {
        for (v, p) in r.iter_mut().enumerate() {
            if [u, v][fixed] == 1 {
                p[axis] = plane;
            }
        }
    }
    assert!(opposite_axis_sides(
        &a,
        b,
        &edge.pcurve,
        &other.pcurve,
        &model.edges[edge.edge].curve
    ));
    let opposite = if row == 0 { 2 } else { 0 };
    if fixed == 0 {
        a.control_points[opposite][0][axis] = plane;
    } else {
        a.control_points[0][opposite][axis] = plane;
    }
    assert!(!opposite_axis_sides(
        &a,
        b,
        &edge.pcurve,
        &other.pcurve,
        &model.edges[edge.edge].curve
    ));
}
#[test]
fn cylinder_caps_admit_curved_exact_trims_but_not_nearby_lifts() {
    let model = crate::analytic::cylinder(2., 4.).unwrap();
    for side in 0..4 {
        for cap in 4..6 {
            let c = inspect_pair(&model, [side, cap])
                .unwrap()
                .expect("exact curved cap trim");
            assert_eq!(c.sided_face, side);
            assert_eq!(c.planar_face, cap);
            let mut broken = model.clone();
            let wire = broken.faces[cap].outer;
            let use_ = broken.loops[wire]
                .coedges
                .iter_mut()
                .find(|u| u.edge == c.edge)
                .unwrap();
            use_.pcurve.control_points[1][0] += 1e-12;
            assert!(certify(&broken, [side, cap]).is_none());
        }
    }
}
#[test]
fn cube_only_certifies_adjacent_pairs() {
    let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut certified = 0;
    for a in 0..6 {
        for b in a + 1..6 {
            if let Some(c) = certify(&m, [a, b]) {
                certified += 1;
                assert!(c.edge < m.edges.len());
                assert_ne!(c.planar_face, c.sided_face);
            }
        }
    }
    assert_eq!(certified, 12);
}
#[test]
fn topology_alone_cannot_hide_a_fold_or_shifted_boundary() {
    let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let pair = (0..6)
        .flat_map(|a| (a + 1..6).map(move |b| [a, b]))
        .find(|&p| certify(&m, p).is_some())
        .unwrap();
    let mut bad = m.clone();
    for row in &mut bad.faces[pair[0]].surface.control_points {
        for p in row {
            p[0] += 0.01;
            p[1] += 0.01;
            p[2] += 0.01;
        }
    }
    assert!(certify(&bad, pair).is_none());
}
#[test]
fn curved_sided_patch_passes_but_fold_and_extra_plane_controls_do_not() {
    let plane = nurbs_core::surface::Surface {
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
    };
    let mut curved = plane.clone();
    curved.degree_u = 2;
    curved.knots_u = vec![0., 0., 0., 1., 1., 1.];
    curved.weights = vec![vec![1.; 2], vec![1.25; 2], vec![0.75; 2]];
    curved.control_points = (0..3)
        .map(|i| {
            (0..2)
                .map(|j| vec![i as f64 * 0.5, j as f64, [0., 0.25, 1.][i]])
                .collect()
        })
        .collect();
    curved.validate().unwrap();
    assert!(sided(&curved, &plane, (0, 0)));
    curved.control_points[1][0][2] = -0.25;
    assert!(!sided(&curved, &plane, (0, 0)));
    curved.control_points[1][0][2] = 0.;
    assert!(sided(&curved, &plane, (0, 0)));
    curved.control_points[2][0][2] = 0.;
    assert!(!sided(&curved, &plane, (0, 0)));
}
#[test]
fn sheared_translated_cube_retains_twelve_shared_boundaries() {
    let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let transform = |p: &mut [f64]| {
        let q = [p[0], p[1], p[2]];
        p[0] = 16. + q[0] + 0.25 * q[1];
        p[1] = -8. + q[1] + 0.5 * q[2];
        p[2] = 4. + q[2] + 0.125 * q[0];
    };
    for v in &mut m.vertices {
        transform(&mut v.point);
    }
    for e in &mut m.edges {
        for p in &mut e.curve.control_points {
            transform(p);
        }
    }
    for f in &mut m.faces {
        for row in &mut f.surface.control_points {
            for p in row {
                transform(p);
            }
        }
    }
    m.validate().unwrap();
    let mut count = 0;
    for a in 0..6 {
        for b in a + 1..6 {
            count += usize::from(inspect_pair(&m, [a, b]).unwrap().is_some());
        }
    }
    assert_eq!(count, 12);
    assert!(inspect_pair(&m, [0, 0]).is_err());
    assert!(inspect_pair(&m, [0, 6]).is_err());
}
#[test]
fn straight_shared_edge_requires_exact_opposite_strict_sides() {
    let edge = Curve::from_polyline(vec![vec![1., 0., 0.], vec![1., 0., 2.]]).unwrap();
    let uv = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
    let make = |side: f64| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            edge.control_points.clone(),
            vec![vec![2., side, 0.], vec![2., side, 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let a = make(1.);
    let b = make(-1.);
    assert!(separates_surfaces(&a, &b, &uv, &uv, &edge).unwrap());
    assert!(!separates_surfaces(&a, &a, &uv, &uv, &edge).unwrap());
    let mut changed = b.clone();
    changed.control_points[1][1][1] = 0.;
    assert!(separates_surfaces(&a, &changed, &uv, &uv, &edge).unwrap());
    // An off-boundary pole on the shared line defeats every strict plane.
    let mut bad = b.clone();
    bad.control_points[1][1] = edge.control_points[1].clone();
    assert!(!separates_surfaces(&a, &bad, &uv, &uv, &edge).unwrap());
    let mut shifted = b;
    shifted.control_points[0][0][1] = f64::EPSILON;
    assert!(!separates_surfaces(&a, &shifted, &uv, &uv, &edge).unwrap());
}
#[test]
fn curved_faces_with_rational_curved_shared_edge_are_separated() {
    let edge = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0.5, 1., 0.], vec![1., 0., 0.]],
        weights: vec![1., 0.75, 1.],
        periodic: false,
    };
    let pcurve = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
    let make = |side: f64| Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: edge.knots.clone(),
        control_points: vec![
            edge.control_points.clone(),
            vec![
                vec![0., 0.25, side],
                vec![0.5, 1.5, side * 0.25],
                vec![1., 0.25, side * 2.],
            ],
        ],
        weights: vec![edge.weights.clone(), vec![0.75, 1.25, 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let a = make(1.);
    let b = make(-1.);
    assert!(separates_surfaces(&a, &b, &pcurve, &pcurve, &edge).unwrap());
    assert!(separates_surfaces(&b, &a, &pcurve, &pcurve, &edge).unwrap());
    assert!(!separates_surfaces(&a, &a, &pcurve, &pcurve, &edge).unwrap());
    let mut fold = b.clone();
    fold.control_points[1][1][2] = 0.5;
    assert!(!separates_surfaces(&a, &fold, &pcurve, &pcurve, &edge).unwrap());
    fold.control_points[1][1][2] = 0.;
    assert!(!separates_surfaces(&a, &fold, &pcurve, &pcurve, &edge).unwrap());
    let mut shifted = b.clone();
    shifted.weights[0][1] = 0.5;
    assert!(!separates_surfaces(&a, &shifted, &pcurve, &pcurve, &edge).unwrap());
    let tangent = |mut s: Surface| {
        s.degree_u = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.control_points.insert(1, s.control_points[0].clone());
        s.weights.insert(1, s.weights[0].clone());
        s
    };
    let mut ta = tangent(a.clone());
    let mut tb = tangent(b.clone());
    let mut oblique = edge.clone();
    let place = |p: &mut Vec<f64>| {
        p[2] += p[0] + 2. * p[1] + 8.;
        p[0] += 16.;
        p[1] -= 4.;
    };
    for s in [&mut ta, &mut tb] {
        for row in &mut s.control_points {
            for p in row {
                place(p);
            }
        }
    }
    for p in &mut oblique.control_points {
        place(p);
    }
    assert!(separates_surfaces(&ta, &tb, &pcurve, &pcurve, &oblique).unwrap());
    let mut extra = tb.clone();
    extra.control_points[2][0] = extra.control_points[0][0].clone();
    assert!(!separates_surfaces(&ta, &extra, &pcurve, &pcurve, &oblique).unwrap());
    let reversed = Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap();
    assert!(separates_surfaces(&a, &b, &reversed, &pcurve, &edge).unwrap());
}
#[test]
fn brep_pair_identifies_curved_edge_without_claiming_face_validity() {
    let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let faces = [0, 2];
    let old = certify(&m, faces).unwrap();
    let edge = m.edges[old.edge].curve.clone();
    let mut bowed = edge.clone();
    bowed.degree = 2;
    bowed.knots = vec![0., 0., 0., 1., 1., 1.];
    bowed.weights = vec![1.; 3];
    let mut mid: Vec<_> = edge.control_points[0]
        .iter()
        .zip(&edge.control_points[1])
        .map(|(a, b)| (a + b) * 0.5)
        .collect();
    // Bow along the sum of inward face directions. The bisector plane
    // contains the resulting curved edge and separates both face interiors.
    let mut prepared = Vec::new();
    for face in faces {
        let f = &m.faces[face];
        let c = m.loops[f.outer]
            .coedges
            .iter()
            .find(|c| c.edge == old.edge)
            .unwrap();
        let (axis, row) = boundary(&f.surface, &c.pcurve, &edge).unwrap();
        let at = |r: usize| {
            if axis == 0 {
                f.surface.control_points[r][0].clone()
            } else {
                f.surface.control_points[0][r].clone()
            }
        };
        let p = at(row);
        let q = at(1 - row);
        for k in 0..3 {
            mid[k] += (q[k] - p[k]) * 0.125;
        }
        prepared.push((face, axis, row));
    }
    bowed.control_points = vec![
        edge.control_points[0].clone(),
        mid,
        edge.control_points[1].clone(),
    ];
    for (face, axis, row) in prepared {
        let s = &mut m.faces[face].surface;
        let original = s.control_points.clone();
        let start = if axis == 0 {
            &original[row][0]
        } else {
            &original[0][row]
        };
        let reverse = *start != edge.control_points[0];
        let sizes = if axis == 0 { [2, 3] } else { [3, 2] };
        s.control_points = (0..sizes[0])
            .map(|u| {
                (0..sizes[1])
                    .map(|v| {
                        let (r, t) = if axis == 0 { (u, v) } else { (v, u) };
                        if r == row {
                            return bowed.control_points[if reverse { 2 - t } else { t }].clone();
                        }
                        let (p, q) = if axis == 0 {
                            (&original[r][0], &original[r][1])
                        } else {
                            (&original[0][r], &original[1][r])
                        };
                        (0..3)
                            .map(|k| p[k] + (q[k] - p[k]) * t as f64 * 0.5)
                            .collect()
                    })
                    .collect()
            })
            .collect();
        s.weights = vec![vec![1.; sizes[1]]; sizes[0]];
        if axis == 0 {
            s.degree_v = 2;
            s.knots_v = bowed.knots.clone();
        } else {
            s.degree_u = 2;
            s.knots_u = bowed.knots.clone();
        }
    }
    m.edges[old.edge].curve = bowed;
    let certificate = inspect_opposite_pair(&m, faces).unwrap().unwrap();
    assert_eq!(certificate.edge, old.edge);
    assert_eq!(certificate.faces, faces);
    let report = crate::face_contacts::inspect(
        &m,
        1e-8,
        crate::face_contacts::Limits {
            pairs: 100,
            cells: 10000,
            domain_cells: 100000,
            cells_per_pair: 16,
            domain_cells_per_pair: 1000,
        },
    )
    .unwrap();
    let pair = report.pairs.iter().find(|p| p.faces == faces).unwrap();
    assert_eq!(pair.reason, "shared-boundary");
    assert!(matches!(
        pair.boundary,
        Some(crate::face_contacts::SharedBoundary::OppositeSides(_))
    ));
    assert!(pair.result.is_none());
    assert!(report.all_pairs_classified);
    assert_eq!(
        report
            .pairs
            .iter()
            .filter(|p| p.reason == "shared-boundary")
            .count(),
        12
    );

    assert!(inspect_pair(&m, faces).unwrap().is_none());
    assert!(inspect_opposite_pair(&m, [0, 0]).is_err());
}

#[test]
fn reflected_step_face_senses_preserve_exact_quarter_disk_side_ownership() {
    let original=crate::analytic::sphere_exact(3.).unwrap();
    let mut canonical=crate::face_senses::canonical_face_senses(&original).unwrap();
    let before=canonical.clone();
    for faces in [[0,4],[1,5],[2,6],[3,7]] {assert!(certify_opposite(&canonical,faces).unwrap().is_some(),"{faces:?}");}
    assert_eq!(canonical,before);
    canonical.faces[4].surface.control_points[1][1][2]=canonical.faces[4].surface.control_points[1][1][2].next_up();
    assert!(certify_opposite(&canonical,[0,4]).unwrap().is_none());
}
