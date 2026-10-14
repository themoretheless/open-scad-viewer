use super::*;

fn rectangle(min: [f64; 2], max: [f64; 2]) -> Vec<[f64; 2]> {
    vec![min, [max[0], min[1]], max, [min[0], max[1]]]
}

fn region_area(rings: &Rings) -> f64 {
    rings.iter().map(|r| area(r)).sum()
}

#[test]
fn curvex_open_gradient_stroke_polyline_remains_closed_region() {
    let p0 = [10_f32, 75.];
    let c1 = [15_f32, -20.];
    let c2 = [85_f32, 120.];
    let p3 = [90_f32, 25.];
    let mut points = Vec::new();
    for i in 0..=28 {
        let t = i as f32 / 28.;
        let mt = 1. - t;
        let (a, b, c, d) = (mt * mt * mt, 3. * mt * mt * t, 3. * mt * t * t, t * t * t);
        points.push([
            (a * p0[0] + b * c1[0] + c * c2[0] + d * p3[0]) as f64,
            (a * p0[1] + b * c1[1] + c * c2[1] + d * p3[1]) as f64,
        ]);
    }
    let path = crate::path::BezierPath::from_polyline(&points, false).unwrap();
    let contours = crate::stroke::outline_stroke_tol(
        &path,
        &crate::stroke::StrokeOptions {
            width: 7.,
            ..Default::default()
        },
        0.05,
    )
    .unwrap();
    let rings = contours.iter().map(|p| p.to_ring(0.05).unwrap()).collect();
    let mesh = crate::tessellation::tessellate_rings(&rings, FillRule::NonZero).unwrap();
    assert!(mesh.triangle_count() >= 28);
    let expected = points
        .windows(2)
        .map(|p| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]))
        .sum::<f64>()
        * 7.;
    assert!((region_area(&rings) - expected).abs() < 1e-6);
}

#[test]
fn convex_fast_route_requires_hull_order_not_only_turn_signs() {
    let points: Vec<_> = (0..5)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 5.;
            [a.cos(), a.sin()]
        })
        .collect();
    assert!(convex_boundary(&points).is_some());
    assert!(convex_boundary(&points.iter().rev().copied().collect::<Vec<_>>()).is_some());
    let pentagram: Vec<_> = (0..5).map(|i| points[(i * 2) % 5]).collect();
    assert!(convex_boundary(&pentagram).is_none());
    let normalized = normalize(&vec![pentagram], FillRule::EvenOdd).unwrap();
    assert_eq!(normalized.len(), 5);
}

#[test]
fn fill_rules_combine_winding_across_nested_rings() {
    let outer = rectangle([0., 0.], [10., 10.]);
    let inner = rectangle([2., 2.], [8., 8.]);
    let same = vec![outer.clone(), inner.clone()];
    assert_eq!(
        region_area(&normalize(&same, FillRule::NonZero).unwrap()),
        100.
    );
    assert_eq!(
        region_area(&normalize(&same, FillRule::EvenOdd).unwrap()),
        64.
    );
    let opposite = vec![outer, inner.into_iter().rev().collect()];
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let result = normalize(&opposite, rule).unwrap();
        assert_eq!(region_area(&result), 64.);
        assert!(!inside([5., 5.], &result));
        assert!(inside([1., 1.], &result));
    }
}

#[test]
fn normalization_resolves_overlap_and_zero_signed_area_crossing() {
    let overlap = vec![rectangle([0., 0.], [4., 4.]), rectangle([2., 0.], [6., 4.])];
    assert_eq!(
        region_area(&normalize(&overlap, FillRule::NonZero).unwrap()),
        24.
    );
    assert_eq!(
        region_area(&normalize(&overlap, FillRule::EvenOdd).unwrap()),
        16.
    );
    let bowtie = vec![vec![[0., 0.], [4., 4.], [0., 4.], [4., 0.]]];
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let result = normalize(&bowtie, rule).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(region_area(&result), 8.);
    }
}

#[test]
fn clockwise_and_cancelled_inputs_are_valid_fills() {
    let ring = rectangle([0., 0.], [4., 4.]);
    let reversed: Vec<_> = ring.iter().rev().copied().collect();
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        assert_eq!(
            region_area(&normalize(&vec![reversed.clone()], rule).unwrap()),
            16.
        );
        assert!(
            normalize(&vec![ring.clone(), reversed.clone()], rule)
                .unwrap()
                .is_empty()
        );
    }
    assert!(
        normalize(&vec![ring.clone(), ring], FillRule::EvenOdd)
            .unwrap()
            .is_empty()
    );
    assert!(normalize(&vec![], FillRule::NonZero).unwrap().is_empty());
    // Preserve the existing Positive-only CAD API.
    assert!(
        planar(&vec![reversed], &vec![], "union")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn booleans_allow_a_different_fill_rule_for_each_operand() {
    let a = vec![
        rectangle([0., 0.], [10., 10.]),
        rectangle([2., 2.], [8., 8.]),
    ];
    let b = vec![rectangle([4., 0.], [6., 10.]).into_iter().rev().collect()];
    let cut = planar_with_rules(&a, &b, "intersection", FillRule::EvenOdd, FillRule::NonZero)
        .unwrap();
    assert_eq!(region_area(&cut), 8.);
    let solid = planar_with_rules(&a, &b, "intersection", FillRule::NonZero, FillRule::EvenOdd)
        .unwrap();
    assert_eq!(region_area(&solid), 20.);
    assert!(
        planar_with_rules(&a, &b, "unknown", FillRule::NonZero, FillRule::NonZero).is_err()
    );
}

#[test]
fn normalization_preserves_small_translated_regions_and_rejects_invalid_coordinates() {
    let ring = rectangle([900_000., 900_000.], [900_000.001, 900_000.001]);
    let expected = area(&ring);
    assert!(expected > 9e-7);
    let result = normalize(&vec![ring], FillRule::NonZero).unwrap();
    assert!((region_area(&result) - expected).abs() < 1e-12);
    assert!(
        normalize(
            &vec![rectangle([0., 0.], [f64::NAN, 1.])],
            FillRule::NonZero
        )
        .is_err()
    );
}

#[test]
fn compound_normalization_is_independent_of_document_units() {
    for (origin, unit) in [(1e7, 10.), (0., 1e30), (0., 1e-30)] {
        let p = |x, y| [origin + x * unit, origin + y * unit];
        let rings = vec![
            rectangle(p(0., 0.), p(10., 10.)),
            rectangle(p(2., 2.), p(8., 8.)),
        ];
        let expected = area(&rings[0]) - area(&rings[1]);
        let result = normalize(&rings, FillRule::EvenOdd).unwrap();
        assert_eq!(result.len(), 2, "origin {origin}, unit {unit}");
        assert!((region_area(&result) / expected - 1.).abs() < 1e-12);
        assert!(inside(p(1., 1.), &result));
        assert!(!inside(p(5., 5.), &result));
    }
    // Non-finite intermediate products must fail before the convex shortcut.
    assert!(
        normalize(
            &vec![rectangle([0., 0.], [1e200, 1e200])],
            FillRule::NonZero
        )
        .is_err()
    );
}

#[test]
fn planar_overlap_and_hole() {
    let a = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
    let b = vec![vec![[1., 1.], [3., 1.], [3., 3.], [1., 3.]]];
    let r = planar(&a, &b, "difference").unwrap();
    assert!((r.iter().map(|r| area(r)).sum::<f64>() - 12.).abs() < 1e-8);
}

#[test]
fn planar_xor() {
    let a = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
    let b = vec![vec![[2., 2.], [6., 2.], [6., 6.], [2., 6.]]];
    let r = planar(&a, &b, "xor").unwrap();
    let s: f64 = r.iter().map(|ring| area(ring).abs()).sum();
    assert!((s - 24.).abs() < 1e-6, "xor area={s}");
}
