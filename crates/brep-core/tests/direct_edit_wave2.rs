use brep_core::Model;
use brep_core::analysis::draft::draft_angle;
use brep_core::analysis::surface_classify::Axis;
use brep_core::direct_edit::*;
use nurbs_core::foundation::guards::Budget;
use nurbs_core::surface::Surface;

fn budget() -> Budget {
    Budget::new(20_000_000, 8, 120_000).unwrap()
}

fn face_at(model: &Model, axis: usize, value: f64) -> usize {
    model
        .faces
        .iter()
        .enumerate()
        .find_map(|(face_index, face)| {
            let on_plane = std::iter::once(face.outer)
                .chain(face.holes.iter().copied())
                .flat_map(|wire| &model.loops[wire].coedges)
                .flat_map(|coedge| model.edges[coedge.edge].vertices)
                .all(|vertex| (model.vertices[vertex].point[axis] - value).abs() < 1e-9);
            on_plane.then_some(face_index)
        })
        .expect("face at requested plane")
}

fn plane_patch(z: f64, slope_x: f64, extent: f64) -> Surface {
    let point = |x: f64, y: f64| vec![x, y, z + slope_x * x];
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![point(0., 0.), point(0., extent)],
            vec![point(extent, 0.), point(extent, extent)],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn cylinder_patch() -> Surface {
    // A shallow circular sag between the original top corners. Keeping both
    // endpoints at z=10 is essential: an arc ending at z=0 would collapse two
    // vertical edges and cannot be a topology-preserving face replacement.
    let w = 2. / 5f64.sqrt();
    let middle_z = 7.5;
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![10., 0., 10.], vec![0., 0., 10.]],
            vec![vec![10., 5., middle_z], vec![0., 5., middle_z]],
            vec![vec![10., 10., 10.], vec![0., 10., 10.]],
        ],
        weights: vec![vec![1., 1.], vec![w, w], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn collapsed_cylinder_patch() -> Surface {
    let w = 0.5f64.sqrt();
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![10., 0., 10.], vec![0., 0., 10.]],
            vec![vec![10., 10., 10.], vec![0., 10., 10.]],
            vec![vec![10., 10., 0.], vec![0., 10., 0.]],
        ],
        weights: vec![vec![1., 1.], vec![w, w], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn noisy_plane(model: &mut Model, face: usize, noise: f64) {
    let mut surface = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![],
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    for i in 0..3 {
        let mut row = Vec::new();
        for j in 0..3 {
            let z = 10. + if i == 1 && j == 1 { noise } else { 0. };
            row.push(vec![i as f64 * 5., j as f64 * 5., z]);
        }
        surface.control_points.push(row);
    }
    model.faces[face].surface = surface;
}

fn noisy_cylinder(model: &mut Model, side: usize, noise: f64) {
    let exact = model.faces[side].surface.clone();
    let homogeneous = |row: usize, column: usize| {
        let p = &exact.control_points[row][column];
        ([p[0], p[1], p[2]], exact.weights[row][column])
    };
    let combine = |a: ([f64; 3], f64), b: ([f64; 3], f64), a_weight: f64| {
        let b_weight = 1. - a_weight;
        let w = a_weight * a.1 + b_weight * b.1;
        let p = std::array::from_fn(|axis| {
            (a_weight * a.1 * a.0[axis] + b_weight * b.1 * b.0[axis]) / w
        });
        (p, w)
    };
    let mut elevated_u = vec![vec![]; 4];
    let mut elevated_u_weights = vec![vec![]; 4];
    for column in 0..2 {
        let h0 = homogeneous(0, column);
        let h1 = homogeneous(1, column);
        let h2 = homogeneous(2, column);
        let (q1, w1) = combine(h0, h1, 1. / 3.);
        let (q2, w2) = combine(h2, h1, 1. / 3.);
        let rows = [h0, (q1, w1), (q2, w2), h2];
        for (row, (point, weight)) in rows.into_iter().enumerate() {
            elevated_u[row].push(point.to_vec());
            elevated_u_weights[row].push(weight);
        }
    }
    let mut control_points = vec![vec![]; 4];
    let mut weights = vec![vec![]; 4];
    for row in 0..4 {
        let h0 = (
            std::array::from_fn(|axis| elevated_u[row][0][axis]),
            elevated_u_weights[row][0],
        );
        let h1 = (
            std::array::from_fn(|axis| elevated_u[row][1][axis]),
            elevated_u_weights[row][1],
        );
        let (mut middle, middle_weight) = combine(h0, h1, 0.5);
        if row == 1 || row == 2 {
            middle[0] += noise;
            middle[1] -= noise * 0.5;
        }
        control_points[row] = vec![h0.0.to_vec(), middle.to_vec(), h1.0.to_vec()];
        weights[row] = vec![h0.1, middle_weight, h1.1];
    }
    model.faces[side].surface = Surface {
        degree_u: 3,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
}

fn noisy_periodic_cylinder() -> (Model, usize) {
    let mut model = brep_core::cylinder(2., 5.).unwrap();
    let mut carrier = model.faces[0].surface.clone();
    carrier.control_points = (0..4)
        .flat_map(|face| {
            model.faces[face].surface.control_points[..2]
                .iter()
                .cloned()
        })
        .collect();
    carrier.weights = (0..4)
        .flat_map(|face| model.faces[face].surface.weights[..2].iter().cloned())
        .collect();
    carrier.control_points.extend_from_within(..2);
    carrier.weights.extend_from_within(..2);
    carrier.knots_u = vec![-1., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 5., 5.];
    carrier.periodic_u = true;
    carrier.validate().unwrap();
    for face in 0..4 {
        let shift = match face {
            0 => 8.,
            3 => -8.,
            _ => 0.,
        };
        let wire = model.faces[face].outer;
        for coedge in &mut model.loops[wire].coedges {
            for point in &mut coedge.pcurve.control_points {
                point[0] += face as f64 + shift;
            }
        }
        model.faces[face].surface = carrier.clone();
    }
    let seam = model
        .edges
        .iter()
        .position(|edge| {
            let a = model.vertices[edge.vertices[0]].point;
            let b = model.vertices[edge.vertices[1]].point;
            a[0] == 2. && a[1] == 0. && b[0] == 2. && b[1] == 0. && a[2] != b[2]
        })
        .unwrap();
    for face in 0..4 {
        let wire = model.faces[face].outer;
        for coedge in &mut model.loops[wire].coedges {
            if coedge.edge == seam {
                let mut curve = [0.17, 0.83];
                let mut pc = [0.17, 0.83];
                if coedge.reversed {
                    curve.reverse();
                    pc.reverse();
                }
                let mut lifts = [[0; 2]; 2];
                for i in 0..2 {
                    lifts[i][0] =
                        (coedge.pcurve.evaluate(pc[i]).unwrap().point[0] / 4.).floor() as i32;
                }
                *coedge = coedge
                    .clone()
                    .with_trim(brep_topology::CoedgeTrim {
                        curve_parameter: curve,
                        pcurve_parameter: pc,
                        periodic_lift: lifts,
                    })
                    .unwrap();
            }
        }
    }

    // Degree-elevate only the axial direction of one chart and perturb its
    // interior, leaving both shared circular arcs and the periodic seam exact.
    let exact = model.faces[0].surface.clone();
    let mut noisy = exact.clone();
    noisy.degree_v = 2;
    noisy.knots_v = vec![0., 0., 0., 1., 1., 1.];
    for row in 0..noisy.control_points.len() {
        let p0 = exact.control_points[row][0].clone();
        let p1 = exact.control_points[row][1].clone();
        let w0 = exact.weights[row][0];
        let w1 = exact.weights[row][1];
        let w = 0.5 * (w0 + w1);
        let mut middle: [f64; 3] =
            std::array::from_fn(|axis| (0.5 * w0 * p0[axis] + 0.5 * w1 * p1[axis]) / w);
        if row % 2 == 1 && row > 1 && row + 2 < noisy.control_points.len() {
            middle[0] += 1e-4;
            middle[1] -= 5e-5;
        }
        noisy.control_points[row] = vec![p0, middle.to_vec(), p1];
        noisy.weights[row] = vec![w0, w, w1];
    }
    noisy.validate().unwrap();
    model.faces[0].surface = noisy;
    model.validate().unwrap();
    (model, 0)
}

fn reverse_face_uv(model: &mut Model, face: usize) {
    let surface = &mut model.faces[face].surface;
    let low_u = surface.knots_u[surface.degree_u];
    let high_u = surface.knots_u[surface.knots_u.len() - 1 - surface.degree_u];
    let low_v = surface.knots_v[surface.degree_v];
    let high_v = surface.knots_v[surface.knots_v.len() - 1 - surface.degree_v];
    surface.control_points.reverse();
    surface.weights.reverse();
    for row in &mut surface.control_points {
        row.reverse();
    }
    for row in &mut surface.weights {
        row.reverse();
    }
    surface.knots_u = surface
        .knots_u
        .iter()
        .rev()
        .map(|knot| low_u + high_u - knot)
        .collect();
    surface.knots_v = surface
        .knots_v
        .iter()
        .rev()
        .map(|knot| low_v + high_v - knot)
        .collect();
    let wires: Vec<usize> = std::iter::once(model.faces[face].outer)
        .chain(model.faces[face].holes.iter().copied())
        .collect();
    for wire in wires {
        for coedge in &mut model.loops[wire].coedges {
            for point in &mut coedge.pcurve.control_points {
                point[0] = low_u + high_u - point[0];
                point[1] = low_v + high_v - point[1];
            }
        }
    }
}

#[test]
fn tweak_predicts_limit_and_rolls_back_excess_without_losing_ids() {
    let model = brep_core::cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let wall = face_at(&model, 0, 10.);
    let opposite = face_at(&model, 0, 0.);
    let axis = Axis {
        point: [10., 0., 0.],
        direction: [0., 1., 0.],
    };
    let draft = draft_angle(&model, wall, axis, &budget()).unwrap();
    assert!(draft.planar && draft.samples >= 12);
    let limit = draft
        .max_angle_positive_rad
        .or(draft.max_angle_negative_rad)
        .expect("cube has a geometric draft limit");
    let angle = if draft.max_angle_positive_rad.is_some() {
        0.05
    } else {
        -0.05
    };
    let before_hash = direct_edit_model_hash(&model);
    let report = tweak_face(&model, wall, axis, angle, &budget()).unwrap();
    assert!(report.audit.ok);
    assert!(report.max_angle_rad.is_some());
    assert_eq!(report.model.1.faces[opposite], model.1.faces[opposite]);
    // Polygon rebuilding may choose a different but equivalent UV basis.
    // Persistent identity and geometric placement are the contract here.
    assert_eq!(face_at(&report.model, 0, 0.), opposite);
    report.model.validate().unwrap();
    assert_eq!(direct_edit_model_hash(&model), before_hash);

    let excessive = if draft.max_angle_positive_rad.is_some() {
        limit + 1e-3
    } else {
        -limit - 1e-3
    };
    let error = tweak_face(&model, wall, axis, excessive, &budget()).unwrap_err();
    assert_eq!(error.code, TWEAK_LIMIT);
    assert!(error.message.contains(&limit.to_string()));
    assert_eq!(direct_edit_model_hash(&model), before_hash);
    assert_eq!(
        tweak_face(&model, wall, axis, f64::NAN, &budget())
            .unwrap_err()
            .code,
        TWEAK_INVALID
    );
}

#[test]
fn replace_with_inclined_plane_certifies_every_target_edge_and_reuses_ids() {
    let model = brep_core::cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let top = face_at(&model, 2, 10.);
    let bottom = face_at(&model, 2, 0.);
    let before_hash = direct_edit_model_hash(&model);
    let report = replace_face(&model, top, &plane_patch(10., -0.05, 10.), &budget()).unwrap();
    assert_eq!(report.target_kind, "plane");
    assert_eq!(report.correspondences.len(), 4);
    assert!(
        report
            .correspondences
            .iter()
            .all(|proof| proof.agreement_cells > 0)
    );
    assert!(report.max_surface_residual_mm <= model.tolerance_mm * 8.);
    assert!((report.volume_after_mm3 - 975.).abs() < 1e-6);
    assert_eq!(report.model.1, model.1);
    assert_eq!(report.model.faces[bottom], model.faces[bottom]);
    assert!(report.rebuilt_faces < report.model.faces.len());
    assert!(report.reused_faces > 0);
    assert_eq!(direct_edit_model_hash(&model), before_hash);
}

#[test]
fn replace_with_cylinder_certifies_arcs_rulings_and_reuses_ids() {
    let model = brep_core::cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let top = face_at(&model, 2, 10.);
    let bottom = face_at(&model, 2, 0.);
    let before_hash = direct_edit_model_hash(&model);
    let report = replace_face(&model, top, &cylinder_patch(), &budget()).unwrap();
    assert_eq!(report.target_kind, "cylinder");
    assert_eq!(report.retrimmed_vertices, 4);
    assert_eq!(report.correspondences.len(), 4);
    assert!(report.correspondences.iter().all(|proof| {
        proof.agreement_cells > 0 && proof.max_residual_mm <= model.tolerance_mm * 8.
    }));
    assert_eq!(report.model.1, model.1);
    assert_eq!(report.model.faces[bottom], model.faces[bottom]);
    report.model.validate().unwrap();
    assert!(
        brep_core::solid_audit::audit_solid(&report.model)
            .unwrap()
            .ok
    );
    assert_eq!(direct_edit_model_hash(&model), before_hash);

    let mut reversed = cylinder_patch();
    reversed.control_points.reverse();
    reversed.weights.reverse();
    for row in &mut reversed.control_points {
        row.reverse();
    }
    for row in &mut reversed.weights {
        row.reverse();
    }
    reversed.knots_u = vec![0., 0., 0., 1., 1., 1.];
    reversed.knots_v = vec![0., 0., 1., 1.];
    let reversed_report = replace_face(&model, top, &reversed, &budget()).unwrap();
    assert_eq!(reversed_report.correspondences.len(), 4);
    assert!(reversed_report.audit.ok);
    assert_eq!(reversed_report.model.1, model.1);

    // The exact quarter arc ending on the bottom plane collapses two slide
    // edges. It must be refused rather than quantized into a degenerate solid.
    let collapsed = replace_face(&model, top, &collapsed_cylinder_patch(), &budget()).unwrap_err();
    assert_eq!(collapsed.code, "BREP_SOLID_AUDIT_REFUSED");
    assert!(collapsed.message.contains("Degenerate sew edge"));
    assert_eq!(direct_edit_model_hash(&model), before_hash);
}

#[test]
fn replace_out_of_domain_names_edge_and_rolls_back() {
    let model = brep_core::cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let top = face_at(&model, 2, 10.);
    let before_hash = direct_edit_model_hash(&model);
    let error = replace_face(&model, top, &plane_patch(11., 0., 1.), &budget()).unwrap_err();
    assert_eq!(error.code, REPLACE_OUT_OF_DOMAIN);
    assert!(error.message.contains("Edge"));
    assert_eq!(direct_edit_model_hash(&model), before_hash);

    let mut short_axial_patch = cylinder_patch();
    for row in &mut short_axial_patch.control_points {
        row[0] = vec![6., row[0][1], row[0][2]];
        row[1] = vec![5., row[1][1], row[1][2]];
    }
    let error = replace_face(&model, top, &short_axial_patch, &budget()).unwrap_err();
    assert_eq!(error.code, REPLACE_OUT_OF_DOMAIN);
    assert!(error.message.contains("Edge"));
    assert_eq!(direct_edit_model_hash(&model), before_hash);
}

#[test]
fn delete_extend_closes_pocket_and_cap_returns_open_sheet() {
    let pocket = brep_core::boolean(
        &brep_core::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &brep_core::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let floor = face_at(&pocket, 2, 2.);
    assert_eq!(
        recommend_delete_face_strategy(&pocket, floor, &budget()).unwrap(),
        DeleteFaceStrategy::Extend
    );
    let before_hash = direct_edit_model_hash(&pocket);
    let extended = delete_face(
        &pocket,
        floor,
        DeleteFaceOptions::with_strategy(DeleteFaceStrategy::Extend),
        &budget(),
    )
    .unwrap();
    assert_eq!(extended.strategy, DeleteFaceStrategy::Extend);
    assert_eq!(extended.boundary_edge_count, 0);
    assert!(extended.audit.as_ref().unwrap().ok);
    assert!(!extended.model.1.faces.is_empty());
    assert_eq!(direct_edit_model_hash(&pocket), before_hash);

    let cube = brep_core::cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let top = face_at(&cube, 2, 10.);
    let retained_id = cube.1.faces[face_at(&cube, 2, 0.)];
    let capped = delete_face(
        &cube,
        top,
        DeleteFaceOptions::with_strategy(DeleteFaceStrategy::Cap),
        &budget(),
    )
    .unwrap();
    assert!(capped.model.bodies.is_empty());
    assert!(!capped.model.shells[0].closed);
    assert_eq!(capped.boundary_edge_count, 4);
    assert!(capped.model.1.faces.contains(&retained_id));
}

#[test]
fn delete_patch_is_planar_only_and_extend_refuses_divergence_atomically() {
    let cube = brep_core::cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let top = face_at(&cube, 2, 10.);
    let before_hash = direct_edit_model_hash(&cube);
    let patched = delete_face(
        &cube,
        top,
        DeleteFaceOptions::with_strategy(DeleteFaceStrategy::Patch),
        &budget(),
    )
    .unwrap();
    assert!(patched.max_g0_gap_mm.unwrap() <= cube.tolerance_mm * 8.);
    assert_eq!(patched.model.1, cube.1);

    let error = delete_face(
        &cube,
        top,
        DeleteFaceOptions::with_strategy(DeleteFaceStrategy::Extend),
        &budget(),
    )
    .unwrap_err();
    assert_eq!(error.code, DELETE_DIVERGENT);
    assert_eq!(direct_edit_model_hash(&cube), before_hash);
}

#[test]
fn snap_plane_honors_tolerance_and_rolls_back_no_fit() {
    let mut model = brep_core::cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let top = face_at(&model, 2, 10.);
    noisy_plane(&mut model, top, 1e-4);
    model.validate().unwrap();
    let before_hash = direct_edit_model_hash(&model);
    let snapped = snap_face_to_analytic(&model, top, 1e-3, &budget()).unwrap();
    assert_eq!(snapped.target_kind, "plane");
    assert!(snapped.analytic_faces_after > snapped.analytic_faces_before);
    assert_eq!(snapped.model.1, model.1);
    assert_eq!(
        snapped.watertight_after,
        brep_core::watertight::Closure::WithinTolerance
    );

    let error = snap_face_to_analytic(&model, top, 1e-7, &budget()).unwrap_err();
    assert_eq!(error.code, SNAP_NO_FIT);
    assert_eq!(direct_edit_model_hash(&model), before_hash);
}

#[test]
fn snap_cylinder_preserves_weighted_arcs_boundaries_and_ids() {
    for reversed in [false, true] {
        let mut model = brep_core::cylinder(2., 5.).unwrap();
        let side = model
            .faces
            .iter()
            .position(|face| face.surface.degree_u == 2 && face.surface.degree_v == 1)
            .unwrap();
        if reversed {
            reverse_face_uv(&mut model, side);
        }
        noisy_cylinder(&mut model, side, 1e-4);
        let noisy = model.faces[side].surface.clone();
        model.validate().unwrap();
        let before_hash = direct_edit_model_hash(&model);
        let snapped = snap_face_to_analytic(&model, side, 1e-3, &budget()).unwrap();
        assert_eq!(snapped.target_kind, "cylinder");
        assert!(snapped.max_deviation_mm <= 1e-3);
        assert!(snapped.max_boundary_deviation_mm <= model.tolerance_mm);
        assert_eq!(snapped.model.1, model.1);
        let surface = &snapped.model.faces[side].surface;
        assert_eq!(surface.degree_u, noisy.degree_u);
        assert_eq!(surface.degree_v, 1);
        for row in 0..noisy.control_points.len() {
            assert_eq!(
                surface.weights[row],
                [noisy.weights[row][0], noisy.weights[row][2]]
            );
            assert_eq!(
                surface.control_points[row],
                [
                    noisy.control_points[row][0].clone(),
                    noisy.control_points[row][2].clone()
                ]
            );
        }
        assert!(surface.weights.iter().flatten().all(|w| *w > 0.));
        snapped.model.validate().unwrap();
        assert!(snapped.audit.ok);

        let error = snap_face_to_analytic(&model, side, 1e-7, &budget()).unwrap_err();
        assert_eq!(error.code, SNAP_NO_FIT);
        assert_eq!(direct_edit_model_hash(&model), before_hash);
    }

    let mut invalid = brep_core::cylinder(2., 5.).unwrap();
    let side = 0;
    noisy_cylinder(&mut invalid, side, 0.05);
    invalid.validate().unwrap();
    let before_hash = direct_edit_model_hash(&invalid);
    let error = snap_face_to_analytic(&invalid, side, 1e-3, &budget()).unwrap_err();
    assert_eq!(error.code, SNAP_NO_FIT);
    assert_eq!(direct_edit_model_hash(&invalid), before_hash);
}

#[test]
fn snap_cylinder_preserves_periodic_seam_charts() {
    let (model, side) = noisy_periodic_cylinder();
    let before_hash = direct_edit_model_hash(&model);
    let snapped = snap_face_to_analytic(&model, side, 1e-3, &budget()).unwrap();
    assert_eq!(snapped.target_kind, "cylinder");
    assert!(snapped.model.faces[side].surface.periodic_u);
    assert_eq!(snapped.model.faces[side].surface.degree_v, 1);
    assert!(snapped.max_boundary_deviation_mm <= model.tolerance_mm);
    assert_eq!(snapped.model.1, model.1);
    snapped.model.validate().unwrap();
    assert!(snapped.audit.ok);
    assert_eq!(direct_edit_model_hash(&model), before_hash);
}

#[test]
fn incremental_rebuild_refuses_id_drift_and_transaction_rollback_is_exact() {
    let model = brep_core::cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let top = face_at(&model, 2, 10.);
    let replaced = replace_face(&model, top, &plane_patch(10., -0.05, 10.), &budget()).unwrap();
    let incremental =
        complete_incremental_rebuild(&model, replaced.model, &[top], &budget()).unwrap();
    assert!(incremental.reused_faces > 0);
    assert!(incremental.validation.publishable());
    assert_eq!(incremental.model.1, model.1);
    if !incremental.validation.self_intersection_proven {
        assert!(incremental.validation.full_validation_fallback);
        assert!(incremental.validation.full_audit.as_ref().unwrap().ok);
    }

    let mut drifted = model.clone();
    drifted.1.faces.swap(0, 1);
    let error = complete_incremental_rebuild(&model, drifted, &[top], &budget()).unwrap_err();
    assert_eq!(error.code, DIRTY_INVALID);

    let before_hash = direct_edit_model_hash(&model);
    let mut invalid = model.clone();
    invalid.shells[0].faces.clear();
    let mut transaction = DirectEditTransaction::stage(&model, invalid);
    assert!(transaction.validate(&[top], &budget()).is_err());
    let rolled_back = transaction.rollback().unwrap();
    assert_eq!(direct_edit_model_hash(&rolled_back), before_hash);
    assert_eq!(rolled_back, model);
}
