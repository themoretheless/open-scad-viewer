//! Axis extrema and tight bounding boxes for rational curves.
//!
//! Each Bézier segment from [`Curve::decompose`] is reparameterized to the
//! local coordinate t in [0,1]. A coordinate stationary point satisfies
//! x'_axis = 0, whose numerator is the Bernstein polynomial
//! `dX·W − X·dW` of degree 2p−1 over the homogeneous control net. Roots are
//! isolated by Bernstein sign variation with de Casteljau subdivision, the
//! same strategy as `foundation::curve_projection`; sign variation cannot
//! discard a real root. Degree-one segments and fully degenerate (constant)
//! derivative forms yield no interior extrema. Tight bounds are the
//! control-polygon box shrunk to the evaluated extrema and endpoints, then
//! intersected with the control box so outward rounding never escapes the
//! convex hull.
use crate::{Result, bounds::Bounds, bounds::from_points, check, curve::Curve};
use math_core::{next_down, next_up};

/// Subdivision budget shared by every isolation run in this module tree.
pub(crate) const MAX_ISOLATION_CELLS: usize = 4096;

pub(crate) fn binomial(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    (0..k).fold(1., |value, i| value * (n - i) as f64 / (i + 1) as f64)
}

/// Bernstein coefficient product: c_k = Σ C(m,i)C(n,k−i)/C(m+n,k) a_i b_{k−i}.
pub(crate) fn bernstein_product(first: &[f64], second: &[f64]) -> Vec<f64> {
    let m = first.len() - 1;
    let n = second.len() - 1;
    (0..=m + n)
        .map(|k| {
            let start = k.saturating_sub(n);
            let end = k.min(m);
            (start..=end)
                .map(|i| {
                    binomial(m, i) * binomial(n, k - i) / binomial(m + n, k)
                        * first[i]
                        * second[k - i]
                })
                .sum()
        })
        .collect()
}

/// de Casteljau split at t = 1/2 into (left, right) coefficient rows.
pub(crate) fn bernstein_split(coefficients: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut rows = vec![coefficients.to_vec()];
    while rows.last().unwrap().len() > 1 {
        rows.push(
            rows.last()
                .unwrap()
                .array_windows()
                .map(|[a, b]| (a + b) * 0.5)
                .collect(),
        );
    }
    let left = rows.iter().map(|row| row[0]).collect();
    let right = rows.iter().rev().map(|row| *row.last().unwrap()).collect();
    (left, right)
}

/// de Casteljau evaluation at t in [0,1].
pub(crate) fn bernstein_value(coefficients: &[f64], t: f64) -> f64 {
    let mut row = coefficients.to_vec();
    while row.len() > 1 {
        row = row
            .array_windows()
            .map(|[a, b]| (1. - t) * a + t * b)
            .collect();
    }
    row[0]
}

/// Bernstein derivative coefficients: degree·(c_{i+1} − c_i).
pub(crate) fn bernstein_derivative(coefficients: &[f64]) -> Vec<f64> {
    let degree = coefficients.len() - 1;
    if degree == 0 {
        return vec![0.];
    }
    coefficients
        .array_windows()
        .map(|[a, b]| degree as f64 * (b - a))
        .collect()
}

pub(crate) fn sign_variations(coefficients: &[f64]) -> usize {
    let mut previous = 0_i8;
    let mut variations = 0;
    for &coefficient in coefficients {
        let sign = if coefficient > 0. {
            1
        } else if coefficient < 0. {
            -1
        } else {
            0
        };
        if sign != 0 {
            if previous != 0 && sign != previous {
                variations += 1;
            }
            previous = sign;
        }
    }
    variations
}

/// Scale by the largest absolute coefficient so products of high-degree forms
/// stay inside binary64 range; zeros and sign variations are unaffected.
pub(crate) fn normalize(coefficients: &mut [f64]) {
    let scale = coefficients
        .iter()
        .map(|value| value.abs())
        .fold(0., f64::max);
    if scale > 0. && scale.is_finite() {
        for value in coefficients.iter_mut() {
            *value /= scale;
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RootBox {
    pub lo: f64,
    pub hi: f64,
    pub variation: usize,
    coefficients: Vec<f64>,
}

/// Isolate roots of a Bernstein form on [0,1]. Returns (boxes, continuum):
/// the continuum flag marks an identically zero form, which carries no
/// isolated roots at all. Unresolved nodes at the subdivision budget are
/// reported as wide boxes rather than dropped, mirroring curve_projection.
pub(crate) fn isolate_roots(coefficients: &[f64], floor: f64) -> (Vec<RootBox>, bool) {
    if coefficients.iter().all(|value| *value == 0.) {
        return (vec![], true);
    }
    let floor = floor.min(2_f64.powi(-48)).max(2_f64.powi(-52));
    let mut pending = vec![RootBox {
        lo: 0.,
        hi: 1.,
        variation: sign_variations(coefficients),
        coefficients: coefficients.to_vec(),
    }];
    let mut roots = Vec::new();
    let mut work = 0_usize;
    while let Some(node) = pending.pop() {
        if node.variation == 0 {
            continue;
        }
        work += 1;
        if node.variation == 1 && node.hi - node.lo <= floor {
            roots.push(node);
            continue;
        }
        if work >= MAX_ISOLATION_CELLS {
            roots.push(node);
            continue;
        }
        let middle = (node.lo + node.hi) * 0.5;
        let (left, right) = bernstein_split(&node.coefficients);
        // Sign variation ignores zero coefficients. A root exactly on this
        // subdivision boundary would otherwise disappear from both children.
        if left.last() == Some(&0.) {
            roots.push(RootBox {
                lo: middle,
                hi: middle,
                variation: node.variation,
                coefficients: vec![0.],
            });
        }
        pending.push(RootBox {
            lo: middle,
            hi: node.hi,
            variation: sign_variations(&right),
            coefficients: right,
        });
        pending.push(RootBox {
            lo: node.lo,
            hi: middle,
            variation: sign_variations(&left),
            coefficients: left,
        });
    }
    roots.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    (roots, false)
}

/// Homogeneous Bézier control net: rows [w·x, w·y, (w·z), w].
pub(crate) fn homogeneous(curve: &Curve) -> Vec<Vec<f64>> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, weight)| {
            point
                .iter()
                .map(|coordinate| coordinate * weight)
                .chain(std::iter::once(*weight))
                .collect()
        })
        .collect()
}

/// Numerator of x'_axis(t) for a rational Bézier segment: dX·W − X·dW.
pub(crate) fn axis_stationary_coefficients(homogeneous: &[Vec<f64>], axis: usize) -> Vec<f64> {
    let degree = homogeneous.len() - 1;
    let dimension = homogeneous[0].len() - 1;
    let derivative: Vec<Vec<f64>> = homogeneous
        .array_windows()
        .map(|[a, b]| {
            a.iter()
                .zip(b)
                .map(|(x, y)| degree as f64 * (y - x))
                .collect()
        })
        .collect();
    let weight: Vec<f64> = homogeneous.iter().map(|row| row[dimension]).collect();
    let dweight: Vec<f64> = derivative.iter().map(|row| row[dimension]).collect();
    let coordinate: Vec<f64> = homogeneous.iter().map(|row| row[axis]).collect();
    let dcoordinate: Vec<f64> = derivative.iter().map(|row| row[axis]).collect();
    let mut numerator: Vec<f64> = bernstein_product(&dcoordinate, &weight)
        .into_iter()
        .zip(bernstein_product(&coordinate, &dweight))
        .map(|(a, b)| a - b)
        .collect();
    normalize(&mut numerator);
    numerator
}

/// Relative isolation floor derived from an absolute parameter tolerance.
pub(crate) fn relative_floor(tolerance: f64, span: f64) -> f64 {
    (tolerance / span)
        .min(2_f64.powi(-48))
        .max(2_f64.powi(-52))
}

/// Sign of a Bernstein form near one end of its domain, sampled just inside
/// to step over exact endpoint zeros.
pub(crate) fn bernstein_end_sign(coefficients: &[f64], head: bool) -> i8 {
    for step in 1..=8 {
        let t = 2_f64.powi(-(step as i32) * 4);
        let t = if head { t } else { 1. - t };
        let value = bernstein_value(coefficients, t);
        if value != 0. {
            return if value > 0. { 1 } else { -1 };
        }
    }
    0
}

fn axis_extrema_with_floor(curve: &Curve, axis: usize, tolerance: f64) -> Result<Vec<f64>> {
    curve.validate()?;
    let dimension = curve.control_points[0].len();
    check(axis < dimension, "Axis index must be inside the curve dimension")?;
    let segments = curve.decompose()?;
    let forms: Vec<(f64, f64, Vec<f64>)> = segments
        .iter()
        .map(|segment| {
            let [a, b] = segment.domain();
            (
                a,
                b,
                axis_stationary_coefficients(&homogeneous(segment.definition()), axis),
            )
        })
        .collect();
    let mut parameters = Vec::new();
    for (a, b, coefficients) in &forms {
        let (roots, continuum) = isolate_roots(coefficients, relative_floor(tolerance, b - a));
        if continuum {
            continue;
        }
        for root in roots {
            parameters.push(a + (b - a) * (root.lo + root.hi) * 0.5);
        }
    }
    // Interior span junctions: the coordinate derivative can jump through a
    // sign change (or vanish on both sides) at a breakpoint, which is also a
    // coordinate extremum — e.g. the vertices of a multi-arc NURBS circle.
    for index in 1..forms.len() {
        let before = bernstein_end_sign(&forms[index - 1].2, false);
        let after = bernstein_end_sign(&forms[index].2, true);
        if before != 0 && after != 0 && before != after {
            parameters.push(forms[index].0);
        }
    }
    parameters.sort_by(f64::total_cmp);
    parameters.dedup_by(|last, next| (*next - *last).abs() <= tolerance.max(2_f64.powi(-40)));
    Ok(parameters)
}

/// Parameters inside the active domain where C'_axis = 0 or jumps through a
/// sign change at a span junction: one entry per isolated stationary point
/// of the coordinate function. Domain endpoints are never included; a
/// constant-along-axis form yields an empty result.
pub fn axis_extrema(curve: &Curve, axis: usize) -> Result<Vec<f64>> {
    axis_extrema_with_floor(curve, axis, 2_f64.powi(-24))
}

fn tight_box(curve: &Curve, tolerance: f64) -> Result<Bounds> {
    curve.validate()?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Tight-bounds tolerance must be positive and finite",
    )?;
    let dimension = curve.control_points[0].len();
    let [a, b] = curve.domain();
    let mut candidates = vec![a, b];
    for axis in 0..dimension {
        candidates.extend(axis_extrema_with_floor(curve, axis, tolerance)?);
    }
    let points = candidates
        .into_iter()
        .map(|u| curve.evaluate(u).map(|evaluation| evaluation.point))
        .collect::<Result<Vec<_>>>()?;
    let mut tight = from_points(&points)?;
    // Evaluation rounding stays inside the positive-weight control hull.
    let control = curve.bounds()?;
    for axis in 0..dimension {
        tight.min[axis] = next_down(tight.min[axis].max(control.min[axis]));
        tight.max[axis] = next_up(tight.max[axis].min(control.max[axis]));
    }
    Ok(tight)
}

/// Control-polygon AABB refined by all axis extrema: the box is the hull of
/// the curve evaluated at every stationary parameter and both endpoints,
/// intersected with the control box. Works for 2D and 3D control points.
pub fn tight_bounds(curve: &Curve, tolerance: f64) -> Result<Bounds> {
    tight_box(curve, tolerance)
}

/// Two-dimensional specialization of [`tight_bounds`]; rejects 3D curves.
pub fn tight_bounds_2d(curve: &Curve, tolerance: f64) -> Result<Bounds> {
    curve.validate()?;
    check(
        curve.control_points[0].len() == 2,
        "Tight 2D bounds require two-dimensional control points",
    )?;
    tight_box(curve, tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadratic(points: &[[f64; 2]]) -> Curve {
        let degree = points.len() - 1;
        Curve {
            degree,
            knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
            control_points: points.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; points.len()],
            periodic: false,
        }
    }

    #[test]
    fn parabola_tight_bounds_shrink_control_box_to_analytic_answer() {
        // y(t) = 2t(1−t) peaks at y = 1/2 for t = 1/2; x(t) = 2t is monotone.
        let curve = quadratic(&[[0., 0.], [1., 1.], [2., 0.]]);
        let control = curve.bounds().unwrap();
        assert_eq!(control.max[1], 1.);
        let extrema = axis_extrema(&curve, 1).unwrap();
        assert_eq!(extrema.len(), 1);
        assert!((extrema[0] - 0.5).abs() < 1e-12);
        assert!(axis_extrema(&curve, 0).unwrap().is_empty());
        let tight = tight_bounds(&curve, 1e-10).unwrap();
        assert!((tight.max[1] - 0.5).abs() < 1e-9);
        assert!(tight.min[1] > -1e-12 && tight.min[1] < 1e-12);
        assert!(tight.min[0] > -1e-12 && tight.max[0] < 2. + 1e-12);
        assert!((tight.max[0] - 2.).abs() < 1e-12);
        let flat = tight_bounds_2d(&curve, 1e-10).unwrap();
        assert_eq!(flat, tight);
    }

    /// Full NURBS circle: four rational quadratic arcs.
    fn circle() -> Curve {
        let w = 2_f64.sqrt() / 2.;
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
            control_points: vec![
                vec![1., 0.],
                vec![1., 1.],
                vec![0., 1.],
                vec![-1., 1.],
                vec![-1., 0.],
                vec![-1., -1.],
                vec![0., -1.],
                vec![1., -1.],
                vec![1., 0.],
            ],
            weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
            periodic: false,
        }
    }

    #[test]
    fn circle_reaches_unit_extrema_at_arc_junctions() {
        // The y extrema sit exactly on the arc junctions u = 1 and u = 3,
        // where the derivative vanishes on both sides of the breakpoint.
        let curve = circle();
        let extrema = axis_extrema(&curve, 1).unwrap();
        assert_eq!(extrema.len(), 2, "extrema: {extrema:?}");
        assert!((extrema[0] - 1.).abs() < 1e-9);
        assert!((extrema[1] - 3.).abs() < 1e-9);
        let tight = tight_bounds(&curve, 1e-12).unwrap();
        for axis in 0..2 {
            assert!((tight.max[axis] - 1.).abs() < 1e-10);
            assert!((tight.min[axis] + 1.).abs() < 1e-10);
        }
    }

    #[test]
    fn degree_one_line_has_no_interior_extrema_and_exact_bounds() {
        let line = Curve::from_polyline(vec![vec![0., 0.], vec![3., 4.]]).unwrap();
        assert!(axis_extrema(&line, 0).unwrap().is_empty());
        assert!(axis_extrema(&line, 1).unwrap().is_empty());
        let tight = tight_bounds(&line, 1e-9).unwrap();
        for axis in 0..2 {
            assert!((tight.min[axis] - [0., 0.][axis]).abs() < 1e-12);
            assert!((tight.max[axis] - [3., 4.][axis]).abs() < 1e-12);
        }
    }

    #[test]
    fn degenerate_segment_and_axis_validation_do_not_panic() {
        // All control points share y = 2: derivative form is identically zero.
        let flat = quadratic(&[[0., 2.], [1., 2.], [2., 2.]]);
        assert!(axis_extrema(&flat, 1).unwrap().is_empty());
        assert!(axis_extrema(&flat, 0).unwrap().is_empty());
        assert!(axis_extrema(&flat, 2).is_err());
        let solid = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 1., 0.], vec![1., 1., 1.], vec![1., 1., 2.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let tight = tight_bounds(&solid, 1e-9).unwrap();
        assert!((tight.min[0] - 1.).abs() < 1e-12 && (tight.min[1] - 1.).abs() < 1e-12);
        assert!(tight_bounds_2d(&solid, 1e-9).is_err());
    }
}
