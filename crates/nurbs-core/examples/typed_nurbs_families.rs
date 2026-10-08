//! Direct native use, including curvature metadata formerly available only in codec output.
use nurbs_core::{
    Result,
    circle_rectangle_transition::{self, RectangleSection},
    circle_transition::{self, CircleSection},
    curve::Curve,
    ellipse_transition::{self, EllipseSection},
    pipe, primitives, profile_sweep, ribbon,
    surface::{DerivativeSide, DerivativeStatus},
};
fn law(a: f64, b: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![a, 0., 0.], vec![b, 0., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn main() -> Result<()> {
    let path = primitives::line([0.; 3], [0., 0., 10.])?;
    let cylinder = pipe::checked(&path, 2., [1., 0., 0.], 8, 1e-8)?;
    assert!(cylinder.report.accepted);
    let cylinder = cylinder.surface.unwrap();
    let jet = cylinder.evaluate(0.13, 0.5)?;
    let (gaussian, mean) = jet.curvatures().unwrap();
    assert!(gaussian.abs() < 1e-12);
    assert!((mean.abs() - 0.25).abs() < 1e-12);
    assert_eq!(jet.derivative_status(), DerivativeStatus::Available);
    assert_eq!(
        jet.derivative_sides(),
        (DerivativeSide::TwoSided, DerivativeSide::TwoSided)
    );
    assert_eq!(jet.domains(), ([0., 1.], [0., 1.]));
    assert!(
        pipe::checked_variable(&path, &law(1., 2.), [1., 0., 0.], 8, 1e-8)?
            .report
            .accepted
    );
    assert!(
        ribbon::checked(&path, &law(1., 2.), [1., 0., 0.], 8, 1e-8)?
            .report
            .accepted
    );
    let profile = primitives::line([1., 0., 0.], [0., 1., 0.])?;
    assert!(
        profile_sweep::checked(&profile, &path, &law(1., 2.), [1., 0., 0.], 8, 1e-8)?
            .report
            .accepted
    );
    let a = CircleSection {
        center: [0.; 3],
        normal: [0., 0., 1.],
        seam: [1., 0., 0.],
        radius: 2.,
    };
    let b = CircleSection {
        center: [0., 0., 10.],
        radius: 3.,
        ..a.clone()
    };
    assert_eq!(circle_transition::ruled(&a, &b)?.control_points.len(), 9);
    let e = EllipseSection {
        center: [0.; 3],
        axis_u: [2., 0., 0.],
        axis_v: [0., 1., 0.],
    };
    assert_eq!(
        ellipse_transition::ruled(
            &e,
            &EllipseSection {
                center: [0., 0., 10.],
                ..e.clone()
            }
        )?
        .control_points
        .len(),
        9
    );
    assert_eq!(
        circle_rectangle_transition::ruled(
            &a,
            &RectangleSection {
                center: [0., 0., 10.],
                axis_u: [3., 0., 0.],
                axis_v: [0., 1., 0.]
            }
        )?
        .len(),
        4
    );
    println!(
        "7 native constructor families and typed surface curvature/jet metadata passed; no codec, WASM or viewer required."
    );
    Ok(())
}
