use brep_core::profile_certificate::original_restriction::{
    self as exact,
    wire::{self, jordan, material, nesting},
};
use nurbs_core::curve::Curve;
fn ratio(n: &str, d: &str) -> [String; 2] {
    [n.into(), d.into()]
}
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
fn region(wires: Vec<wire::Wire>) -> nesting::Nesting {
    nesting::qualify(
        &jordan::qualify(&wires, jordan::Limits::default()).unwrap(),
        nesting::Limits::default(),
    )
    .unwrap()
}
fn rectangle(x: [f64; 2], y: [f64; 2], reverse: bool) -> wire::Wire {
    let mut p = vec![[x[0], y[0]], [x[1], y[0]], [x[1], y[1]], [x[0], y[1]]];
    if reverse {
        p.reverse();
    }
    let curves = (0..4)
        .map(|i| line(p[i], p[(i + 1) % 4]))
        .collect::<Vec<_>>();
    let pieces = curves
        .iter()
        .map(|c| exact::restrict_binary64(c, c.domain()).unwrap())
        .collect();
    wire::sew(pieces, vec![false; 4], ratio("0", "1")).unwrap()
}
#[test]
fn coincident_boundaries_and_crossing_rectangles_have_complete_side_census() {
    let source = region(vec![rectangle([0., 4.], [0., 4.], false)]);
    let equal = material::inspect(&source, &source, material::Limits::default()).unwrap();
    assert!(equal.result_is_subset() && equal.source_is_subset());
    equal.reverify().unwrap();
    let result = region(vec![rectangle([2., 6.], [1., 3.], false)]);
    let relation = material::inspect(&source, &result, material::Limits::default()).unwrap();
    assert!(relation.added().next().is_some());
    assert!(relation.removed().next().is_some());
    relation.reverify().unwrap();
    assert!(relation.require_no_addition().is_err());
}
#[test]
fn filled_source_hole_is_detected_even_with_identical_outer_boundary() {
    let source = region(vec![
        rectangle([0., 4.], [0., 4.], false),
        rectangle([1., 3.], [1., 3.], true),
    ]);
    let result = region(vec![rectangle([0., 4.], [0., 4.], false)]);
    let relation = material::inspect(&source, &result, material::Limits::default()).unwrap();
    assert!(relation
        .added()
        .any(|s| s.source_winding == 0 && s.result_winding == 1));
    assert!(relation.source_is_subset());
    relation.reverify().unwrap();
}
#[test]
fn nonbinary_rational_wire_controls_are_compared_without_rounding() {
    let a = line([0., 0.], [1., 0.]);
    let b = line([0.3, 0.], [0.3, 1.]);
    let c = line([1., 1.], [0., 1.]);
    let d = line([0., 1.], [0., 0.]);
    let pieces = vec![
        exact::restrict(&a, [ratio("0", "1"), ratio("1", "3")]).unwrap(),
        exact::restrict_binary64(&b, b.domain()).unwrap(),
        exact::restrict(&c, [ratio("2", "3"), ratio("1", "1")]).unwrap(),
        exact::restrict_binary64(&d, d.domain()).unwrap(),
    ];
    let result = wire::sew(pieces, vec![false, true, false, false], ratio("1", "1")).unwrap();
    assert_eq!(
        result.pieces()[1][0].homogeneous_controls()[0][0],
        ratio("1", "3")
    );
    let result = region(vec![result]);
    let source = region(vec![rectangle([0., 1.], [0., 1.], false)]);
    let relation = material::inspect(&source, &result, material::Limits::default()).unwrap();
    assert!(relation.result_is_subset());
    assert!(!relation.source_is_subset());
    relation.reverify().unwrap();
    relation.require_no_addition().unwrap();
    let limits = material::Limits {
        max_pair_projections: 0,
        ..Default::default()
    };
    assert!(material::inspect(&source, &result, limits).is_err());
}

fn curved_top(end: f64, middle: f64) -> nesting::Nesting {
    let top = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., end], vec![0.5, middle], vec![0., end]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let curves = vec![
        line([0., 0.], [1., 0.]),
        line([1., 0.], [1., end]),
        top,
        line([0., end], [0., 0.]),
    ];
    let pieces = curves
        .iter()
        .map(|c| exact::restrict_binary64(c, c.domain()).unwrap())
        .collect();
    region(vec![
        wire::sew(pieces, vec![false; 4], ratio("0", "1")).unwrap()
    ])
}
#[test]
fn two_hidden_crossings_and_even_tangency_are_fully_partitioned() {
    let source = region(vec![rectangle([0., 1.], [0., 1.], false)]);
    let crossing = curved_top(0.75, 1.5);
    let relation = material::inspect(&source, &crossing, material::Limits::default()).unwrap();
    assert!(relation.added().next().is_some());
    assert!(relation.removed().next().is_some());
    relation.reverify().unwrap();
    let tangent = curved_top(0.75, 1.25);
    let relation = material::inspect(&source, &tangent, material::Limits::default()).unwrap();
    assert!(relation.result_is_subset());
    assert!(!relation.source_is_subset());
    relation.reverify().unwrap();
}
#[test]
fn thin_added_band_is_not_lost_to_a_tolerance_or_probe_grid() {
    let source = region(vec![rectangle([0., 1.], [0., 1.], false)]);
    let result = region(vec![rectangle([0., 1. + 2f64.powi(-45)], [0., 1.], false)]);
    let relation = material::inspect(&source, &result, material::Limits::default()).unwrap();
    assert!(!relation.result_is_subset());
    assert!(relation.source_is_subset());
    relation.reverify().unwrap();
}
