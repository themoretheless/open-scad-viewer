//! Native tensor-product edits. No fitting or snapping; real-arithmetic
//! geometric preservation does not constitute a binary64 error certificate.
use crate::{
    Result, check,
    curve::Curve,
    surface::{Axis, Surface},
};

pub fn insert(source: &Surface, axis: Axis, parameter: f64, count: usize) -> Result<Surface> {
    source.edit_axis(axis, |c| c.insert(parameter, count))
}
pub fn elevate(source: &Surface, axis: Axis, degree: usize) -> Result<Surface> {
    source.edit_axis(axis, |c| c.elevate(degree))
}
/// Both outputs retain their original parameter coordinates and meet at the
/// requested isoparametric seam. Split must be strictly within the domain.
pub fn split(source: &Surface, axis: Axis, parameter: f64) -> Result<[Surface; 2]> {
    source.validate()?;
    let (degree, knots, count) = match axis {
        Axis::U => (
            source.degree_u,
            &source.knots_u,
            source.control_points.len(),
        ),
        Axis::V => (
            source.degree_v,
            &source.knots_v,
            source.control_points[0].len(),
        ),
    };
    let a = knots[degree];
    let b = knots[count];
    check(
        parameter.is_finite() && a < parameter && parameter < b,
        "Surface split must lie strictly inside the active domain",
    )?;
    Ok([
        source.edit_axis(axis, |c| c.trim(a, parameter))?,
        source.edit_axis(axis, |c| c.trim(parameter, b))?,
    ])
}
pub use crate::core::surface_transpose::transpose;
/// Reverse one parameter within its existing active interval; also reverses
/// normal orientation. Periodic data follow the native curve reverse contract.
pub fn reverse(source: &Surface, axis: Axis) -> Result<Surface> {
    source.edit_axis(axis, Curve::reverse)
}
/// Split at every active knot into clamped rational Bezier patches. Retains
/// original U/V intervals. Output order is U-major; maximum 1024 patches.
pub fn decompose(source: &Surface) -> Result<Vec<Surface>> {
    source.validate()?;
    let breaks = |knots: &[f64], degree: usize, count: usize| {
        let mut out = knots
            .iter()
            .copied()
            .filter(|&k| k >= knots[degree] && k <= knots[count])
            .collect::<Vec<_>>();
        out.dedup();
        out
    };
    let u = breaks(
        &source.knots_u,
        source.degree_u,
        source.control_points.len(),
    );
    let v = breaks(
        &source.knots_v,
        source.degree_v,
        source.control_points[0].len(),
    );
    check(
        (u.len() - 1) * (v.len() - 1) <= 1024,
        "Bezier decomposition exceeds patch budget",
    )?;
    let mut out = Vec::new();
    for a in u.windows(2) {
        for b in v.windows(2) {
            out.push(source.trim([a[0], a[1], b[0], b[1]])?);
        }
    }
    Ok(out)
}

/// Extend one exactly periodic axis by a second period in the original UV
/// coordinates. The output has an ordinary, nonperiodic lifted axis.
pub fn lift_two_periods(source: &Surface, axis: Axis) -> Result<Surface> {
    use num_rational::BigRational as Q;
    use num_traits::ToPrimitive;

    source.edit_axis(axis, |curve| {
        check(
            curve.periodic,
            "A lifted axis requires periodic source storage",
        )?;
        let count = curve.control_points.len();
        let stride = count - curve.degree;
        let exact = |x| Q::from_float(x).expect("validated knot");
        let period = exact(curve.knots[count]) - exact(curve.knots[curve.degree]);
        check(
            curve
                .knots
                .windows(stride + 1)
                .all(|window| exact(window[stride]) - exact(window[0]) == period),
            "Lifting requires exact periodic knot translations",
        )?;
        let mut lifted = curve.clone();
        lifted.periodic = false;
        lifted
            .control_points
            .extend_from_slice(&curve.control_points[curve.degree..]);
        lifted
            .weights
            .extend_from_slice(&curve.weights[curve.degree..]);
        for &knot in &curve.knots[curve.knots.len() - stride..] {
            let shifted = exact(knot) + &period;
            let value = shifted
                .to_f64()
                .filter(|x| Q::from_float(*x).as_ref() == Some(&shifted))
                .ok_or_else(|| crate::input("Lifted knot is not exactly representable"))?;
            lifted.knots.push(value);
        }
        Ok(lifted)
    })
}

#[cfg(test)]
mod lifted_tests {
    use super::*;

    #[test]
    fn lifted_axis_preserves_both_periods_and_the_original_seam() {
        let ring = [[0., 0.], [1., 1.], [2., 0.], [1., -1.], [0., 0.]];
        let source = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![-0.25, 0., 0.25, 0.5, 0.75, 1., 1.25],
            control_points: [0., 1.]
                .map(|y| ring.map(|[x, z]| vec![x, y, z]).to_vec())
                .to_vec(),
            weights: vec![vec![1.; 5]; 2],
            periodic_u: false,
            periodic_v: true,
        };
        let lifted = lift_two_periods(&source, Axis::V).unwrap();
        assert!(!lifted.periodic_v);
        for v in [0., 0.125, 0.5, 0.875, 1.] {
            let original = source.evaluate(0.375, v).unwrap().point;
            assert_eq!(lifted.evaluate(0.375, v).unwrap().point, original);
            assert_eq!(lifted.evaluate(0.375, 1. + v).unwrap().point, original);
        }
        assert!(lift_two_periods(&source, Axis::U).is_err());
    }
}
