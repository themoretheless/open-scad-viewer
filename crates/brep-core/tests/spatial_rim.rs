use brep_core::region_replacement::inspect_spatial_rim;
use nurbs_core::curve::Curve;
fn polygon(points: &[[f64; 3]]) -> Vec<Curve> {
    (0..points.len())
        .map(|i| {
            Curve::from_polyline(vec![
                points[i].to_vec(),
                points[(i + 1) % points.len()].to_vec(),
            ])
            .unwrap()
        })
        .collect()
}
#[test]
fn original_spatial_curve_bounds_distinguish_square_from_bow_tie() {
    let square = polygon(&[[0., 0., 0.], [1., 0., 1.], [1., 1., 2.], [0., 1., 1.]]);
    let simple = inspect_spatial_rim(&square, 1e-7, 10000).unwrap();
    assert!(simple.proven_simple);
    assert_eq!(simple.pairs.len(), 6);
    let bow = polygon(&[[0., 0., 0.], [1., 1., 2.], [0., 1., 1.], [1., 0., 1.]]);
    let crossed = inspect_spatial_rim(&bow, 1e-7, 10000).unwrap();
    assert!(!crossed.proven_simple);
    assert!(crossed.pairs.iter().any(|(_, ok)| !*ok));
}
#[test]
fn mismatching_original_endpoints_are_refused() {
    let mut square = polygon(&[[0., 0., 0.], [1., 0., 1.], [1., 1., 2.], [0., 1., 1.]]);
    square[1].control_points[0][0] += 1e-12;
    assert!(inspect_spatial_rim(&square, 1e-7, 1000).is_err());
}
