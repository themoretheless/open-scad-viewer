use nurbs_core::{
    Result,
    curve::Curve,
    foundation::{self, ExactEditOperation},
};
fn main() -> Result<()> {
    let curve = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]])?;
    let inserted =
        foundation::certify_exact_edit_report(&curve, ExactEditOperation::Insert, 0.5, 1, None)?;
    assert_eq!(
        inserted.curve.evaluate(0.25)?.point,
        curve.evaluate(0.25)?.point
    );
    let mapped = foundation::reparameterize_curve_report(&curve, [-2., 2.], None)?;
    assert_eq!(mapped.curve.domain(), [-2., 2.]);
    assert_eq!(
        mapped.curve.evaluate(-1.)?.point,
        curve.evaluate(0.25)?.point
    );
    println!("Exact insertion and affine parameter map available directly in Rust");
    Ok(())
}
