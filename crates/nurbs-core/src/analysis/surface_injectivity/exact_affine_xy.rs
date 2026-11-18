//! Exact homogeneous identities for nondegenerate affine coordinate projections.
use crate::{Result, check, surface::Surface};
use num_rational::BigRational as Q;
use num_traits::Zero;

fn admit(s: &Surface) -> Result<()> {
    s.validate()?;
    check(
        !s.periodic_u && !s.periodic_v
            && (1..=8).contains(&s.degree_u) && (1..=8).contains(&s.degree_v)
            && s.control_points.len() == s.degree_u + 1
            && s.control_points[0].len() == s.degree_v + 1
            && s.control_points.iter().flatten().all(|point| point.len() == 3),
        "Exact affine XY projection needs a spatial single-span chart",
    )?;
    for (degree, knots) in [(s.degree_u, &s.knots_u), (s.degree_v, &s.knots_v)] {
        check(
            knots[..=degree].iter().all(|&v| v == knots[degree])
                && knots[degree + 1..].iter().all(|&v| v == knots[degree + 1]),
            "Exact affine XY projection needs clamped knots",
        )?;
    }
    Ok(())
}

fn axis(s: &Surface, parameter: usize, coordinate: usize) -> Result<[Q; 2]> {
    let degrees = [s.degree_u, s.degree_v];
    let degree = degrees[parameter];
    let lines = degrees[1 - parameter] + 1;
    let exact = |value| Q::from_float(value).ok_or_else(|| crate::input("Nonfinite affine coefficient"));
    let indices = |k, line| if parameter == 0 { (k, line) } else { (line, k) };
    let (i, j) = indices(degree, 0);
    let lo = exact(s.control_points[0][0][coordinate])?;
    let hi = exact(s.control_points[i][j][coordinate])?;
    check(lo != hi, "Exact affine XY projection is degenerate")?;
    for line in 0..lines {
        let residual = |k, endpoint: &Q| -> Result<Q> {
            let (i, j) = indices(k, line);
            Ok(exact(s.weights[i][j])? * (exact(s.control_points[i][j][coordinate])? - endpoint))
        };
        // Compare the elevated homogeneous numerator to W times the
        // affine coordinate. Multiplication by degree+1 clears the
        // common Bernstein elevation denominator.
        for k in 0..=degree + 1 {
            let mut difference = Q::zero();
            if k <= degree {
                difference += residual(k, &lo)? * Q::from_integer((degree + 1 - k).into());
            }
            if k > 0 {
                difference += residual(k - 1, &hi)? * Q::from_integer(k.into());
            }
            check(difference.is_zero(), "Exact affine XY coefficient identity failed")?;
        }
    }
    Ok([lo.clone(), hi - lo])
}

pub(crate) fn frame_on(s: &Surface, axes: [usize; 2]) -> Result<[Q; 4]> {
    admit(s)?;
    let [x, dx] = axis(s, 0, axes[0])?;
    let [y, dy] = axis(s, 1, axes[1])?;
    Ok([x, y, dx, dy])
}

/// Cartesian coordinate pair whose homogeneous identities prove a nonsingular
/// affine map of the full single-span UV chart.
pub fn projection(s: &Surface) -> Option<[usize; 2]> {
    admit(s).ok()?;
    let affine = [0, 1, 2].map(|coordinate| affine_row(s, coordinate));
    [[0, 1], [0, 2], [1, 2]].into_iter().find(|&[x, y]| {
        affine[x].as_ref().zip(affine[y].as_ref()).is_some_and(|(a, b)| {
            &a[0] * &b[1] != &a[1] * &b[0]
        })
    })
}

fn affine_row(s: &Surface, coordinate: usize) -> Option<[Q; 2]> {
    let [p, q] = [s.degree_u, s.degree_v];
    let [origin, end_u, end_v] = [[0, 0], [p, 0], [0, q]]
        .map(|[i, j]| Q::from_float(s.control_points[i][j][coordinate]));
    let origin = origin?;
    let slopes = [end_u? - &origin, end_v? - &origin];
    let controls = [
        origin.clone(), &origin + &slopes[0], &origin + &slopes[1],
        &origin + &slopes[0] + &slopes[1],
    ];
    let weighted_residual = |i: usize, j: usize, corner: &Q| -> Option<Q> {
        let point = &s.control_points[i][j];
        Some(Q::from_float(s.weights[i][j])? * (Q::from_float(point[coordinate])? - corner))
    };
    for i in 0..=p + 1 {
        for j in 0..=q + 1 {
            let mut difference = Q::zero();
            for u in 0..=1 {
                for v in 0..=1 {
                    let (Some(k), Some(l)) = (i.checked_sub(u), j.checked_sub(v)) else {
                        continue;
                    };
                    if k > p || l > q {
                        continue;
                    }
                    let scale_u = if u == 0 { p + 1 - i } else { i };
                    let scale_v = if v == 0 { q + 1 - j } else { j };
                    difference += weighted_residual(k, l, &controls[u + 2 * v])?
                        * Q::from_integer((scale_u * scale_v).into());
                }
            }
            if !difference.is_zero() {
                return None;
            }
        }
    }
    Some(slopes)
}

/// Degree-one evaluator of exactly the same spatial graph in the same UV clock.
pub(crate) fn equivalent_affine_surface(s: &Surface) -> Option<Surface> {
    let w = *s.weights.first()?.first()?;
    if s.weights.iter().flatten().all(|&weight| weight == w) {
        return None;
    }
    certify_affine_parameterization(s)
}

/// Prove a clamped single-span spatial surface is affine in its original UV
/// coordinates. The returned bilinear representation has the same domain and
/// evaluates to the same point for every parameter in that domain.
pub fn certify_affine_parameterization(s: &Surface) -> Option<Surface> {
    admit(s).ok()?;
    for coordinate in 0..3 {
        affine_row(s, coordinate)?;
        let [p, q] = [s.degree_u, s.degree_v];
        let get = |i: usize, j: usize| Q::from_float(s.control_points[i][j][coordinate]);
        if get(0, 0)? + get(p, q)? != get(0, q)? + get(p, 0)? {
            return None;
        }
    }
    affine_corners(s)
}

/// Materialize the four corners only after proving the entire affine map.
pub(crate) fn affine_corners(s: &Surface) -> Option<Surface> {
    let [u, v] = [s.control_points.len() - 1, s.control_points[0].len() - 1];
    let [u0, u1] = [s.knots_u[s.degree_u], s.knots_u[u + 1]];
    let [v0, v1] = [s.knots_v[s.degree_v], s.knots_v[v + 1]];
    let mut out = s.clone();
    out.degree_u = 1;
    out.degree_v = 1;
    out.knots_u = vec![u0, u0, u1, u1];
    out.knots_v = vec![v0, v0, v1, v1];
    out.control_points = [0, u].map(|i| [0, v].map(|j| s.control_points[i][j].clone()).to_vec()).to_vec();
    out.weights = vec![vec![1.; 2]; 2];
    out.validate().ok()?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_projection_survives_axis_permutation_and_reversal() {
        let mut surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            periodic_u: false,
            periodic_v: false,
            control_points: [0., 0.25, 1.]
                .map(|x| vec![vec![0., 0., x], vec![0., -1., x]])
                .to_vec(),
            weights: vec![vec![1.; 2], vec![2.; 2], vec![3.; 2]],
        };
        assert_eq!(projection(&surface), Some([1, 2]));
        assert!(frame_on(&surface, [0, 1]).is_err());
        assert!(frame_on(&surface, [2, 1]).is_ok());
        for point in surface.control_points.iter_mut().flatten() {
            point[0] = point[1] + point[2];
            point[1] -= point[2];
        }
        assert_eq!(projection(&surface), Some([0, 1]));
        assert!(frame_on(&surface, [0, 1]).is_err());
        surface.control_points[1][0][2] += 0.125;
        assert_eq!(projection(&surface), Some([0, 1]));
        surface.control_points[1][0][0] += 0.125;
        assert_eq!(projection(&surface), None);
    }
}
