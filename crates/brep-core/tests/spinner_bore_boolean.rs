use brep_core::{GearSpec, boolean, cylinder, gear};

#[test]
fn spinner_bore_difference_preserves_teeth_or_reports_unproved_coverage() {
    let spec = GearSpec { module: 1.5, teeth: 32, height: 10., helix_angle_deg: -35.,
        herringbone: true, clearance: 0.1, backlash: 0.05, bore: 42., ..Default::default() };
    let sun = gear(&spec).unwrap();
    let caps: Vec<_> = sun
        .faces
        .iter()
        .enumerate()
        .filter_map(|(i, face)| (!face.holes.is_empty()).then_some((i, 0)))
        .collect();
    let ends = [caps[0], caps[1]];
    let shell = sun.bodies[0].outer_shell;
    let wall = brep_core::body_analysis::through_wall::between_holes(&sun, shell, ends).unwrap();
    assert_eq!(wall.len(), 32);
    assert!(
        brep_core::body_analysis::through_wall::between_holes(&sun, shell, [ends[0]; 2]).is_err()
    );
    assert!(
        brep_core::body_analysis::through_wall::between_holes(
            &sun,
            shell,
            [ends[0], (ends[1].0, 1)]
        )
        .is_err()
    );
    let mut inside = 0;
    let mut outside = 0;
    for (index, face) in sun.faces.iter().enumerate() {
        if !face.holes.is_empty() { continue; } // The two trimmed end caps meet the tool.
        let bounds = nurbs_core::radial_bounds::piecewise_axis_radius_bounds(
            &face.surface, 2, [0.;3], 256).unwrap().expect("whole wall radial certificate");
        assert_eq!(wall.binary_search(&index).is_ok(), bounds[1] < 22.);
        if bounds[1] < 22. { inside += 1; }
        else if bounds[0] > 22. { outside += 1; }
        else { panic!("wall could meet the cutter: {bounds:?}"); }
    }
    assert!(inside > 0 && outside > 0);
    assert_eq!(inside + outside + 2, sun.faces.len());
    eprintln!("Whole wall radial certificates: {inside} inside, {outside} outside");
    let cutter = cylinder(22., 10.).unwrap();
    // All four rational arcs at each end of the cutter must transfer to
    // the existing gear cap, retaining the spatial edge's parameterization.
    let mut certified_arcs = 0;
    for (face_index, _) in ends {
        let cap = &sun.faces[face_index].surface;
        let z = cap.control_points[0][0][2];
        for edge in &cutter.edges {
            if edge.curve.control_points.iter().all(|p| p[2] == z) {
                let proof = brep_core::profile_certificate::original_restriction::plane::pullback::
                    stored_affine_trim(cap, &edge.curve, sun.tolerance_mm).unwrap();
                proof.reverify().unwrap();
                assert_eq!(proof.pcurve().weights, edge.curve.weights);
                assert_eq!(proof.pcurve().knots, edge.curve.knots);
                certified_arcs += 1;
            }
        }
    }
    assert_eq!(certified_arcs, 8);
    let reference = gear(&GearSpec { bore: 44., ..spec }).unwrap();
    assert_eq!(reference.validate().unwrap().boundary_edge_count, 0);
    match boolean(&sun, &cutter, "difference") {
        Err(error) => {
            assert_eq!(error.code, "BREP_NURBS_BOOLEAN_UNPROVED");
            eprintln!("Pending spinner bore Boolean: {}", error.message);
        }
        Ok(result) => {
            assert_eq!(result.bodies.len(), 1);
            assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
            for z in [0., 10.] {
                let cap_radius = result.vertices.iter()
                    .filter(|vertex| (vertex.point[2] - z).abs() < 1e-7)
                    .map(|vertex| vertex.point[0].hypot(vertex.point[1])).fold(f64::INFINITY, f64::min);
                assert!((cap_radius - 22.).abs() < 1e-7);
            }
            // The cylindrical cut is strictly inside the tooth roots. Preserve the authored flanks.
            for edge in sun.edges.iter().filter(|edge| edge.curve.degree > 2
                && edge.curve.control_points.iter().all(|p| p[0].hypot(p[1]) > 22.1)) {
                assert!(result.edges.iter().any(|retained| retained.curve == edge.curve));
            }
        }
    }
}

#[test]
fn cap_holes_on_distinct_passages_cannot_share_a_wall() {
    let square =
        |x: f64, width: f64| vec![[x, 0.], [x, width], [x + width, width], [x + width, 0.]];
    let outer = [[-1., -1.], [9., -1.], [9., 5.], [-1., 5.]];
    let model =
        brep_core::extrude_polygon_with_holes(&outer, &[square(0., 2.), square(5., 2.)], 0., 3.)
            .unwrap();
    let caps: Vec<_> = model
        .faces
        .iter()
        .enumerate()
        .filter_map(|(i, face)| (face.holes.len() == 2).then_some(i))
        .collect();
    assert_eq!(caps.len(), 2);
    let locate = |a, b| {
        brep_core::body_analysis::through_wall::between_holes(
            &model,
            model.bodies[0].outer_shell,
            [(caps[0], a), (caps[1], b)],
        )
    };
    assert_eq!(locate(0, 0).unwrap().len(), 4);
    assert_eq!(locate(1, 1).unwrap().len(), 4);
    assert!(locate(0, 1).is_err());
}
