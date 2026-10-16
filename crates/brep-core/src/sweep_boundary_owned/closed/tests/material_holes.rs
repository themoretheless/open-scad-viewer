//! Hollow and oriented original material cases.
use super::*;
fn hollow_material() -> (LoftCap, Vec<Curve>) {
    let (mut material, mut profiles) = material(false, true);
    let uv = [[-0.5, 3.], [1.5, 3.], [1.5, 5.], [-0.5, 5.]];
    let world = |p: [f64; 2]| {
        let u = (p[0] + 1.) / 4.;
        let v = (p[1] - 2.) / 4.;
        let r = 1. / 128.;
        [u * v / 65536., -r + 2. * r * u, -r + 2. * r * v]
    };
    let mut inner = Vec::new();
    for i in (0..4).rev() {
        let a = uv[(i + 1) % 4];
        let b = uv[i];
        profiles.push(nurbs_core::primitives::line(world(a), world(b)).unwrap());
        inner.push(Curve {
            degree: 1,
            knots: vec![7., 7., 11., 11.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        });
    }
    material.trims.push(inner);
    (material, profiles)
}
fn qualify_hollow_case(mode: usize, spacing: Spacing) {
    let (material, profiles) = hollow_material();
    let path = path(false, mode != 0);
    let axis = Curve {
        degree: 8,
        knots: [vec![2.; 9], vec![6.; 9]].concat(),
        control_points: path
            .control_points
            .windows(2)
            .map(|p| (0..3).map(|k| 9. * (p[1][k] - p[0][k])).collect())
            .collect(),
        weights: vec![1.; 9],
        periodic: false,
    };
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] += 1.;
    }
    let normal = constant_vector_law([0., 0., 1.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let mut o = options(false, spacing);
    if mode == 1 {
        o.orientation = Orientation::Fixed;
    }
    let mut sweep = MultiSweep::new(&profiles, &path, &scale, &twist, o).unwrap();
    if mode == 1 {
        sweep = sweep.with_frame_laws(&axis, &normal).unwrap();
    }
    if mode == 2 {
        sweep = sweep.with_orientation_guide(&guide).unwrap();
    }
    let sections = if mode == 2 || matches!(spacing, Spacing::ArcLength { .. }) {
        129
    } else {
        33
    };
    let attempt = construct_closed_with_ideal_atlas(
        &sweep,
        &[4, 4],
        sections,
        &material,
        1e-8,
        0.05,
        limits(),
    )
    .unwrap();
    let body = match attempt {
        ClosedAttempt::Certified(body) => body,
        ClosedAttempt::Refused(r) => {
            eprintln!("hollow mode={mode}/{spacing:?}");
            assert_body(ClosedAttempt::Refused(r))
        }
    };
    let r = body.report();
    assert!(r.certified && r.material.certified && r.continuous_bound);
    assert_eq!(
        r.material.trim.as_ref().unwrap().winding,
        vec![Some(1), Some(-1)]
    );
    assert_eq!(r.material.bound_spans, 8);
    assert!(
        r.ideal_volume.as_ref().unwrap().embedding_certified && r.volume.as_ref().unwrap().proven
    );
    assert_eq!(body.model().faces.len(), 8 * (sections - 1));
    assert_eq!(
        r.retained_walls.as_ref().unwrap().inspected_faces,
        body.model().faces.len()
    );
    assert_eq!(body.model().shells.len(), 2);
    assert_eq!(body.model().bodies.len(), 1);
    assert_eq!(body.model().bodies[0].inner_shells.len(), 1);
    eprintln!(
        "owned hollow mode={mode}/{spacing:?} E={:?} source={}",
        r.boundary_error_upper, r.source_cells
    );
}
#[test]
fn closed_owned_hollow_fixed_parameter() {
    qualify_hollow_case(0, Spacing::Parameter);
}
#[test]
fn closed_owned_hollow_fixed_arc() {
    qualify_hollow_case(
        0,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    );
}
#[test]
fn closed_owned_hollow_authored_parameter() {
    qualify_hollow_case(1, Spacing::Parameter);
}
#[test]
fn closed_owned_hollow_authored_arc() {
    qualify_hollow_case(
        1,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    );
}
#[test]
fn closed_owned_hollow_guided_parameter() {
    qualify_hollow_case(2, Spacing::Parameter);
}
#[test]
fn closed_owned_hollow_guided_arc() {
    qualify_hollow_case(
        2,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    );
}
#[test]
fn closed_original_material_reverse_roles_and_wrong_hole_orientation() {
    let (material, profiles) = hollow_material();
    let path = path(false, false);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let mut reverse = material.clone();
    for ring in &mut reverse.trims {
        *ring = ring.iter().rev().map(|c| c.reverse().unwrap()).collect();
    }
    let reverse_profiles = profiles
        .chunks(4)
        .flat_map(|ring| ring.iter().rev().map(|c| c.reverse().unwrap()))
        .collect::<Vec<_>>();
    let sweep = MultiSweep::new(
        &reverse_profiles,
        &path,
        &scale,
        &twist,
        options(false, Spacing::Parameter),
    )
    .unwrap();
    let ClosedAttempt::Certified(body) =
        construct_closed_with_ideal_atlas(&sweep, &[4, 4], 33, &reverse, 1e-8, 0.05, limits())
            .unwrap()
    else {
        panic!("reversed original material roles must preserve the Solid");
    };
    assert_eq!(
        body.report().material.trim.as_ref().unwrap().winding,
        vec![Some(-1), Some(1)]
    );
    assert!(body.report().volume.as_ref().unwrap().proven);
    let mut wrong = material.clone();
    wrong.trims[1] = wrong.trims[1]
        .iter()
        .rev()
        .map(|c| c.reverse().unwrap())
        .collect();
    let wrong_profiles = profiles[..4]
        .iter()
        .cloned()
        .chain(profiles[4..].iter().rev().map(|c| c.reverse().unwrap()))
        .collect::<Vec<_>>();
    let sweep = MultiSweep::new(
        &wrong_profiles,
        &path,
        &scale,
        &twist,
        options(false, Spacing::Parameter),
    )
    .unwrap();
    let ClosedAttempt::Refused(r) =
        construct_closed_with_ideal_atlas(&sweep, &[4, 4], 33, &wrong, 1e-8, 0.05, limits())
            .unwrap()
    else {
        panic!("same-winding hole became a Solid");
    };
    assert!(!r.material.certified && r.ideal_volume.is_none());
}

#[test]
fn closed_original_multispan_world_and_uv_chains_keep_exact_material_ownership() {
    let (mut material, curves) = material(false, true);
    let mut uv = material.trims[0]
        .iter()
        .map(|c| c.control_points[0].clone())
        .collect::<Vec<_>>();
    uv.push(uv[0].clone());
    material.trims = vec![vec![Curve {
        degree: 1,
        knots: vec![7., 7., 8., 9., 10., 11., 11.],
        control_points: uv,
        weights: vec![1.; 5],
        periodic: false,
    }]];
    let mut points = curves
        .iter()
        .map(|c| c.control_points[0].clone())
        .collect::<Vec<_>>();
    points.push(points[0].clone());
    let profiles = vec![Curve {
        degree: 1,
        knots: vec![2., 2., 3., 4., 5., 6., 6.],
        control_points: points,
        weights: vec![1.; 5],
        periodic: false,
    }];
    let original = (profiles.clone(), material.trims.clone());
    let path = path(false, true);
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        options(true, Spacing::Parameter),
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let body = assert_body(
        construct_closed_with_ideal_atlas(&sweep, &[1], 33, &material, 1e-8, 0.05, limits())
            .unwrap(),
    );
    assert_eq!(body.report().material.bound_spans, 4);
    assert_eq!(body.report().material.profiles.len(), 1);
    assert_eq!((profiles.clone(), material.trims.clone()), original);
}
