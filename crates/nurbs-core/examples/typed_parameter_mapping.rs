use nurbs_core::{
    Result,
    curve::Curve,
    foundation::{
        MapPiece, ParameterMapping, certify_reparameterization_report,
        evaluate_reparameterized_curve_report, materialize_reparameterized_curve_report,
        parameter_mapping::ParameterMapProof,
    },
};

fn main() -> Result<()> {
    let mapping = ParameterMapping::Pieces(vec![MapPiece {
        domain: [0., 1.],
        range: [0., 1.],
        values: vec![0., 0.3, 1.],
        weights: vec![1., 1., 1.],
    }]);
    let certificate = certify_reparameterization_report(&mapping, None)?;
    let ParameterMapProof::Piecewise(pieces) = certificate.proof else {
        panic!("Expected a piecewise monotonicity certificate");
    };
    assert!(pieces[0].derivative_numerator_bounds[0] > 0.);
    let curve = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]])?;
    let query = evaluate_reparameterized_curve_report(&curve, &mapping, 0.5, None)?;
    assert!((query.source_parameter - 0.4).abs() < 1e-14);
    let materialized = materialize_reparameterized_curve_report(&curve, &mapping, None)?;
    assert_eq!(materialized.curve.degree, 2);
    let point = materialized.curve.evaluate(0.5)?.point;
    assert!((point[0] - query.evaluation.point[0]).abs() < 1e-14);
    println!(
        "Certified map: source parameter {}; materialized degree {}",
        query.source_parameter, materialized.curve.degree
    );
    Ok(())
}
