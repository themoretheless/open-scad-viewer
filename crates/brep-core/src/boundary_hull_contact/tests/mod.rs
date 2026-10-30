use super::*;
#[test]
fn variable_annular_contacts_require_whole_original_chart_proofs() {
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        let model = crate::circular_blend::partial_annular_variable_arc(
            20.,
            5.,
            6.,
            radii,
            [0.3, 0.7, 0.4],
            1e-7,
        )
        .unwrap();
        for faces in [[13, 15], [13, 18], [17, 19], [17, 22], [23, 29], [26, 29]] {
            assert!(
                certify_vertex_plane(&model, faces).is_some(),
                "vertex {faces:?}"
            );
        }
        let middle = certify_polar_charts(&model, [5, 7]).expect("regular middle join");
        assert!(middle.polar_join.unwrap().determinant_lower > 0.);
        let exit = certify_joined_charts(&model, [10, 12]).expect("collapsed exit join");
        assert!(exit.source_join.unwrap().report.proven);
        let a = &model.faces[5].surface;
        let b = &model.faces[7].surface;
        let frame = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., 1., 0.]];
        assert!(
            nurbs_core::surface_injectivity::certify_polar_join(a, b, frame, 16, 512)
                .unwrap()
                .is_some()
        );
        assert!(
            nurbs_core::surface_injectivity::certify_polar_join(a, b, frame, 16, 511)
                .unwrap()
                .is_none()
        );
        let mut broken = b.clone();
        broken.control_points[0][1][0] += 1e-12;
        assert!(
            nurbs_core::surface_injectivity::certify_polar_join(a, &broken, frame, 16, 512)
                .unwrap()
                .is_none()
        );
        let mut reversed = b.clone();
        for row in &mut reversed.control_points {
            row[0][2] = 12.;
        }
        assert!(
            nurbs_core::surface_injectivity::certify_polar_join(a, &reversed, frame, 16, 512)
                .unwrap()
                .is_none()
        );
        let a = &model.faces[10].surface;
        let b = &model.faces[12].surface;
        let run = |b: &nurbs_core::surface::Surface, budget| {
            nurbs_core::surface_quotient_injectivity::certify_ruled_join_source_frame(
                a, b, 1, 16, budget,
            )
            .unwrap()
        };
        assert!(!run(b, 511).proven);
        let mut broken = b.clone();
        broken.control_points[0][1][0] += 1e-12;
        assert!(!run(&broken, 512).proven);
        let mut reversed = b.clone();
        for row in &mut reversed.control_points {
            row[0][2] = 12.;
        }
        assert!(!run(&reversed, 512).proven);
        let mut order = b.clone();
        let n = order.control_points.len();
        order.control_points[n - 2][0][2] += 1e-12;
        assert!(!run(&order, 512).proven);
    }
}
#[test]
fn skewed_station_hulls_meet_only_at_owned_vertex() {
    let section = |z: f64, t: f64| {
        vec![
            [
                [[0.5, 0.], [0.5, 0.5], [0., 0.5]],
                [[0., 0.5], [-0.5, 0.5], [-0.5, 0.]],
                [[-0.5, 0.], [-0.5, -0.5], [0., -0.5]],
                [[0., -0.5], [0.5, -0.5], [0.5, 0.]],
            ]
            .into_iter()
            .map(|p| nurbs_core::curve::Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p
                    .into_iter()
                    .map(|[x, y]| vec![x - t * y, y + t * x, z])
                    .collect(),
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            })
            .collect(),
        ]
    };
    let model =
        crate::rational_section_loft(&[section(0., 0.), section(5., 0.25), section(10., 0.5)])
            .unwrap();
    let c = certify(&model, [0, 7]).expect("skewed common station vertex");
    assert!(c.vertex.is_some());
    assert!(c.edges.is_empty());
    let mut changed = model.clone();
    changed.vertices[c.vertex.unwrap()].point[0] =
        changed.vertices[c.vertex.unwrap()].point[0].next_up();
    assert!(certify(&changed, [0, 7]).is_none());
}
#[test]
fn retained_hollow_sweep_vertex_contacts_require_authored_vertex_ownership() {
    let section = |z| {
        // Author exact quadrant endpoints rather than rounded trig values.
        let ring = |r: f64| {
            [
                [[r, 0.], [r, r], [0., r]],
                [[0., r], [-r, r], [-r, 0.]],
                [[-r, 0.], [-r, -r], [0., -r]],
                [[0., -r], [r, -r], [r, 0.]],
            ]
            .into_iter()
            .map(|p| nurbs_core::curve::Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p.into_iter().map(|xy| vec![xy[0], xy[1], z]).collect(),
                weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                periodic: false,
            })
            .collect::<Vec<_>>()
        };
        let outer = ring(0.5);
        let inner = ring(0.2)
            .into_iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect();
        vec![outer, inner]
    };
    let model =
        crate::rational_section_loft(&[section(0.), section(5.), section(10.)]).unwrap();
    let before = model.clone();
    let walls = model.faces.len() - 2;
    let mut vertices = Vec::new();
    for a in 0..walls {
        for b in a + 1..walls {
            if let Some(c) = certify(&model, [a, b]) {
                if c.vertex.is_some() {
                    vertices.push(c);
                }
            }
        }
    }
    assert!(
        !vertices.is_empty(),
        "actual diagonal wall panels must exercise vertex-only contact"
    );
    for c in vertices {
        let vertex = c.vertex.unwrap();
        let mut broken = model.clone();
        // Keep all geometry identical but give one face independent edges
        // and vertices. Coordinate coincidence cannot authorize contact.
        let f = &model.faces[c.faces[1]];
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for i in 0..broken.loops[wire].coedges.len() {
                let mut edge = model.edges[broken.loops[wire].coedges[i].edge].clone();
                for v in &mut edge.vertices {
                    let point = model.vertices[*v].clone();
                    *v = broken.vertices.len();
                    broken.vertices.push(point);
                }
                broken.loops[wire].coedges[i].edge = broken.edges.len();
                broken.edges.push(edge);
            }
        }
        assert!(certify(&broken, c.faces).is_none(), "{c:?}");
        let mut displaced = model.clone();
        displaced.vertices[vertex].point[0] = displaced.vertices[vertex].point[0].next_up();
        assert!(
            certify(&displaced, c.faces).is_none(),
            "one ULP invalidates exact ownership"
        );
    }
    assert_eq!(model, before);
}
#[test]
fn coincident_geometry_without_shared_topology_is_not_authorized() {
    let m = crate::operations::boolean(
        &crate::cuboid([0.; 3], [10.; 3]).unwrap(),
        &crate::cuboid([2.; 3], [8.; 3]).unwrap(),
        "difference",
    )
    .unwrap();
    let mut examples = [None, None];
    for a in 0..m.faces.len() {
        for b in a + 1..m.faces.len() {
            if let Some(c) = certify(&m, [a, b]) {
                let k = usize::from(c.vertex.is_some());
                if examples[k].is_none() {
                    examples[k] = Some(c);
                }
            }
        }
    }
    for c in examples.into_iter().map(Option::unwrap) {
        let mut broken = m.clone();
        let face = &m.faces[c.faces[1]];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for i in 0..broken.loops[wire].coedges.len() {
                let old = broken.loops[wire].coedges[i].edge;
                let mut edge = broken.edges[old].clone();
                for v in &mut edge.vertices {
                    let vertex = broken.vertices[*v].clone();
                    *v = broken.vertices.len();
                    broken.vertices.push(vertex);
                }
                broken.loops[wire].coedges[i].edge = broken.edges.len();
                broken.edges.push(edge);
            }
        }
        assert!(certify(&broken, c.faces).is_none());
    }
}
#[test]
fn overlapping_face_interiors_and_microscopic_gaps_are_not_shared_boundaries() {
    let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    assert!(certify(&m, [0, 0]).is_none());
    let c = (0..6)
        .flat_map(|a| (a + 1..6).map(move |b| [a, b]))
        .find_map(|p| certify(&m, p))
        .unwrap();
    let axis = (0..3)
        .find(|&k| c.contact_enclosure[k][0] < c.contact_enclosure[k][1])
        .unwrap();
    for e in c.edges {
        for p in &mut m.edges[e].curve.control_points {
            p[(axis + 1) % 3] += 1e-12;
        }
    }
    assert!(certify(&m, c.faces).is_none());
}

#[test]
fn joined_projection_requires_source_seam_budget_order_and_dominance() {
    let model =
        crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
    for (a, b, end, projection) in [
        (0, 2, 0, [[0., 1., 0.], [1., 0., -1.]]),
        (10, 12, 1, [[1., 0., 0.], [0., 1., -1.]]),
    ] {
        let blend = &model.faces[a].surface;
        let wall = &model.faces[b].surface;
        let run = |wall: &nurbs_core::surface::Surface, budget| {
            nurbs_core::surface_quotient_injectivity::certify_ruled_join(
                blend, wall, end, projection, 16, budget,
            )
            .unwrap()
        };
        let r = run(wall, 512);
        assert!(r.proven);
        assert_eq!(r.cells, 512);
        assert!(r.dominance_margin_lower.unwrap() > 0.);
        assert!(!run(wall, 511).proven);
        let mut weak = projection;
        weak[1][if a == 0 { 0 } else { 1 }] = 0.25;
        assert!(
            nurbs_core::surface_quotient_injectivity::certify(blend, end, weak, 16, 256)
                .unwrap()
                .proven
        );
        assert!(
            !nurbs_core::surface_quotient_injectivity::certify_ruled_join(
                blend, wall, end, weak, 16, 512
            )
            .unwrap()
            .proven
        );
        let mut moved = wall.clone();
        moved.control_points[0][1][0] += 1e-12;
        assert!(!run(&moved, 512).proven);
        let mut weight = wall.clone();
        weight.weights[0][0] *= 2.;
        assert!(!run(&weight, 512).proven);
        let mut order = wall.clone();
        let i = if end == 0 {
            1
        } else {
            order.control_points.len() - 2
        };
        order.control_points[i][0][2] += 1e-12;
        assert!(!run(&order, 512).proven);
        let mut reversed = wall.clone();
        for row in &mut reversed.control_points {
            row[0][2] = 12.;
        }
        assert!(!run(&reversed, 512).proven);
        let c = certify_joined_charts(&model, [a, b]).unwrap();
        assert!(c.joined_proof.as_ref().unwrap().report.proven);
        assert!(c.edges.contains(&if a == 0 { 2 } else { 27 }));
    }
}
#[test]
fn joined_projection_is_covariant_for_coordinate_rotation_and_translation() {
    let source =
        crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
    let model = crate::transform::affine(
        &source,
        [
            [0., -1., 0., 123.],
            [0., 0., -1., -45.],
            [1., 0., 0., 67.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    for faces in [[0, 2], [10, 12]] {
        let certificate = certify_joined_charts(&model, faces).unwrap();
        let proof = certificate.joined_proof.unwrap();
        assert!(proof.report.proven && proof.report.cells == 512);
        let mut broken = model.clone();
        broken.faces[proof.wall_face].surface.control_points[0][1][0] += 1e-12;
        assert!(certify_joined_charts(&broken, faces).is_none());
    }
}
#[test]
fn strict_bezier_basis_excludes_intermediate_support_controls_from_vertex_contact() {
    let model =
        crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
    let plane: [&[f64]; 3] = [&[0., 0., 6.], &[1., 0., 6.], &[0., 1., 6.]];
    let point = &[20., 0., 6.];
    let surface = &model.faces[2].surface;
    assert_ne!(surface.control_points[1][1], point);
    assert_eq!(surface.control_points[1][1][2], 6.);
    assert!(bezier_plane_vertex_only(surface, plane, point));
    let mut extra = surface.clone();
    let n = extra.control_points.len();
    extra.control_points[n - 1][1][2] = 6.;
    assert!(!bezier_plane_vertex_only(&extra, plane, point));
    assert!(certify_vertex_plane(&model, [1, 2]).is_some());
}
#[test]
fn oblique_straight_edge_support_requires_exact_segment_ownership() {
    let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
    let map = |p: &mut [f64]| {
        let [x, y, z] = [p[0], p[1], p[2]];
        p[0] = x + y;
        p[1] = y + z;
        p[2] = z + x;
    };
    for v in &mut model.vertices {
        map(&mut v.point);
    }
    for e in &mut model.edges {
        for p in &mut e.curve.control_points {
            map(p);
        }
    }
    for f in &mut model.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            map(p);
        }
    }
    model.validate().unwrap();
    let certificate = (0..6)
        .flat_map(|a| (a + 1..6).map(move |b| [a, b]))
        .filter(|p| certify_axis(&model, *p).is_none())
        .find_map(|p| certify_edge_plane(&model, p))
        .unwrap();
    assert_eq!(certificate.edges.len(), 1);
    assert!(certificate.vertex.is_none());
    let mut moved = model.clone();
    let edge = certificate.edges[0];
    moved.edges[edge].curve.control_points[0][0] += 1e-12;
    assert!(certify_edge_plane(&moved, certificate.faces).is_none());
    let mut overlap = model.clone();
    overlap.faces[certificate.faces[1]].surface =
        overlap.faces[certificate.faces[0]].surface.clone();
    assert!(certify_edge_plane(&overlap, certificate.faces).is_none());
}
#[test]
fn oblique_support_is_exact_and_requires_the_owned_vertex() {
    let mut model = crate::analytic::sphere(3.).unwrap();
    let map = |p: &mut [f64]| {
        let [x, y, z] = [p[0], p[1], p[2]];
        p[0] = x - y;
        p[1] = -x + 2. * y;
        p[2] = x - 2. * y + z;
    };
    for v in &mut model.vertices {
        map(&mut v.point);
    }
    for e in &mut model.edges {
        for p in &mut e.curve.control_points {
            map(p);
        }
    }
    for f in &mut model.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            map(p);
        }
    }
    model.validate().unwrap();

    // The supporting-plane refinement can now certify this fixture as well.
    if let Some(axis) = certify_axis(&model, [0, 2]) {
        assert_eq!(axis.vertex, Some(4));
        assert_eq!(
            axis.contact_enclosure,
            model.vertices[4].point.map(|v| [v, v])
        );
    }
    let c = certify_vertex_plane(&model, [0, 2]).unwrap();
    assert_eq!(c.vertex, Some(4));
    assert!(c.edges.is_empty());
    assert_eq!(c.contact_enclosure, model.vertices[4].point.map(|v| [v, v]));
    let mut overlap = model.clone();
    overlap.faces[2].surface = overlap.faces[0].surface.clone();
    assert!(certify_vertex_plane(&overlap, [0, 2]).is_none());
    let mut moved = model.clone();
    moved.vertices[4].point[0] += 1e-12;
    assert!(certify_vertex_plane(&moved, [0, 2]).is_none());
}
#[test]
fn opposite_sphere_quadrants_meet_only_at_the_authored_pole() {
    let model = crate::analytic::sphere(3.).unwrap();
    let before = format!("{model:?}");
    for (faces, pole) in [([0, 2], 4), ([1, 3], 4), ([4, 6], 5), ([5, 7], 5)] {
        let c = certify(&model, faces).unwrap();
        assert_eq!(c.vertex, Some(pole));
        assert!(c.edges.is_empty());
        assert_eq!(
            c.contact_enclosure,
            model.vertices[pole].point.map(|x| [x, x])
        );
        let mut separate = model.clone();
        for wire in std::iter::once(separate.faces[faces[1]].outer)
            .chain(separate.faces[faces[1]].holes.clone())
        {
            for i in 0..separate.loops[wire].coedges.len() {
                let old = separate.loops[wire].coedges[i].edge;
                let mut edge = separate.edges[old].clone();
                for v in &mut edge.vertices {
                    let vertex = separate.vertices[*v].clone();
                    *v = separate.vertices.len();
                    separate.vertices.push(vertex);
                }
                separate.loops[wire].coedges[i].edge = separate.edges.len();
                separate.edges.push(edge);
            }
        }
        assert!(certify(&separate, faces).is_none());
    }
    assert_eq!(format!("{model:?}"), before);
}
#[test]
fn adjacent_hemisphere_quadrants_meet_only_at_a_shared_equator_vertex() {
    let model = crate::analytic::sphere(3.).unwrap();
    for upper in 0..4 {
        for quarter in [(upper + 1) % 4, (upper + 3) % 4] {
            let c = certify(&model, [upper, quarter + 4]).unwrap();
            assert_eq!(
                c.vertex,
                Some(if quarter == (upper + 1) % 4 {
                    quarter
                } else {
                    upper
                })
            );
            assert!(c.edges.is_empty());
        }
    }
}
#[test]
fn a_plane_cutting_a_control_hull_cannot_discard_its_interior_controls() {
    let mut model = crate::analytic::sphere(3.).unwrap();
    for row in &mut model.faces[2].surface.control_points {
        for p in row {
            p[0] = 0.;
        }
    }
    // Face 0 straddles X=0; equality there is not a supporting-plane
    // constraint. Filtering only its X=0 controls would hide intersections.
    model.faces[0].surface.control_points[1][0][0] = -3.;
    assert!(certify(&model, [0, 2]).is_none());
}
