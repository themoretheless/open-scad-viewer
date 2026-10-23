use brep_core::{
    Model, prism,
    prism_frame::{Frame, localize},
};
fn pose(model: &Model, angles: [f64; 3], translation: [f64; 3]) -> Model {
    let rotate = |p: [f64; 3]| {
        let (sx, cx) = angles[0].sin_cos();
        let (sy, cy) = angles[1].sin_cos();
        let (sz, cz) = angles[2].sin_cos();
        let p = [p[0], cx * p[1] - sx * p[2], sx * p[1] + cx * p[2]];
        let p = [cy * p[0] + sy * p[2], p[1], -sy * p[0] + cy * p[2]];
        [
            cz * p[0] - sz * p[1] + translation[0],
            sz * p[0] + cz * p[1] + translation[1],
            p[2] + translation[2],
        ]
    };
    let mut result = model.clone();
    for v in &mut result.vertices {
        v.point = rotate(v.point);
    }
    for e in &mut result.edges {
        for p in &mut e.curve.control_points {
            *p = rotate([p[0], p[1], p[2]]).to_vec();
        }
    }
    for f in &mut result.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            *p = rotate([p[0], p[1], p[2]]).to_vec();
        }
    }
    result.validate().unwrap();
    result
}
fn compare(a: &Model, b: &Model, tol: f64) {
    assert_eq!(a.1.faces, b.1.faces);
    assert_eq!(a.1.edges, b.1.edges);
    assert_eq!(a.1.lineage, b.1.lineage);
    for (x, y) in a.vertices.iter().zip(&b.vertices) {
        for i in 0..3 {
            assert!((x.point[i] - y.point[i]).abs() < tol);
        }
    }
    for (x, y) in a.faces.iter().zip(&b.faces) {
        for (p, q) in x
            .surface
            .control_points
            .iter()
            .flatten()
            .zip(y.surface.control_points.iter().flatten())
        {
            for i in 0..3 {
                assert!((p[i] - q[i]).abs() < tol);
            }
        }
    }
}
#[test]
fn arbitrary_axis_prisms_localize_and_restore_with_inherited_ids() {
    let a = brep_core::cylinder(3., 5.).unwrap();
    let b = brep_core::cuboid([0., -2., 0.], [4., 2., 5.]).unwrap();
    for angles in [
        [0., 0., 0.],
        [0.2, 0.7, 0.4],
        [1.1, -0.4, 2.1],
        [0., std::f64::consts::FRAC_PI_2, 0.],
    ] {
        let a = pose(&a, angles, [7., -9., 11.]);
        let b = pose(&b, angles, [7., -9., 11.]);
        let (la, lb, frame) = localize(&a, &b)
            .unwrap()
            .unwrap_or_else(|| panic!("Failed pose {angles:?}"));
        assert!(prism::recognize(&la).unwrap().is_some());
        assert!(prism::recognize(&lb).unwrap().is_some());
        assert!(frame.max_adjustment_mm <= frame.roundoff_bound_mm);
        compare(&a, &frame.restore(&la).unwrap(), 1e-10);
        compare(&b, &frame.restore(&lb).unwrap(), 1e-10);
        let (_, _, reordered) = localize(&b, &a).unwrap().unwrap();
        assert_eq!(frame.axes, reordered.axes);
        assert_eq!(frame.origin, reordered.origin);
    }
}
#[test]
fn holes_and_disconnected_prismatic_operands_share_rigid_frame() {
    let a = pose(
        &brep_core::tube(3., 1., 5.).unwrap(),
        [0.7, 1.2, -0.4],
        [-3., 7., 9.],
    );
    let source = prism::recognize(&brep_core::cylinder(2., 8.).unwrap())
        .unwrap()
        .unwrap();
    let mut profiles = source.loops.clone();
    let mut other = source.loops[0].clone();
    for curve in &mut other {
        for p in &mut curve.control_points {
            p[0] += 6.;
        }
    }
    profiles.push(other);
    let disconnected = prism::extrude(&profiles, 0., 8.).unwrap();
    let b = pose(&disconnected, [0.7, 1.2, -0.4], [-3., 7., 9.]);
    let (la, lb, frame) = localize(&a, &b).unwrap().unwrap();
    assert_eq!(prism::recognize(&la).unwrap().unwrap().loops.len(), 2);
    assert_eq!(prism::recognize(&lb).unwrap().unwrap().loops.len(), 2);
    assert_eq!(lb.bodies.len(), 2);
    assert!(frame.adjustment_count > 0);
    frame.restore(&la).unwrap().validate().unwrap();
}
#[test]
fn nonparallel_prisms_tapers_and_authored_distortions_above_bound_are_refused() {
    let cylinder = brep_core::cylinder(3., 5.).unwrap();
    let a = pose(&cylinder, [0.2, 0.7, 0.4], [7., -9., 11.]);
    let tilted = pose(&cylinder, [0.20001, 0.7, 0.4], [7., -9., 11.]);
    assert!(localize(&a, &tilted).unwrap().is_none());
    for other in [
        brep_core::frustum(3., 2., 5.).unwrap(),
        brep_core::sphere(3.).unwrap(),
    ] {
        assert!(
            localize(&a, &pose(&other, [0.2, 0.7, 0.4], [7., -9., 11.]))
                .unwrap()
                .is_none()
        );
    }
    let mut distorted = a.clone();
    distorted.tolerance_mm = 1e-3;
    distorted.faces[0].surface.control_points[1][1][0] += 1e-6;
    distorted.validate().unwrap();
    assert!(localize(&a, &distorted).unwrap().is_none());
}
#[test]
fn malformed_frames_and_empty_operands_are_explicit() {
    let cube = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut frame = Frame {
        origin: [0.; 3],
        axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        roundoff_bound_mm: 0.,
        max_adjustment_mm: 0.,
        adjustment_count: 0,
    };
    frame.axes[0][0] = 2.;
    assert_eq!(frame.restore(&cube).unwrap_err().code, "BREP_INVALID_FRAME");
    assert!(
        localize(&cube, &Model::empty(1e-7).unwrap())
            .unwrap()
            .is_none()
    );
}
