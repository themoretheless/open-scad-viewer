use brep_core::region_replacement as r;
#[test]
fn six_valent_stock_and_unequal_profile_body_are_closed_without_source_edits() {
    let source = r::six_edge_corner_stock().unwrap();
    let before = source.clone();
    let corner = r::propose_six_edge_corner(&source, [0., 0.25, 0.5]).unwrap();
    assert_eq!(source, before);
    corner.model.validate().unwrap();
    assert_eq!(
        corner.model.vertices.len() + corner.model.faces.len(),
        corner.model.edges.len() + 2
    );
    let apex = source
        .vertices
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.point[2].total_cmp(&b.point[2]))
        .unwrap()
        .0;
    assert_eq!(
        source
            .edges
            .iter()
            .filter(|e| e.vertices.contains(&apex))
            .count(),
        6
    );
    assert!(!corner.smooth_edges.is_empty());
    eprintln!(
        "six-edge top_faces={} F={} E={} V={} smooth={}",
        corner.top_faces.len(),
        corner.model.faces.len(),
        corner.model.edges.len(),
        corner.model.vertices.len(),
        corner.smooth_edges.len()
    );
}
#[test]
fn exact_saved_boundary_certificate_covers_all_thirty_g2_seams_and_face_pairs() {
    let source = r::six_edge_corner_stock().unwrap();
    let mut corner = r::propose_six_edge_corner(&source, [0., 0.25, 0.5]).unwrap();
    let proof = r::inspect_six_edge_graph(&source, &corner).unwrap();
    assert_eq!(proof.roof_cells, 19);
    assert_eq!(proof.smooth_seams, 30);
    assert_eq!(proof.exact_jet_equalities, 180);
    assert_eq!(proof.classified_face_pairs, 325);
    assert!(proof.minimum_height_mm > 0. && proof.volume_mm3 > 0. && proof.material_inside_source);
    for old in 0..source.faces.len() {
        let id = source.1.faces[old];
        assert!(
            corner.model.1.faces.contains(&id)
                || corner
                    .model
                    .1
                    .lineage
                    .iter()
                    .any(|r| r.entity_kind == "face"
                        && r.operation == "persist"
                        && r.parents.contains(&id))
        );
    }
    corner.smooth_edges.pop();
    assert!(r::inspect_six_edge_graph(&source, &corner).is_err());
}
#[test]
fn stale_body_handles_and_invalid_inputs_cannot_inherit_canonical_proof() {
    let source = r::six_edge_corner_stock().unwrap();
    let before = source.clone();
    let mut corner = r::propose_six_edge_corner(&source, [0., 0.25, 0.5]).unwrap();
    corner.handles[0] = 0.125;
    assert!(r::inspect_six_edge_graph(&source, &corner).is_err());
    for handles in [
        [f64::NAN, 0., 0.],
        [-0.25, 0., 0.],
        [1.25, 0., 0.],
        [0.1, 0., 0.],
    ] {
        assert!(r::propose_six_edge_corner(&source, handles).is_err());
    }
    for radius in [0., f64::NAN, 1e-300, 1e300] {
        assert!(r::propose_six_edge_corner_radii(&source, [radius, 2.8, 0.9]).is_err());
    }
    assert_eq!(source, before);
}
#[test]
fn prescribed_paired_radii_match_independent_actual_arm_shape_tensors() {
    let source = r::six_edge_corner_stock().unwrap();
    let body = r::build_six_edge_corner_radii(&source, [2.6, 2.8, 0.9]).unwrap();
    assert_eq!(
        body.closure().closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let corner = body.corner();
    let proof = r::inspect_six_edge_graph(&source, corner).unwrap();
    assert!(proof.curvature_upper_per_mm <= 10.);
    for (i, xy) in [
        [0., 8.],
        [0., -8.],
        [8., 0.],
        [-8., 0.],
        [8., -8.],
        [-8., 8.],
    ]
    .into_iter()
    .enumerate()
    {
        let face = corner
            .top_faces
            .iter()
            .copied()
            .find(|&f| {
                let s = &corner.model.faces[f].surface;
                if xy[0] < s.knots_u[s.degree_u]
                    || xy[0] > s.knots_u[s.control_points.len()]
                    || xy[1] < s.knots_v[s.degree_v]
                    || xy[1] > s.knots_v[s.control_points[0].len()]
                {
                    return false;
                }
                brep_core::face_domain::FaceDomain::new(&corner.model, f, 1e-8)
                    .unwrap()
                    .classify([[xy[0]; 2], [xy[1]; 2]], 10000)
                    .unwrap()
                    .location
                    == nurbs_core::trim_domain::Location::Inside
            })
            .unwrap();
        let shape = nurbs_core::surface_shape_operator::at(
            &corner.model.faces[face].surface,
            xy,
            [nurbs_core::curve_differential::Side::Automatic; 2],
        )
        .unwrap();
        use nurbs_core::interval_eval::Interval as I;
        let tensor = shape.tensor.unwrap();
        let mut trace = I::point(0.);
        for k in 0..3 {
            trace = trace
                .add(I::new(tensor[k][k][0], tensor[k][k][1]).unwrap())
                .unwrap();
        }
        let k = if trace.hi < 0. {
            I::new(-trace.hi, -trace.lo).unwrap()
        } else {
            trace
        };
        let radius = I::point(1.).div(k).unwrap();
        let requested = [2.6, 2.8, 0.9][i / 2];
        assert!(
            radius.lo >= requested - 1e-7 && radius.hi <= requested + 1e-7,
            "arm {i}: {:?}",
            radius
        );
    }
}
