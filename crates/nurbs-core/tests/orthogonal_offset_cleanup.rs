use nurbs_core::{
    chord_winding::FillRule,
    curve::Curve,
    curve_offset::{CleanupLimits, CleanupStop, cleanup_orthogonal_miter_offset as cleanup},
};
fn wire(points: &[[f64; 2]], z: Option<f64>) -> Vec<Curve> {
    (0..points.len())
        .map(|i| {
            Curve::from_polyline(
                [points[i], points[(i + 1) % points.len()]]
                    .into_iter()
                    .map(|p| {
                        let mut v = p.to_vec();
                        if let Some(z) = z {
                            v.push(z)
                        }
                        v
                    })
                    .collect(),
            )
            .unwrap()
        })
        .collect()
}
fn square() -> Vec<Curve> {
    wire(&[[0., 0.], [4., 0.], [4., 4.], [0., 4.]], None)
}
fn slot() -> Vec<Curve> {
    wire(
        &[
            [0., 0.],
            [8., 0.],
            [8., 8.],
            [5., 8.],
            [5., 2.],
            [3., 2.],
            [3., 8.],
            [0., 8.],
        ],
        Some(3.),
    )
}
fn area(curves: &[Curve]) -> f64 {
    curves
        .iter()
        .flat_map(|c| c.control_points.windows(2))
        .map(|p| p[0][0] * p[1][1] - p[1][0] * p[0][1])
        .sum::<f64>()
        * 0.5
}
#[test]
fn exact_original_miter_square_preserves_zero_transfer_error_and_signed_distance() {
    let source = square();
    let before = source.clone();
    let r = cleanup(&source, 0.5, FillRule::NonZero, CleanupLimits::default()).unwrap();
    assert_eq!(r.stop, CleanupStop::Complete, "{r:?}");
    assert_eq!(r.source_to_arrangement_error_upper, Some(0.));
    assert_eq!(
        r.offset_vertices,
        vec![[0.5, 0.5], [3.5, 0.5], [3.5, 3.5], [0.5, 3.5]]
    );
    let loops = r.curves.unwrap();
    assert_eq!(loops.len(), 1);
    assert_eq!(area(&loops[0]), 9.);
    assert_eq!(source, before);
    let r = cleanup(&source, -0.5, FillRule::NonZero, CleanupLimits::default()).unwrap();
    assert_eq!(area(&r.curves.unwrap()[0]), 25.);
}
#[test]
fn exact_overlap_cancellation_and_proper_crossings_clean_the_original_offset_image() {
    let source = slot();
    let r = cleanup(&source, 1., FillRule::NonZero, CleanupLimits::default()).unwrap();
    assert_eq!(r.stop, CleanupStop::Complete, "{r:?}");
    let loops = r.curves.unwrap();
    assert_eq!(loops.len(), 2);
    assert_eq!(loops.iter().map(|c| area(c)).sum::<f64>(), 12.);
    assert!(
        loops
            .iter()
            .flatten()
            .flat_map(|c| &c.control_points)
            .all(|p| p.len() == 3 && p[2] == 3.)
    );
    let r = cleanup(&source, 1.25, FillRule::NonZero, CleanupLimits::default()).unwrap();
    assert_eq!(r.stop, CleanupStop::Complete, "{r:?}");
    assert_eq!(r.curves.unwrap().len(), 3);
    assert_eq!(r.source_to_arrangement_error_upper, Some(0.));
}
#[test]
fn precision_degeneracy_unsupported_curves_and_work_limits_never_return_a_fit() {
    let r = cleanup(&square(), 2., FillRule::NonZero, CleanupLimits::default()).unwrap();
    assert_eq!(r.stop, CleanupStop::CollapsedOffset);
    assert!(r.curves.is_none());
    let fractional = wire(&[[0.1, 0.1], [4.1, 0.1], [4.1, 4.1], [0.1, 4.1]], None);
    let r = cleanup(
        &fractional,
        0.2,
        FillRule::NonZero,
        CleanupLimits::default(),
    )
    .unwrap();
    assert_eq!(r.stop, CleanupStop::PrecisionLimit);
    assert!(r.curves.is_none());
    let mut curved = square();
    curved[0] = curved[0].elevate(2).unwrap();
    let r = cleanup(&curved, 0.5, FillRule::NonZero, CleanupLimits::default()).unwrap();
    assert_eq!(r.stop, CleanupStop::UnsupportedSource);
    assert!(r.curves.is_none());
    let r = cleanup(
        &slot(),
        1.,
        FillRule::NonZero,
        CleanupLimits {
            pairs: 1,
            ..CleanupLimits::default()
        },
    )
    .unwrap();
    assert_eq!(r.stop, CleanupStop::WorkLimit);
    assert!(r.curves.is_none());
    assert!(cleanup(&[], 1., FillRule::EvenOdd, CleanupLimits::default()).is_err());
    let mut malformed = square();
    malformed[1].control_points.clear();
    assert!(cleanup(&malformed, 0.5, FillRule::NonZero, CleanupLimits::default()).is_err());
}
