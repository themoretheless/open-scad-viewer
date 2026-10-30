use super::*;
fn rect(a: [f64; 2], b: [f64; 2]) -> Vec<Curve> {
    crate::sketch::polygon_wire(vec![a, [b[0], a[1]], b, [a[0], b[1]]]).unwrap()
}
fn reverse(w: Vec<Curve>) -> Vec<Curve> {
    w.into_iter().rev().map(|c| c.reverse().unwrap()).collect()
}
#[test]
fn exact_nested_profile_roundtrip_and_claim_tampering() {
    let loops = vec![
        rect([0., 0.], [10., 10.]),
        reverse(rect([2., 2.], [8., 8.])),
        rect([4., 4.], [6., 6.]),
    ];
    let c = certify(&loops, 1e-7).unwrap();
    assert_eq!(c.nesting_depth, vec![0, 1, 2]);
    assert_eq!(c.orientation, vec![1, -1, 1]);
    let v = verify(&loops, 1e-7, &c).unwrap();
    assert_eq!(v.spans(), 12);
    let mut bad = c.clone();
    bad.version += 1;
    assert!(verify(&loops, 1e-7, &bad).is_err());
    let mut bad = c.clone();
    bad.source.push('0');
    assert!(verify(&loops, 1e-7, &bad).is_err());
    let mut bad = c.clone();
    bad.monotone_axes[0] = 2;
    assert!(verify(&loops, 1e-7, &bad).is_err());
    let mut bad = c.clone();
    bad.separating_normals.pop();
    assert!(verify(&loops, 1e-7, &bad).is_err());
    let mut bad = c.clone();
    bad.separating_normals[0] = 65000;
    assert!(verify(&loops, 1e-7, &bad).is_err());
    let mut bad = c.clone();
    bad.orientation[0] = -1;
    assert!(verify(&loops, 1e-7, &bad).is_err());
    let mut bad = c.clone();
    bad.nesting_depth[1] = 0;
    assert!(verify(&loops, 1e-7, &bad).is_err());
    let mut bad = c.clone();
    bad.subdivision_depth = 255;
    assert!(verify(&loops, 1e-7, &bad).is_err());
    #[cfg(feature = "codec")]
    {
        let json = value_codec::to_string(&c).unwrap();
        let decoded: ProfileCertificate = value_codec::from_str(&json).unwrap();
        assert_eq!(c, decoded);
        verify(&loops, 1e-7, &decoded).unwrap();
        let mut value = value_codec::to_value(&c).unwrap();
        value["solidValid"] = value_codec::Value::Bool(true);
        assert!(value_codec::from_value::<ProfileCertificate>(value).is_err());
    }
}
#[test]
fn source_binding_includes_weights_knots_tolerance_signed_zero_and_layout() {
    let loops = vec![rect([0., 0.], [10., 10.])];
    let c = certify(&loops, 1e-7).unwrap();
    let mut changed = loops.clone();
    changed[0][0].weights[0] = 2.;
    assert!(verify(&changed, 1e-7, &c).is_err());
    let mut changed = loops.clone();
    changed[0][0].knots[2] = 2.;
    changed[0][0].knots[3] = 2.;
    assert!(verify(&changed, 1e-7, &c).is_err());
    let mut changed = loops.clone();
    changed[0][0].control_points[0][0] = -0.;
    assert!(verify(&changed, 1e-7, &c).is_err());
    let mut changed = loops.clone();
    changed[0].rotate_left(1);
    assert!(verify(&changed, 1e-7, &c).is_err());
    assert!(verify(&loops, 1e-6, &c).is_err());
}
#[test]
fn rational_circle_and_affine_ellipse_retain_original_definitions() {
    let mut loops = vec![crate::sketch::circle_wire(2.).unwrap()];
    for c in &mut loops[0] {
        for p in &mut c.control_points {
            p[0] = 3. * p[0] + p[1];
            p[1] *= 2.;
        }
    }
    let before = loops.clone();
    let c = certify(&loops, 1e-7).unwrap();
    verify(&loops, 1e-7, &c).unwrap();
    assert_eq!(loops, before);
}
#[test]
fn empty_material_has_a_complete_empty_certificate() {
    let c = certify(&[], 1e-7).unwrap();
    assert_eq!(verify(&[], 1e-7, &c).unwrap().pieces(), 0);
    assert!(verify(&[rect([0., 0.], [1., 1.])], 1e-7, &c).is_err());
}
#[test]
fn crossing_overlap_open_wire_and_wrong_winding_never_certify() {
    let bow =
        crate::sketch::polygon_wire(vec![[0., 0.], [4., 4.], [0., 4.], [4., 0.]]).unwrap();
    assert!(certify(&[bow], 1e-7).is_err());
    assert!(certify(&[rect([0., 0.], [4., 4.]), rect([2., 2.], [6., 6.])], 1e-7).is_err());
    let mut open = rect([0., 0.], [4., 4.]);
    open[0].control_points[0][0] = 1.;
    assert!(certify(&[open], 1e-7).is_err());
    assert!(certify(&[reverse(rect([0., 0.], [4., 4.]))], 1e-7).is_err());
}
#[test]
fn same_side_curved_cusp_has_an_algebraic_certificate_in_both_frames() {
    let cubic = |points: Vec<Vec<f64>>| Curve {
        degree: 3,
        control_points: points,
        weights: vec![1.; 4],
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        periodic: false,
    };
    let a = cubic(vec![vec![0., 0.], vec![2., 0.], vec![4., 2.], vec![6., 6.]]);
    let b = cubic(vec![
        vec![0., 0.],
        vec![2., 0.],
        vec![6., 2.],
        vec![12., 12.],
    ]);
    for transformed in [false, true] {
        let mut wire = vec![
            b.clone(),
            Curve::from_polyline(vec![vec![12., 12.], vec![6., 6.]]).unwrap(),
            a.reverse().unwrap(),
        ];
        if transformed {
            for c in &mut wire {
                for p in &mut c.control_points {
                    let (x, y) = (p[0], p[1]);
                    p[0] = 17. + x - y;
                    p[1] = -9. + x + y;
                }
            }
        }
        let loops = vec![wire];
        let c = certify(&loops, 1e-7).unwrap();
        assert!(c.separating_normals.contains(&u16::MAX));
        assert_eq!(c.orientation, vec![1]);
        verify(&loops, 1e-7, &c).unwrap();
        let mut bad = c.clone();
        bad.orientation_depth = None;
        assert!(verify(&loops, 1e-7, &bad).is_err());
        if !transformed {
            let mut crossing = loops.clone();
            crossing[0][0].control_points[2][1] = 4.;
            // Simulate a malicious producer: update source binding too.
            // The algebraic census itself must reject the new crossing.
            let mut forged = c.clone();
            forged.source = source_bits(&crossing, 1e-7).unwrap();
            assert!(verify(&crossing, 1e-7, &forged).is_err());
        }
    }
}
