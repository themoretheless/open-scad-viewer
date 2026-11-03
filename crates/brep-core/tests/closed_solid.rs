use brep_core::{
    closed_solid as solid,
    healing_report::{AutoRepairOperation, HealingStatus},
};
fn policy() -> solid::AdmissionPolicy {
    solid::AdmissionPolicy {
        tolerance_uv: 1e-8,
        max_linear_cells: 100000,
        geometry: brep_core::volume_validity::Limits {
            boundary: brep_core::boundary_embedding::Limits {
                exact_work: 1000000,
                trim_pairs: 1000,
                trim_cells: 10000,
                trim_domain_cells: 100000,
                spans: 100,
                contacts: brep_core::face_contacts::Limits {
                    pairs: 1000,
                    cells: 10000,
                    domain_cells: 100000,
                    cells_per_pair: 16,
                    domain_cells_per_pair: 1000,
                },
            },
            nesting_pairs: 100,
            nesting_cells: 10000,
            nesting_domain_cells: 100000,
            orientation_cells: 10000,
            orientation_domain_cells: 100000,
            orientation_spans: 100,
        },
    }
}
#[test]
fn continuous_geometry_admission_owns_point_material_classification() {
    let model = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let body = solid::qualify(model.clone(), policy())
        .unwrap()
        .into_solid()
        .unwrap();
    body.reverify().unwrap();
    let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419]];
    assert_eq!(
        body.classify_point([1.; 3], &directions, 10000, 100000)
            .unwrap()
            .parity,
        Some(true)
    );
    assert_eq!(
        body.classify_point([3.; 3], &directions, 10000, 100000)
            .unwrap()
            .parity,
        Some(false)
    );
    let mut inward = model.clone();
    for use_ in &mut inward.shells[0].faces {
        use_.reversed = !use_.reversed;
    }
    assert!(solid::qualify(inward, policy()).unwrap().solid().is_none());
    let mut open = model;
    open.shells[0].closed = false;
    assert!(solid::qualify(open, policy()).is_err());
}
#[test]
fn a_successful_tolerance_repair_cannot_publish_geometrically_unproved_body() {
    let model = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let before = model.clone();
    let cap = model
        .tolerance_context()
        .unwrap()
        .spatial_bounds()
        .absolute_mm;
    let mut curve = model.edges[0].curve.clone();
    curve.control_points[1][2] += cap * 0.125;
    let outcome = solid::heal_and_qualify(
        &model,
        &[AutoRepairOperation::RefitEdge {
            edge: 0,
            replacement: curve,
        }],
        policy(),
    );
    assert_eq!(outcome.repair_attempt().status, HealingStatus::Complete);
    assert_eq!(
        outcome.status(),
        solid::GeometricHealingStatus::GeometryUnproved
    );
    assert!(outcome.solid().is_none());
    assert_eq!(outcome.model(), &before);
    assert_eq!(model, before);
}
#[test]
fn restoring_source_edge_geometry_is_readmitted_after_real_healing() {
    let source = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let mut damaged = source.clone();
    let cap = source
        .tolerance_context()
        .unwrap()
        .spatial_bounds()
        .absolute_mm;
    damaged.edges[0].curve.control_points[1][2] += cap * 0.125;
    let before = damaged.clone();
    let outcome = solid::heal_and_qualify(
        &damaged,
        &[AutoRepairOperation::RefitEdge {
            edge: 0,
            replacement: source.edges[0].curve.clone(),
        }],
        policy(),
    );
    assert_eq!(outcome.status(), solid::GeometricHealingStatus::Accepted);
    assert!(outcome.solid().is_some());
    assert!(outcome.geometry().unwrap().proven);
    assert_eq!(damaged, before);
}
#[test]
fn nested_shell_roles_and_material_orientation_are_rebuilt_and_reaudited() {
    let outer = brep_core::cuboid([0.; 3], [4.; 3]).unwrap();
    let inner = brep_core::cuboid([1.; 3], [3.; 3]).unwrap();
    let good = brep_core::minkowski3d::curved::certify_enclosed_difference(&outer, &inner, 0)
        .unwrap()
        .into_model();
    let mut wrong = good.clone();
    wrong.bodies[0].inner_shells.clear();
    wrong.bodies.push(brep_core::Body {
        outer_shell: 1,
        inner_shells: vec![],
    });
    for face in &mut wrong.shells[0].faces {
        face.reversed = !face.reversed;
    }
    wrong.rebuild_topology_ids();
    let before = wrong.clone();
    let p = solid::ShellRepairPolicy {
        establish_forest: policy(),
        establish_orientation: policy(),
        readmit: policy(),
    };
    let result = solid::rebuild_even_odd_shells(&wrong, p);
    assert!(result.solid().is_some(), "{:?}", result.refusal());
    assert_eq!(result.parents().unwrap(), [None, Some(0)]);
    assert_eq!(result.flipped_shells(), [0]);
    assert_eq!(result.model().bodies[0].inner_shells, [1]);
    assert_eq!(result.model().faces, good.faces);
    assert_eq!(result.model().edges, good.edges);
    assert_eq!(result.model().vertices, good.vertices);
    let directions = [[1., 0.317, 0.173]];
    let body = result.solid().unwrap();
    assert_eq!(
        body.classify_point([0.5; 3], &directions, 10000, 100000)
            .unwrap()
            .parity,
        Some(true)
    );
    assert_eq!(
        body.classify_point([2.; 3], &directions, 10000, 100000)
            .unwrap()
            .parity,
        Some(false)
    );
    assert_eq!(wrong, before);
    let mut exhausted = p;
    exhausted.readmit.geometry.boundary.exact_work = 1;
    let rejected = solid::rebuild_even_odd_shells(&wrong, exhausted);
    assert!(rejected.solid().is_none());
    assert_eq!(rejected.model(), &before);
}
#[test]
fn rational_surface_material_seed_is_tied_to_the_original_point_not_its_rounding() {
    use nurbs_core::surface::Surface;
    let model = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
    let body = solid::qualify(model, policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let mut source = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0.25, 0.25, 1.], vec![0.25, 1.75, 1.]],
            vec![vec![1.75, 0.25, 1.], vec![1.75, 1.75, 1.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 5.]],
        periodic_u: false,
        periodic_v: false,
    };
    let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]];
    let seed = solid::qualify_surface_material_seed(
        &body,
        &source,
        [0.5; 2],
        &directions,
        Default::default(),
    )
    .unwrap();
    assert!(seed.inside());
    seed.reverify().unwrap();
    for row in &mut source.control_points {
        for p in row {
            p[2] = 3.;
        }
    }
    assert!(
        !solid::qualify_surface_material_seed(
            &body,
            &source,
            [0.5; 2],
            &directions,
            Default::default()
        )
        .unwrap()
        .inside()
    );
    for row in &mut source.control_points {
        for p in row {
            p[2] = 2.;
        }
    }
    assert!(
        solid::qualify_surface_material_seed(
            &body,
            &source,
            [0.5; 2],
            &directions,
            Default::default()
        )
        .is_err()
    );
    assert!(
        solid::qualify_surface_material_seed(
            &body,
            &source,
            [1.5; 2],
            &directions,
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn all_boolean_operations_select_original_face_material_and_subtraction_orientation() {
    use solid::{BooleanMaterialOp as Op, BooleanSourceSide as Side};
    let a = solid::qualify(brep_core::cuboid([0.; 3], [2.; 3]).unwrap(), policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let b = solid::qualify(
        brep_core::cuboid([1., -1., -1.], [3.; 3]).unwrap(),
        policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap();
    let face = |x: f64| {
        a.model()
            .faces
            .iter()
            .position(|f| f.surface.control_points.iter().flatten().all(|p| p[0] == x))
            .unwrap()
    };
    let directions = [[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]];
    for (op, inner_keep, inner_reverse, outer_keep) in [
        (Op::Union, false, false, true),
        (Op::Intersection, true, false, false),
        (Op::Difference, false, false, true),
        (Op::Xor, true, true, true),
    ] {
        let inside = solid::qualify_boolean_face_seed(
            &a,
            &b,
            op,
            Side::First,
            face(2.),
            [0.5; 2],
            &directions,
            Default::default(),
        )
        .unwrap();
        assert_eq!(inside.retain(), inner_keep);
        assert_eq!(inside.reverse_material_normal(), inner_reverse);
        let outside = solid::qualify_boolean_face_seed(
            &a,
            &b,
            op,
            Side::First,
            face(0.),
            [0.5; 2],
            &directions,
            Default::default(),
        )
        .unwrap();
        assert_eq!(outside.retain(), outer_keep);
        assert!(!outside.reverse_material_normal());
    }
    let bf = b
        .model()
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[0] == 1.)
        })
        .unwrap();
    let subtract = solid::qualify_boolean_face_seed(
        &a,
        &b,
        Op::Difference,
        Side::Second,
        bf,
        [0.5; 2],
        &directions,
        Default::default(),
    )
    .unwrap();
    assert!(subtract.retain() && subtract.reverse_material_normal());
    subtract.reverify().unwrap();
    assert!(
        solid::qualify_boolean_face_seed(
            &a,
            &b,
            Op::Union,
            Side::First,
            face(2.),
            [0.; 2],
            &directions,
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn independent_retained_flux_includes_cavities_and_curved_face_volume() {
    let outer = brep_core::cuboid([0.; 3], [4.; 3]).unwrap();
    let inner = brep_core::cuboid([1.; 3], [3.; 3]).unwrap();
    let cavity = brep_core::minkowski3d::curved::certify_enclosed_difference(&outer, &inner, 0)
        .unwrap()
        .into_model();
    let limits = solid::VolumeLimits {
        axis: 2,
        origin: 0.,
        absolute_error: 1e-6,
        cells: 100000,
        spans: 100000,
        domain_cells: 100000,algebra:Default::default(),
    };
    let body = solid::qualify(cavity, policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let bounds = body.volume_bounds(limits).unwrap().positive_volume.unwrap();
    assert!(bounds[0] <= 56. && 56. <= bounds[1]);
    let mut curved = outer;
    let f = curved
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 4.)
        })
        .unwrap();
    let original = curved.faces[f].surface.clone();
    let mut points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    original
                        .evaluate(i as f64 / 2., j as f64 / 2.)
                        .unwrap()
                        .point.to_vec()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    points[1][1][2] += 0.125;
    curved.faces[f].surface = nurbs_core::surface::Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: points,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let body = solid::qualify(curved, policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let report = body.volume_bounds(limits).unwrap();
    let bounds = report
        .positive_volume
        .unwrap_or_else(|| panic!("{} {:?}", report.reason, report.signed_bounds));
    let expected = 64. + 2. / 9.;
    assert!(bounds[0] <= expected && expected <= bounds[1], "{bounds:?}");
    let exhausted = body
        .volume_bounds(solid::VolumeLimits { cells: 1, ..limits })
        .unwrap();
    assert!(exhausted.positive_volume.is_none());
    assert!(exhausted.signed_bounds.is_none());
}
