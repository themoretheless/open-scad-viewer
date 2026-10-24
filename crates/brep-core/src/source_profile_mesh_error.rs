//! Interval mean-value residual for the two affine UV triangles.
use nurbs_core::{Result, interval_eval::Interval as I, surface::Surface};
pub(super) type Derivatives = [[[f64; 2]; 3]; 2];
/// Source validation proves positive rational weights and C0 interior knots.
/// Weight profiles preserve those positive weights. Hermite rail corrections
/// combine their positions/jets with polynomials and nonzero constant widths.
/// Periodic endpoint wrapping needs a separate exact closure qualification.
pub(super) fn continuous_source(source: &Surface) -> Result<bool> {
    source.validate()?;
    Ok(!source.periodic_u && !source.periodic_v)
}
/// Requires continuous support and derivatives enclosing every piece of the
/// rectangle. Integrate S'-T' from corner 0 along the segment inside each UV
/// triangle; the anchor interval includes rounded display-vertex error.
pub(super) fn derivative_error_upper(
    domain: [[f64; 2]; 2],
    corners: [[f64; 3]; 4],
    anchor: [[f64; 2]; 3],
    derivatives: Derivatives,
) -> Result<Option<f64>> {
    let width = [
        I::point(domain[0][1]).sub(I::point(domain[0][0]))?,
        I::point(domain[1][1]).sub(I::point(domain[1][0]))?,
    ];
    if width.iter().any(|w| w.lo <= 0.) {
        return Ok(None);
    }
    let delta = [I::new(0., width[0].hi)?, I::new(0., width[1].hi)?];
    let mut maximum = 0_f64;
    for slopes in [[(1, 0), (2, 1)], [(2, 3), (3, 0)]] {
        let mut sum = I::point(0.);
        for axis in 0..3 {
            let mut error =
                I::new(anchor[axis][0], anchor[axis][1])?.sub(I::point(corners[0][axis]))?;
            for direction in 0..2 {
                let (end, start) = slopes[direction];
                let linear = I::point(corners[end][axis])
                    .sub(I::point(corners[start][axis]))?
                    .div(width[direction])?;
                let d = derivatives[direction][axis];
                error = error.add(I::new(d[0], d[1])?.sub(linear)?.mul(delta[direction])?)?;
            }
            sum = sum.add(I::point(error.lo.abs().max(error.hi.abs())))?;
        }
        maximum = maximum.max(sum.hi);
    }
    Ok(Some(maximum))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuity_authority_checks_source_validation_and_periodic_wrapping() {
        let plane = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        assert!(continuous_source(&plane).unwrap());
        let mut disconnected = plane.clone();
        disconnected.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
        disconnected
            .control_points
            .extend(plane.control_points.clone());
        disconnected.weights = vec![vec![1., 1.]; 4];
        assert!(continuous_source(&disconnected).is_err());
        let mut periodic = plane.clone();
        periodic.periodic_u = true;
        periodic.knots_u = vec![-1., 0., 1., 2., 3.];
        periodic
            .control_points
            .push(plane.control_points[0].clone());
        periodic.weights = vec![vec![1., 1.]; 3];
        assert!(!continuous_source(&periodic).unwrap());
    }
    #[test]
    fn affine_support_is_not_penalized_by_its_physical_size() {
        let corners = [
            [0., 0., 0.],
            [100., 0., 200.],
            [100., 50., 350.],
            [0., 50., 150.],
        ];
        let derivatives = [
            [[100., 100.], [0., 0.], [200., 200.]],
            [[0., 0.], [50., 50.], [150., 150.]],
        ];
        let bound = derivative_error_upper([[0., 1.]; 2], corners, [[0., 0.]; 3], derivatives)
            .unwrap()
            .unwrap();
        assert!(bound > 0. && bound < 1e-9);
        let bound = derivative_error_upper([[0., 1.]; 2], corners, [[0.01, 0.01]; 3], derivatives)
            .unwrap()
            .unwrap();
        assert!(
            bound >= 0.03,
            "Anchor rounding/uncertainty remains in the bound"
        );
    }
    #[test]
    fn degenerate_parameter_width_does_not_claim_a_residual_bound() {
        assert!(
            derivative_error_upper(
                [[1., 1.], [0., 1.]],
                [[0.; 3]; 4],
                [[0., 0.]; 3],
                [[[0., 0.]; 3]; 2]
            )
            .unwrap()
            .is_none()
        );
    }
}
