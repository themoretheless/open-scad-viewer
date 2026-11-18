//! Exact factorization of a periodic tensor carrier into an angular curve
//! and a meridian. Curve coordinates are stored only if the exact factors
//! are representable. Identifying a factorization grants no embedding credit.
use crate::{Result, check, curve::Curve, surface::Surface};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, ToPrimitive, Zero};

pub struct RadialTensor {
    /// Coordinates in the two world axes other than `height_axis`.
    pub center: [f64; 2],
    pub height_axis: usize,
    /// Center-relative planar coordinates at unit meridian scale.
    pub angular: Curve,
    /// Positive radial scale and world height, in that order.
    pub meridian: Curve,
}

fn exact(value: Q) -> Option<f64> {
    let f = value.to_f64()?;
    (Q::from_float(f).as_ref() == Some(&value)).then_some(f)
}

/// The original map is `(center + scale(v) * angular(u), height(v))`,
/// with the coordinates placed in the selected world axes. Each original
/// tensor weight is the product of the two recovered curve weights.
pub fn factor_radial_tensor(
    surface: &Surface,
    periodic_axis: usize,
    height_axis: usize,
    max_cells: usize,
) -> Result<Option<RadialTensor>> {
    surface.validate()?;
    check(
        periodic_axis < 2 && height_axis < 3,
        "Invalid radial tensor axes",
    )?;
    let flags = [surface.periodic_u, surface.periodic_v];
    if !flags[periodic_axis] || surface.control_points[0][0].len() != 3 {
        return Ok(None);
    }
    let counts = [
        surface.control_points.len(),
        surface.control_points[0].len(),
    ];
    let [n, m] = [counts[periodic_axis], counts[1 - periodic_axis]];
    if n.checked_mul(m).is_none_or(|cells| cells > max_cells) {
        return Ok(None);
    }
    Ok(factor(surface, periodic_axis, height_axis, n, m))
}

fn factor(s: &Surface, axis: usize, height: usize, n: usize, m: usize) -> Option<RadialTensor> {
    let q = |value| Q::from_float(value).unwrap();
    let index = |i: usize, j: usize| if axis == 0 { [i, j] } else { [j, i] };
    let pole = |i, j, k: usize| {
        let [u, v] = index(i, j);
        q(s.control_points[u][v][k])
    };
    let weight = |i, j| {
        let [u, v] = index(i, j);
        q(s.weights[u][v])
    };
    let plane = [(height + 1) % 3, (height + 2) % 3];
    let (i, k) = (1..n)
        .flat_map(|i| plane.map(|k| (i, k)))
        .find(|&(i, k)| pole(i, 0, k) != pole(0, 0, k))?;
    let base = pole(i, 0, k) - pole(0, 0, k);
    let scale = (0..m)
        .map(|j| (pole(i, j, k) - pole(0, j, k)) / &base)
        .collect::<Vec<_>>();
    if scale.iter().any(|r| *r <= Q::zero()) {
        return None;
    }
    let varied = scale.iter().position(|r| !r.is_one())?;
    let center = plane.map(|k| {
        (pole(0, varied, k) - &scale[varied] * pole(0, 0, k)) / (Q::one() - &scale[varied])
    });
    let w00 = weight(0, 0);
    for (j, scale) in scale.iter().enumerate() {
        for i in 0..n {
            if pole(i, j, height) != pole(0, j, height)
                || weight(i, j) * &w00 != weight(i, 0) * weight(0, j)
                || plane
                    .iter()
                    .zip(&center)
                    .any(|(&k, c)| pole(i, j, k) - c != scale * (pole(i, 0, k) - c))
            {
                return None;
            }
        }
    }
    let curve = |axis, control_points, weights| {
        let (degree, knots, periodic) = if axis == 0 {
            (s.degree_u, s.knots_u.clone(), s.periodic_u)
        } else {
            (s.degree_v, s.knots_v.clone(), s.periodic_v)
        };
        Curve {
            degree,
            knots,
            periodic,
            control_points,
            weights,
        }
    };
    let encode = |gauge: &Q| {
        let angular = (0..n)
            .map(|i| {
                plane
                    .iter()
                    .zip(&center)
                    .map(|(&k, c)| exact((pole(i, 0, k) - c) / gauge))
                    .collect()
            })
            .collect::<Option<Vec<_>>>()?;
        let meridian = scale
            .iter()
            .enumerate()
            .map(|(j, r)| Some(vec![exact(r * gauge)?, exact(pole(0, j, height))?]))
            .collect::<Option<Vec<_>>>()?;
        Some((angular, meridian))
    };
    // Prefer a common exact angular gauge across meridian subintervals.
    // An exact original gauge remains usable when that normalization does
    // not have a binary64 representation.
    let anchor = (0..n).find_map(|i| {
        plane
            .iter()
            .zip(&center)
            .map(|(&k, c)| (pole(i, 0, k) - c).abs())
            .find(|v| !v.is_zero())
    })?;
    let (angular, meridian) = encode(&anchor).or_else(|| encode(&Q::one()))?;
    let angular = curve(
        axis,
        angular,
        (0..n).map(|i| exact(weight(i, 0))).collect::<Option<_>>()?,
    );
    let meridian = curve(
        1 - axis,
        meridian,
        (0..m)
            .map(|j| exact(weight(0, j) / &w00))
            .collect::<Option<_>>()?,
    );
    Some(RadialTensor {
        center: [exact(center[0].clone())?, exact(center[1].clone())?],
        height_axis: height,
        angular,
        meridian,
    })
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AngularRingReport {
    /// Exactly one visit to each positive ray and nonzero angular speed on
    /// every original span, with exact closure at the authored seam.
    pub proven: bool,
    pub cells: usize,
    pub pairs: usize,
}

/// A simple closed rational curve with a strict, common angular derivative
/// sign winds exactly once about the origin. Homogeneous Bernstein controls
/// prove the derivative sign over each whole original span.
pub fn certify_angular_ring(
    curve: &Curve,
    max_cells: usize,
    max_pairs: usize,
) -> Result<AngularRingReport> {
    curve.validate()?;
    check(
        max_cells <= 100000 && max_pairs <= 100000,
        "Invalid angular ring budget",
    )?;
    let mut report = AngularRingReport::default();
    if !curve.periodic || curve.degree == 0 || curve.control_points[0].len() != 2 || max_pairs == 0
    {
        return Ok(report);
    }
    let scan = curve.knots.len().saturating_mul(curve.control_points.len());
    if scan >= max_cells {
        return Ok(report);
    }
    report.cells = scan;
    if !crate::curve_partition_agreement::verify_source_closure(curve)? {
        return Ok(report);
    }
    let Some(spans) = crate::curve_partition_agreement::exact_bezier_span_partition(curve)? else {
        return Ok(report);
    };
    if !(2..=256).contains(&spans.len()) {
        return Ok(report);
    }
    let mut direction = None;
    for span in &spans {
        let work = span.control_points.len().saturating_mul(span.degree);
        if work >= max_cells - report.cells {
            return Ok(report);
        }
        report.cells += work;
        let Some(sign) = angular_span_direction(span) else {
            return Ok(report);
        };
        if direction
            .replace(sign)
            .is_some_and(|previous| previous != sign)
        {
            return Ok(report);
        }
    }
    let simple =
        crate::trim_simplicity::inspect(&spans, 1e-10, max_pairs, max_cells - report.cells)?;
    report.cells += simple.cells;
    report.pairs = simple.pairs.len();
    report.proven = simple.proven_simple;
    Ok(report)
}

fn angular_span_direction(span: &Curve) -> Option<bool> {
    let q = |x| Q::from_float(x).unwrap();
    let pairs = span
        .control_points
        .iter()
        .zip(&span.weights)
        .map(|(p, &w)| [q(p[0]) * q(w), q(p[1]) * q(w)])
        .collect::<Vec<_>>();
    let det = |a: &[Q; 2], b: &[Q; 2]| &a[0] * &b[1] - &a[1] * &b[0];
    let start = det(&pairs[0], &pairs[1]);
    let end = det(&pairs[pairs.len() - 2], pairs.last()?);
    if start.is_zero() || end.is_zero() || start.is_positive() != end.is_positive() {
        return None;
    }
    let positive = start.is_positive();
    for edge in pairs.windows(2) {
        let d = [&edge[1][0] - &edge[0][0], &edge[1][1] - &edge[0][1]];
        if pairs.iter().any(|p| {
            let sign = det(p, &d);
            !sign.is_zero() && sign.is_positive() != positive
        }) {
            return None;
        }
    }
    Some(positive)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RadialBandPairReport {
    pub proven: bool,
    pub cells: usize,
    pub pairs: usize,
}

/// Complete source-carrier embedding for two compatible radial bands.
/// The only contacts are corresponding azimuths at the two opposite
/// meridian joins. The caller must still bind those joins and periodic
/// shores to the actual trim loops, world edges, and shell incidence.
pub fn certify_radial_band_pair(
    surfaces: [&Surface; 2],
    periodic_axes: [usize; 2],
    height_axis: usize,
    max_cells: usize,
    max_pairs: usize,
) -> Result<RadialBandPairReport> {
    certify_radial_band_cycle(&surfaces, &periodic_axes, height_axis, max_cells, max_pairs)
}

/// An ordered cycle of exact original bands. The simple closed meridian
/// proves only cyclic-neighbor contacts. BRep ownership of rims and shores
/// remains a separate obligation at the actual topology entry point.
pub fn certify_radial_band_cycle(
    surfaces: &[&Surface],
    periodic_axes: &[usize],
    height_axis: usize,
    max_cells: usize,
    max_pairs: usize,
) -> Result<RadialBandPairReport> {
    check(
        (2..=256).contains(&surfaces.len()) && surfaces.len() == periodic_axes.len(),
        "Radial band cycle requires matching surface and axis sequences",
    )?;
    check(
        max_cells <= 100000 && max_pairs <= 100000,
        "Invalid radial band budget",
    )?;
    let mut out = RadialBandPairReport::default();
    let mut factors = Vec::new();
    for (&surface, &axis) in surfaces.iter().zip(periodic_axes) {
        surface.validate()?;
        let cost = surface
            .control_points
            .len()
            .saturating_mul(surface.control_points[0].len());
        if cost > max_cells - out.cells {
            return Ok(out);
        }
        out.cells += cost;
        let Some(factor) = factor_radial_tensor(surface, axis, height_axis, cost)? else {
            return Ok(out);
        };
        factors.push(factor);
    }
    let left = &factors[0];
    if factors[1..].iter().any(|other| {
        left.center != other.center || !same_angular_curve(&left.angular, &other.angular)
    }) {
        return Ok(out);
    }
    let angular = certify_angular_ring(&left.angular, max_cells - out.cells, max_pairs)?;
    out.cells += angular.cells;
    out.pairs += angular.pairs;
    if !angular.proven {
        return Ok(out);
    }
    let mut meridians = Vec::new();
    for factor in factors {
        let meridian = factor.meridian;
        if meridian.periodic {
            return Ok(out);
        }
        let regularity = crate::curve_regularity::inspect(&meridian, max_cells - out.cells)?;
        out.cells += regularity.cells;
        if !regularity.spanwise_regular {
            return Ok(out);
        }
        let cost = meridian
            .knots
            .len()
            .saturating_mul(meridian.control_points.len());
        if cost > max_cells - out.cells {
            return Ok(out);
        }
        out.cells += cost;
        let Some(spans) = crate::curve_partition_agreement::exact_bezier_span_partition(&meridian)?
        else {
            return Ok(out);
        };
        meridians.extend(spans);
    }
    if !(2..=256).contains(&meridians.len()) || out.cells == max_cells || out.pairs == max_pairs {
        return Ok(out);
    }
    let meridian = crate::trim_simplicity::inspect(
        &meridians,
        1e-10,
        max_pairs - out.pairs,
        max_cells - out.cells,
    )?;
    out.cells += meridian.cells;
    out.pairs += meridian.pairs.len();
    out.proven = meridian.proven_simple;
    Ok(out)
}

fn same_angular_curve(a: &Curve, b: &Curve) -> bool {
    a.degree == b.degree
        && a.knots == b.knots
        && a.control_points == b.control_points
        && a.weights.iter().zip(&b.weights).all(|(&u, &v)| {
            crate::exact_products::sum_sign(&[(u, b.weights[0]), (-v, a.weights[0])])
                == std::cmp::Ordering::Equal
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band<const N: usize>(meridian: [[f64; 2]; N]) -> Surface {
        let angular = [
            [1., -1.],
            [1., 0.],
            [1., 1.],
            [0., 1.],
            [-1., 1.],
            [-1., 0.],
            [-1., -1.],
            [0., -1.],
            [1., -1.],
            [1., 0.],
        ];
        let w = [0.5, 1., 0.5, 1., 0.5, 1., 0.5, 1., 0.5, 1.];
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![-1., -1., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 5.],
            knots_v: if N == 3 {
                vec![0., 0., 0., 1., 1., 1.]
            } else {
                vec![0., 0., 0., 1., 1., 2., 2., 2.]
            },
            control_points: angular
                .iter()
                .map(|&[x, y]| {
                    meridian
                        .iter()
                        .map(|&[r, z]| vec![x * r, y * r, z])
                        .collect()
                })
                .collect(),
            weights: w
                .iter()
                .map(|&u| {
                    (0..N)
                        .map(|i| if i % 2 == 0 { u } else { u * 0.5 })
                        .collect()
                })
                .collect(),
            periodic_u: true,
            periodic_v: false,
        }
    }

    #[test]
    fn radial_band_pair_requires_a_simple_regular_closed_meridian() {
        let upper = band([[5., 0.], [5., 1.], [4., 1.], [3., 1.], [3., 0.]]);
        let lower = band([[3., 0.], [3., -1.], [4., -1.], [5., -1.], [5., 0.]]);
        let report = certify_radial_band_pair([&upper, &lower], [0; 2], 2, 10000, 100).unwrap();
        assert!(report.proven, "{report:?}");
        let short = certify_radial_band_pair([&upper, &lower], [0; 2], 2, 99, 100).unwrap();
        assert!(!short.proven);
        let mut disconnected = lower;
        for point in disconnected.control_points.iter_mut().flatten() {
            point[2] += 0.25;
        }
        assert!(
            !certify_radial_band_pair([&upper, &disconnected], [0; 2], 2, 10000, 100)
                .unwrap()
                .proven
        );
    }

    #[test]
    fn four_exact_meridian_arcs_form_one_radial_shell_cycle() {
        let bands = [
            band([[5., 0.], [5., 1.], [4., 1.]]),
            band([[4., 1.], [3., 1.], [3., 0.]]),
            band([[3., 0.], [3., -1.], [4., -1.]]),
            band([[4., -1.], [5., -1.], [5., 0.]]),
        ];
        let [a, b, c, d] = &bands;
        let good = certify_radial_band_cycle(&[a, b, c, d], &[0; 4], 2, 10000, 100).unwrap();
        assert!(good.proven, "{good:?}");
        assert!(
            !certify_radial_band_cycle(&[a, c, b, d], &[0; 4], 2, 10000, 100)
                .unwrap()
                .proven
        );
        assert!(
            !certify_radial_band_cycle(&[a, b, c, d], &[0; 4], 2, 10000, 1)
                .unwrap()
                .proven
        );
    }
}
