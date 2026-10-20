use brep_core::profile_certificate::{self as profile, WindingRegularization};
use nurbs_core::curve::Curve;
fn rectangle(b: [f64; 4], clockwise: bool, cubic: bool) -> Vec<Curve> {
    let [x0, x1, y0, y1] = b;
    let mut p = vec![vec![x0, y0], vec![x1, y0], vec![x1, y1], vec![x0, y1]];
    if clockwise {
        p.reverse();
    }
    (0..4)
        .map(|i| {
            let mut c = Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap();
            if cubic {
                c = c.elevate(3).unwrap();
                c.knots = vec![0.1, 0.1, 0.1, 0.1, 1.1, 1.1, 1.1, 1.1];
            }
            c
        })
        .collect()
}
fn inside(wire: &[Curve], q: [f64; 2]) -> bool {
    let mut b = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
    for p in wire.iter().flat_map(|c| &c.control_points) {
        for k in 0..2 {
            b[0][k] = b[0][k].min(p[k]);
            b[1][k] = b[1][k].max(p[k]);
        }
    }
    b[0][0] < q[0] && q[0] < b[1][0] && b[0][1] < q[1] && q[1] < b[1][1]
}
fn filled(result: &WindingRegularization, q: [f64; 2]) -> bool {
    result
        .components()
        .iter()
        .any(|c| inside(&c.wires()[0], q) && !c.wires()[1..].iter().any(|w| inside(w, q)))
}
#[test]
fn nested_same_orientation_wires_remove_only_nonzero_internal_boundaries() {
    let source = vec![
        rectangle([0., 10., 0., 10.], false, false),
        rectangle([2., 8., 2., 8.], false, false),
    ];
    // Ordinary material-left admission remains strict.
    assert!(profile::certify(&source, 1e-7).is_err());
    let result = profile::regularize_nonzero_winding(&source, &[1, 2], 1e-7).unwrap();
    result.verify().unwrap();
    assert_eq!(result.winding(), &[[0, 1], [1, 3]]);
    assert_eq!(result.removed_source_loops(), &[1]);
    assert_eq!(result.components().len(), 1);
    assert_eq!(result.components()[0].wires(), &source[..1]);
    assert_eq!(result.components()[0].source_loops(), &[0]);
    for q in [[1., 1.], [3., 3.], [9., 9.], [11., 11.]] {
        assert_eq!(filled(&result, q), inside(&source[0], q));
    }
}
#[test]
fn signed_multiplicities_assign_holes_and_negative_winding_material_islands() {
    let source = vec![
        rectangle([0., 12., 0., 12.], false, false),
        rectangle([2., 10., 2., 10.], false, false),
        rectangle([4., 8., 4., 8.], false, false),
        rectangle([5., 7., 5., 7.], false, false),
    ];
    let coefficients = [2, -2, -3, 3];
    let result = profile::regularize_nonzero_winding(&source, &coefficients, 1e-7).unwrap();
    result.verify().unwrap();
    assert_eq!(result.winding(), &[[0, 2], [2, 0], [0, -3], [-3, 0]]);
    assert_eq!(result.components().len(), 2);
    assert_eq!(result.components()[0].source_loops(), &[0, 1]);
    assert_eq!(result.components()[1].source_loops(), &[2, 3]);
    for component in result.components() {
        assert_eq!(component.reversed(), &[false, true]);
        assert_eq!(component.verified_profile().nesting_depth(), &[0, 1]);
    }
    for i in -1..14 {
        for j in -1..14 {
            let q = [i as f64 + 0.25, j as f64 + 0.25];
            let winding: i32 = source
                .iter()
                .zip(coefficients)
                .filter(|(w, _)| inside(w, q))
                .map(|(_, c)| c)
                .sum();
            assert_eq!(filled(&result, q), winding != 0, "q={q:?}");
        }
    }
}
#[test]
fn exact_reversal_preserves_cubic_sources_and_non_dyadic_domains() {
    let mut source = vec![
        rectangle([0., 10., 0., 10.], true, true),
        rectangle([2., 8., 2., 8.], false, true),
    ];
    let original = source.clone();
    assert!(profile::certify(&source, 1e-7).is_err());
    let result = profile::regularize_nonzero_winding(&source, &[1, 1], 1e-7).unwrap();
    result.verify().unwrap();
    assert_eq!(result.components()[0].reversed(), &[true, true]);
    for c in result.components()[0].wires().iter().flatten() {
        assert_eq!(c.domain(), [-1.1, -0.1]);
        assert_eq!(c.degree, 3);
        assert_eq!(c.weights, vec![1.; 4]);
    }
    let output = &result.components()[0].wires()[0];
    for (a, b) in original[0].iter().rev().zip(output) {
        for t in [0.1, 0.3, 0.7, 1.1] {
            let a = a.evaluate_de_boor(t).unwrap();
            let b = b.evaluate_de_boor(-t).unwrap();
            assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12));
        }
    }
    source[0][0].control_points[0][0] = 100.;
    assert_eq!(result.source(), original);
    result.verify().unwrap();
    assert!(filled(&result, [1., 1.]));
    assert!(!filled(&result, [3., 3.]));
}
#[test]
fn zero_chains_are_empty_but_invalid_or_unseparated_sources_never_get_a_region() {
    let source = vec![rectangle([0., 2., 0., 2.], false, false)];
    let empty = profile::regularize_nonzero_winding(&source, &[0], 1e-7).unwrap();
    empty.verify().unwrap();
    assert!(empty.components().is_empty());
    assert_eq!(empty.removed_source_loops(), &[0]);
    assert!(
        profile::regularize_nonzero_winding(&[], &[], 1e-7)
            .unwrap()
            .components()
            .is_empty()
    );
    assert!(profile::regularize_nonzero_winding(&source, &[], 1e-7).is_err());
    assert!(profile::regularize_nonzero_winding(&source, &[i32::MIN], 1e-7).is_err());
    assert!(profile::regularize_nonzero_winding(&source, &[1], f64::NAN).is_err());
    for b in [[1., 3., 1., 3.], [2., 4., 2., 4.], [0., 2., 0., 2.]] {
        let crossing = vec![source[0].clone(), rectangle(b, false, false)];
        assert!(profile::regularize_nonzero_winding(&crossing, &[1, -1], 1e-7).is_err());
    }
}

#[test]
fn original_rational_ellipses_remain_compact_through_winding_normalization() {
    let mut source = vec![
        brep_core::sketch::circle_wire(3.).unwrap(),
        brep_core::sketch::circle_wire(1.).unwrap(),
    ];
    for curve in source.iter_mut().flatten() {
        for p in &mut curve.control_points {
            let (x, y) = (p[0], p[1]);
            p[0] = 3. * x + y;
            p[1] = 2. * y;
        }
    }
    let original = source.clone();
    let result = profile::regularize_nonzero_winding(&source, &[1, -1], 1e-7).unwrap();
    result.verify().unwrap();
    assert_eq!(result.winding(), &[[0, 1], [1, 0]]);
    let component = &result.components()[0];
    assert_eq!(component.wires()[0], original[0]);
    assert_eq!(component.wires().iter().map(Vec::len).sum::<usize>(), 8);
    assert_eq!(component.reversed(), &[false, true]);
    assert!(
        component
            .wires()
            .iter()
            .flatten()
            .all(|c| c.degree == 2 && c.control_points.len() == 3)
    );
    assert!(
        component
            .wires()
            .iter()
            .flatten()
            .any(|c| c.weights.iter().any(|w| *w != 1.))
    );
    assert_eq!(result.source(), original);
}
