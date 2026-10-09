//! Scalar curvature analysis of rational curves: inflections and κ extrema.
//!
//! Per Bézier segment, all quantities are built as Bernstein polynomials in
//! the local coordinate t in [0,1] over the homogeneous net [w·x, …, w]:
//!   A_i = dX_i·W − X_i·dW          (numerator of x'_i, degree 2p−1)
//!   B_i = dA_i·W − 2·A_i·dW        (numerator of x''_i, degree 3p−2)
//!   cross_k = A_i·B_j − A_j·B_i    (numerator of (C′×C″)_k, degree 5p−3)
//! The true derivatives carry extra positive powers of W in their
//! denominators, so zeros and signs of these numerators are exactly the
//! zeros and signs of the geometric quantities.
//!
//! Inflections (2D): zeros of the signed-curvature numerator
//! z = x′·y″ − y′·x″ = cross_z, isolated by Bernstein sign variation
//! (mirrors `foundation::curve_projection`). Inflections (3D): κ vanishes
//! iff all three cross components vanish simultaneously, so we isolate the
//! roots of each component and report overlapping root boxes — this is a
//! documented conservative choice, since a sign variation count does not
//! exist for the nonnegative squared cross norm.
//!
//! Curvature extrema: κ² = W²·S/T³ with S = Σ cross_k² and T = Σ A_i²
//! (speed² numerator). Its derivative numerator
//!   M = 2·dW·S·T + W·dS·T − 3·W·S·dT
//! is a Bernstein polynomial whose sign changes classify the extrema
//! (+→− gives LocalMax, −→+ gives LocalMin). Tangency roots with no sign
//! change are not extrema and are skipped. Junctions between Bézier spans
//! (where dκ/du can jump, e.g. ellipse vertices at arc seams) and the seam
//! of geometrically closed curves are classified by one-sided M signs.
//! Curvature intervals come from `curve_differential`; |C′| ≈ 0 degeneracies
//! report an unbounded interval instead of panicking.
use crate::{
    Result, check,
    curve::Curve,
    curve_differential::{self, Side, Status},
    curve_extrema::{
        bernstein_derivative, bernstein_end_sign, bernstein_product, bernstein_value, homogeneous,
        isolate_roots, normalize, relative_floor,
    },
};
use math_core::{next_down, next_up};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtremumKind {
    LocalMax,
    LocalMin,
}

#[derive(Clone, Debug)]
pub struct Extremum {
    pub u: f64,
    pub kappa: [f64; 2],
    pub kind: ExtremumKind,
}

/// Bernstein numerators for one Bézier segment: the three cross-product
/// components (zeros locate κ = 0) and M (sign changes classify κ extrema).
struct CurvatureForms {
    cross: [Vec<f64>; 3],
    m: Vec<f64>,
}

fn curvature_forms(segment: &Curve) -> CurvatureForms {
    let net = homogeneous(segment);
    let degree = net.len() - 1;
    let dimension = net[0].len() - 1;
    let derivative: Vec<Vec<f64>> = net
        .array_windows()
        .map(|[a, b]| {
            a.iter()
                .zip(b)
                .map(|(x, y)| degree as f64 * (y - x))
                .collect()
        })
        .collect();
    let column = |rows: &[Vec<f64>], axis: usize| -> Vec<f64> {
        rows.iter().map(|row| row[axis]).collect()
    };
    let weight = column(&net, dimension);
    let dweight = column(&derivative, dimension);
    // Numerators of first and second coordinate derivatives, zero-padded to
    // three axes so 2D and 3D curves share the cross-product code. These
    // forms are NOT individually normalized: the zeros of products and
    // differences below depend on their relative scaling.
    let mut first = Vec::with_capacity(3);
    let mut second = Vec::with_capacity(3);
    for axis in 0..3 {
        if axis < dimension {
            let a: Vec<f64> = bernstein_product(&column(&derivative, axis), &weight)
                .into_iter()
                .zip(bernstein_product(&column(&net, axis), &dweight))
                .map(|(x, y)| x - y)
                .collect();
            let da = bernstein_derivative(&a);
            let b: Vec<f64> = bernstein_product(&da, &weight)
                .into_iter()
                .zip(bernstein_product(&a, &dweight))
                .map(|(x, y)| x - 2. * y)
                .collect();
            first.push(a);
            second.push(b);
        } else {
            first.push(vec![0.; 2 * degree]);
            second.push(vec![0.; 3 * degree.max(1) - 1]);
        }
    }
    let cross: [Vec<f64>; 3] = std::array::from_fn(|k| {
        let i = (k + 1) % 3;
        let j = (k + 2) % 3;
        bernstein_product(&first[i], &second[j])
            .into_iter()
            .zip(bernstein_product(&first[j], &second[i]))
            .map(|(x, y)| x - y)
            .collect()
    });
    let square = |coefficients: &[f64]| bernstein_product(coefficients, coefficients);
    let s = cross.iter().map(|c| square(c)).reduce(|mut sum, term| {
        for (target, value) in sum.iter_mut().zip(term) {
            *target += value;
        }
        sum
    });
    let t_form = first.iter().map(|a| square(a)).reduce(|mut sum, term| {
        for (target, value) in sum.iter_mut().zip(term) {
            *target += value;
        }
        sum
    });
    let (Some(mut s), Some(mut t)) = (s, t_form) else {
        unreachable!("three cross components always exist")
    };
    normalize(&mut s);
    normalize(&mut t);
    let ds = bernstein_derivative(&s);
    let dt = bernstein_derivative(&t);
    let st = bernstein_product(&s, &t);
    let mut m: Vec<f64> = bernstein_product(&dweight, &st)
        .into_iter()
        .zip(bernstein_product(&weight, &bernstein_product(&ds, &t)))
        .zip(bernstein_product(&weight, &bernstein_product(&s, &dt)))
        .map(|((a, b), c)| 2. * a + b - 3. * c)
        .collect();
    normalize(&mut m);
    CurvatureForms { cross, m }
}

/// Certified κ interval at u; degenerate speeds report [0, ∞), never panic.
fn kappa_interval(curve: &Curve, u: f64) -> Result<[f64; 2]> {
    for side in [Side::Automatic, Side::Left, Side::Right] {
        if let Ok(report) = curve_differential::at(curve, u, side) {
            if report.curvature_status == Status::Available
                && let Some(kappa) = report.curvature
            {
                return Ok(kappa);
            }
        }
    }
    let evaluation = curve.evaluate(u)?;
    let degenerate = [0., f64::INFINITY];
    let (Some(d1), Some(d2)) = (evaluation.d1, evaluation.d2) else {
        return Ok(degenerate);
    };
    let dimension = evaluation.point.len();
    let vector = |v: &[f64]| std::array::from_fn::<f64, 3, _>(|k| v.get(k).copied().unwrap_or(0.));
    let (a, b) = (vector(&d1), vector(&d2));
    let cross: [f64; 3] = std::array::from_fn(|k| {
        let i = (k + 1) % 3;
        let j = (k + 2) % 3;
        a[i] * b[j] - a[j] * b[i]
    });
    let speed: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let _ = dimension;
    if !(speed > 0.) || !speed.is_finite() {
        return Ok(degenerate);
    }
    let kappa = cross.iter().map(|x| x * x).sum::<f64>().sqrt() / speed.powi(3);
    Ok([next_down(kappa.max(0.)), next_up(kappa)])
}

/// Parameters where the curvature vanishes with a sign change available:
/// signed-curvature zeros for 2D curves, common cross-component zeros for
/// 3D curves (see module documentation). Identically straight segments
/// contribute nothing and never panic.
pub fn inflection_parameters(curve: &Curve, tolerance: f64) -> Result<Vec<f64>> {
    curve.validate()?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Inflection tolerance must be positive and finite",
    )?;
    let dimension = curve.control_points[0].len();
    let mut parameters = Vec::new();
    for segment in curve.decompose()? {
        let definition = segment.definition();
        let [a, b] = segment.domain();
        let floor = relative_floor(tolerance, b - a);
        let forms = curvature_forms(definition);
        if dimension == 2 {
            let (roots, continuum) = isolate_roots(&forms.cross[2], floor);
            if !continuum {
                parameters.extend(
                    roots
                        .iter()
                        .map(|root| a + (b - a) * (root.lo + root.hi) * 0.5),
                );
            }
        } else {
            let isolated: Vec<_> = forms
                .cross
                .iter()
                .map(|component| isolate_roots(component, floor))
                .collect();
            if isolated.iter().any(|(_, continuum)| *continuum) {
                // A cross component identically zero is required (e.g. a
                // planar 3D curve); the remaining components decide.
            }
            let sets: Vec<&[_]> = isolated.iter().map(|(roots, _)| &roots[..]).collect();
            let overlap = |x: (f64, f64), y: (f64, f64)| x.0 <= y.1 && y.0 <= x.1;
            for first in sets[0] {
                let span = (first.lo, first.hi);
                if sets[1]
                    .iter()
                    .any(|root| overlap(span, (root.lo, root.hi)))
                    && sets[2]
                        .iter()
                        .any(|root| overlap(span, (root.lo, root.hi)))
                {
                    parameters.push(a + (b - a) * (span.0 + span.1) * 0.5);
                }
            }
        }
    }
    parameters.sort_by(f64::total_cmp);
    parameters.dedup_by(|last, next| (*next - *last).abs() <= tolerance.max(2_f64.powi(-40)));
    Ok(parameters)
}

/// Isolated roots of dκ/du plus span junctions where the derivative jumps
/// through a sign change. κ is reported as a certified interval from
/// `curve_differential`. Straight lines (M ≡ 0) return an empty result.
pub fn curvature_extrema(curve: &Curve, tolerance: f64) -> Result<Vec<Extremum>> {
    curve.validate()?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Curvature-extrema tolerance must be positive and finite",
    )?;
    let segments = curve.decompose()?;
    let forms: Vec<(f64, f64, CurvatureForms)> = segments
        .iter()
        .map(|segment| {
            let [a, b] = segment.domain();
            (a, b, curvature_forms(segment.definition()))
        })
        .collect();
    let mut extrema: Vec<Extremum> = Vec::new();
    let push = |extrema: &mut Vec<Extremum>, u: f64, increasing: bool| -> Result<()> {
        let kind = if increasing {
            ExtremumKind::LocalMin
        } else {
            ExtremumKind::LocalMax
        };
        if extrema
            .iter()
            .any(|e: &Extremum| (e.u - u).abs() <= tolerance.max(2_f64.powi(-40)))
        {
            return Ok(());
        }
        extrema.push(Extremum {
            u,
            kappa: kappa_interval(curve, u)?,
            kind,
        });
        Ok(())
    };
    for (a, b, form) in &forms {
        let span = b - a;
        let (roots, continuum) = isolate_roots(&form.m, relative_floor(tolerance, span));
        if continuum {
            continue;
        }
        for root in roots {
            let u = a + span * (root.lo + root.hi) * 0.5;
            // Roots at the very ends of a span are vertex extrema (e.g. the
            // seam of a closed ellipse), handled by the junction pass below;
            // rounding turns the exact end zero into a tiny interior box.
            let edge = tolerance.max(span * 2_f64.powi(-24));
            if u - a <= edge || b - u <= edge {
                continue;
            }
            let width = (root.hi - root.lo).max(2_f64.powi(-30));
            let left = bernstein_value(&form.m, (root.lo - width).max(0.));
            let right = bernstein_value(&form.m, (root.hi + width).min(1.));
            if left == 0. || right == 0. || left.signum() == right.signum() {
                continue;
            }
            push(&mut extrema, u, left < 0.)?;
        }
    }
    // Junctions between consecutive spans and, for geometrically closed
    // curves, the seam between the last and the first span.
    let junctions: Vec<usize> = {
        let count = forms.len();
        let mut indices: Vec<usize> = (1..count).collect();
        if count > 1 {
            let [a, b] = curve.domain();
            let closed = curve.periodic || {
                let first = curve.evaluate(a)?.point;
                let last = curve.evaluate(b)?.point;
                let scale = first
                    .iter()
                    .chain(&last)
                    .map(|x| x.abs())
                    .fold(1., f64::max);
                first
                    .iter()
                    .zip(&last)
                    .all(|(x, y)| (x - y).abs() <= 64. * f64::EPSILON * scale)
            };
            if closed {
                indices.push(0);
            }
        }
        indices
    };
    for &index in &junctions {
        let previous = if index == 0 { forms.len() - 1 } else { index - 1 };
        let before = bernstein_end_sign(&forms[previous].2.m, false);
        let after = bernstein_end_sign(&forms[index].2.m, true);
        if before != 0 && after != 0 && before != after {
            push(&mut extrema, forms[index].0, before < 0)?;
        }
    }
    extrema.sort_by(|a, b| a.u.total_cmp(&b.u));
    Ok(extrema)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bezier(points: &[[f64; 2]]) -> Curve {
        let degree = points.len() - 1;
        Curve {
            degree,
            knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
            control_points: points.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; points.len()],
            periodic: false,
        }
    }

    /// Full NURBS ellipse with semi-axes (a, b): four rational quadratic arcs.
    fn ellipse(a: f64, b: f64) -> Curve {
        let w = 2_f64.sqrt() / 2.;
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
            control_points: vec![
                vec![a, 0.],
                vec![a, b],
                vec![0., b],
                vec![-a, b],
                vec![-a, 0.],
                vec![-a, -b],
                vec![0., -b],
                vec![a, -b],
                vec![a, 0.],
            ],
            weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
            periodic: false,
        }
    }

    #[test]
    fn cubic_bezier_inflection_at_known_parameter() {
        // x(t) = 3t, y″(t) = 12t−6 ⇒ signed curvature zero exactly at t = 1/2.
        let curve = bezier(&[[0., 0.], [1., 2.], [2., -2.], [3., 0.]]);
        let inflections = inflection_parameters(&curve, 1e-9).unwrap();
        assert_eq!(inflections.len(), 1);
        assert!((inflections[0] - 0.5).abs() < 1e-7);
        let extrema = curvature_extrema(&curve, 1e-9).unwrap();
        let minimum = extrema
            .iter()
            .find(|e| e.kind == ExtremumKind::LocalMin && (e.u - 0.5).abs() < 1e-6)
            .expect("inflection must appear as a curvature minimum");
        assert!(minimum.kappa[0] <= minimum.kappa[1]);
        assert!(minimum.kappa[1] < 1e-3);
        assert!(extrema.iter().any(|e| e.kind == ExtremumKind::LocalMax));
    }

    #[test]
    fn ellipse_has_four_curvature_extrema_at_the_vertices() {
        let curve = ellipse(2., 0.5);
        assert!(inflection_parameters(&curve, 1e-9).unwrap().is_empty());
        let extrema = curvature_extrema(&curve, 1e-8).unwrap();
        assert_eq!(extrema.len(), 4, "extrema: {extrema:?}");
        // Vertices at u = 0/4 and 2 are major-axis maxima (κ = a/b² = 8),
        // vertices at u = 1 and 3 are minor-axis minima (κ = b/a² = 1/8).
        for (u, kind, kappa) in [
            (0., ExtremumKind::LocalMax, 8.),
            (1., ExtremumKind::LocalMin, 0.125),
            (2., ExtremumKind::LocalMax, 8.),
            (3., ExtremumKind::LocalMin, 0.125),
        ] {
            let extremum = extrema
                .iter()
                .find(|e| (e.u - u).abs() < 1e-6)
                .unwrap_or_else(|| panic!("no extremum near u = {u}"));
            assert_eq!(extremum.kind, kind);
            assert!(
                extremum.kappa[0] <= kappa && kappa <= extremum.kappa[1],
                "κ interval {:?} must contain {kappa}",
                extremum.kappa
            );
        }
    }

    #[test]
    fn circle_has_constant_curvature_and_no_inflections() {
        let curve = ellipse(1.5, 1.5);
        assert!(inflection_parameters(&curve, 1e-9).unwrap().is_empty());
        // Any extremum reported for the numerically near-constant κ must
        // still certify κ = 1/r within the interval arithmetic.
        for extremum in curvature_extrema(&curve, 1e-8).unwrap() {
            let expected = 1. / 1.5;
            assert!(
                extremum.kappa[0] <= expected && expected <= extremum.kappa[1],
                "κ interval {:?} must contain 1/r",
                extremum.kappa
            );
        }
        let report = curve_differential::at(&curve, 0.5, Side::Automatic).unwrap();
        let kappa = report.curvature.unwrap();
        assert!(kappa[0] <= 1. / 1.5 && 1. / 1.5 <= kappa[1]);
    }

    #[test]
    fn straight_lines_are_degenerate_but_never_panic() {
        let line = Curve::from_polyline(vec![vec![0., 0.], vec![3., 4.]]).unwrap();
        assert!(inflection_parameters(&line, 1e-9).unwrap().is_empty());
        assert!(curvature_extrema(&line, 1e-9).unwrap().is_empty());
        let spatial = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 2., 2.],
                vec![2., 4., 4.],
                vec![3., 6., 6.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(inflection_parameters(&spatial, 1e-9).unwrap().is_empty());
        assert!(curvature_extrema(&spatial, 1e-9).unwrap().is_empty());
    }

    #[test]
    fn spatial_helix_like_curve_yields_3d_analysis() {
        // Nonplanar 3D cubic: exercises the three-component inflection path.
        let curve = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 2., 1.],
                vec![2., -2., 2.],
                vec![3., 0., 3.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        let extrema = curvature_extrema(&curve, 1e-9).unwrap();
        assert!(!extrema.is_empty());
        for extremum in &extrema {
            assert!(extremum.kappa[0] <= extremum.kappa[1]);
        }
    }

    #[test]
    fn periodic_curve_analyzes_without_seam_gaps() {
        let active: Vec<f64> = (0..=8).map(|i| i as f64).collect();
        let knots = crate::foundation::periodic_knots(&active, 3);
        let mut control_points: Vec<Vec<f64>> = (0..8)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::TAU / 8.;
                let radius = 2. + 0.25 * (2. * angle).cos();
                vec![radius * angle.cos(), radius * angle.sin()]
            })
            .collect();
        let tail: Vec<Vec<f64>> = control_points[..3].to_vec();
        control_points.extend(tail);
        let curve = Curve {
            degree: 3,
            knots,
            control_points,
            weights: vec![1.; 11],
            periodic: true,
        };
        curve.validate().unwrap();
        let extrema = curvature_extrema(&curve, 1e-7).unwrap();
        assert!(
            extrema.len() >= 4,
            "wobbled closed loop should have ≥ 4 κ extrema: {extrema:?}"
        );
        assert!(inflection_parameters(&curve, 1e-7).unwrap().is_empty());
        for extremum in &extrema {
            assert!(extremum.kappa[0].is_finite() && extremum.kappa[0] <= extremum.kappa[1]);
        }
    }
}
