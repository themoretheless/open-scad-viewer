//! Native adapter for exact-expansion planar scalar root reductions.
//! Root counts and multiplicities concern the represented input definitions;
//! returned trim parameters are numerical observations within those enclosures.
use crate::{Result, check, curve::Curve};
use cad_predicates::{
    AuthoredScalar, Limits, PlanarReduction, PlanarScalarOutcome, PredicateContext, SourceArena,
    ToleranceContext, planar_bezier_scalar_roots,
};
#[derive(Clone, Debug)]
pub struct Point {
    pub first: f64,
    pub second: f64,
    pub multiplicity: usize,
}
#[derive(Clone, Debug)]
pub enum Relation {
    Points(Vec<Point>),
    Coincident {
        reversed: bool,
        linear: bool,
    },
    Overlap {
        points: Vec<[f64; 2]>,
        linear: bool,
        first: [f64; 2],
        second: [f64; 2],
        reversed: bool,
    },
}
pub(super) fn admit(c: &Curve) -> bool {
    (1..=3).contains(&c.degree)
        && !c.periodic
        && c.control_points.len() == c.degree + 1
        && c.control_points.iter().all(|p| p.len() == 2)
        && c.knots[..=c.degree].iter().all(|t| *t == c.domain()[0])
        && c.knots[c.degree + 1..].iter().all(|t| *t == c.domain()[1])
}
fn parameter(c: &Curve, t: f64) -> f64 {
    let [a, b] = c.domain();
    a + t * (b - a)
}
fn map_back(c: &Curve, t: f64, reversed: bool) -> f64 {
    if reversed {
        let [a, b] = c.domain();
        a + (b - t)
    } else {
        t
    }
}
pub fn monotone(c: &Curve, k: usize) -> bool {
    if k >= 2 || c.control_points.is_empty() || c.control_points.iter().any(|p| p.len() != 2) {
        return false;
    }
    let a = c.control_points[0][k];
    let b = c.control_points.last().unwrap()[k];
    if a == b {
        return false;
    }
    if c.control_points.windows(2).all(|p| {
        if b > a {
            p[1][k] >= p[0][k]
        } else {
            p[1][k] <= p[0][k]
        }
    }) {
        return true;
    }
    // Control-polygon monotonicity is sufficient, but not necessary. An
    // exact derivative root census admits positive derivatives with mixed
    // Bernstein coefficients, and stationary roots of even multiplicity.
    match stationary_definition(c, k).ok().flatten() {
        Some(cad_predicates::PlanarStationaryOutcome::Roots(roots)) => roots.iter().all(|r| {
            r.multiplicity % 2 == 0
                || (r.interval[0] == r.interval[1] && (r.interval[0] == 0. || r.interval[0] == 1.))
        }),
        _ => false,
    }
}
pub fn invert(c: &Curve, k: usize, value: f64) -> Result<Option<f64>> {
    c.validate()?;
    check(
        k < 2 && value.is_finite() && c.control_points.iter().all(|p| p.len() == 2),
        "Invalid planar coordinate inversion",
    )?;
    let a = c.control_points[0][k];
    let b = c.control_points.last().unwrap()[k];
    if !monotone(c, k) || value < a.min(b) || value > a.max(b) {
        return Ok(None);
    }
    if value == a {
        return Ok(Some(c.domain()[0]));
    }
    if value == b {
        return Ok(Some(c.domain()[1]));
    }
    let mut lo = 0.;
    let mut hi = 1.;
    for _ in 0..64 {
        let mid = (lo + hi) * 0.5;
        if mid == lo || mid == hi {
            break;
        }
        let v = c.evaluate(parameter(c, mid))?.point[k];
        if (v < value) == (b > a) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(Some(parameter(c, (lo + hi) * 0.5)))
}
fn residual(a: &Curve, b: &Curve, t: f64, u: f64) -> Result<f64> {
    let p = a.evaluate(t)?.point;
    let q = b.evaluate(u)?.point;
    Ok((p[0] - q[0]).hypot(p[1] - q[1]))
}
fn carrier_overlap(a: &Curve, b: &Curve, tolerance: f64) -> Result<Option<Relation>> {
    fn ends(c: &Curve) -> (&[f64], &[f64]) {
        (&c.control_points[0], c.control_points.last().unwrap())
    }
    let (aa, az) = ends(a);
    let (bb, bz) = ends(b);
    let axis = usize::from((az[1] - aa[1]).abs() > (az[0] - aa[0]).abs());
    if !monotone(a, axis) || !monotone(b, axis) {
        return Ok(None);
    }
    let lo = aa[axis].min(az[axis]).max(bb[axis].min(bz[axis]));
    let hi = aa[axis].max(az[axis]).min(bb[axis].max(bz[axis]));
    if hi < lo {
        return Ok(Some(Relation::Points(Vec::new())));
    }
    let Some(t0) = invert(a, axis, lo)? else {
        return Ok(None);
    };
    let Some(t1) = invert(a, axis, hi)? else {
        return Ok(None);
    };
    let Some(u0) = invert(b, axis, lo)? else {
        return Ok(None);
    };
    let Some(u1) = invert(b, axis, hi)? else {
        return Ok(None);
    };
    if residual(a, b, t0, u0)? > tolerance || residual(a, b, t1, u1)? > tolerance {
        return Ok(None);
    }
    if lo == hi {
        return Ok(Some(Relation::Points(vec![Point {
            first: t0,
            second: u0,
            multiplicity: 1,
        }])));
    }
    let first = [t0.min(t1), t0.max(t1)];
    let second = [u0.min(u1), u0.max(u1)];
    let reversed = (az[axis] > aa[axis]) != (bz[axis] > bb[axis]);
    if first == a.domain() && second == b.domain() {
        Ok(Some(Relation::Coincident {
            reversed,
            linear: true,
        }))
    } else {
        Ok(Some(Relation::Overlap {
            points: Vec::new(),
            linear: true,
            first,
            second,
            reversed,
        }))
    }
}
/// None requests the general two-parameter intersection driver. It never
/// denotes a disjoint pair or a discarded uncertain scalar root.
pub fn intersect(a: &Curve, b: &Curve, tolerance: f64) -> Result<Option<Relation>> {
    a.validate()?;
    b.validate()?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Positive planar contact tolerance required",
    )?;
    if !admit(a) || !admit(b) {
        return Ok(None);
    }
    let scale = a
        .control_points
        .iter()
        .chain(&b.control_points)
        .flatten()
        .map(|v| v.abs())
        .fold(1., f64::max);
    let ratio = |c: &Curve| {
        c.weights.iter().copied().fold(0., f64::max)
            / c.weights.iter().copied().fold(f64::INFINITY, f64::min)
    };
    let weight_ratio = ratio(a).max(ratio(b));
    let parameter_tolerance = (tolerance / (scale * 64. * weight_ratio * weight_ratio)).min(1e-12);
    if !parameter_tolerance.is_finite() || parameter_tolerance <= 0. {
        return Ok(None);
    }
    for reversed in [false, true] {
        let other = if reversed { b.reverse()? } else { b.clone() };
        let values = a
            .control_points
            .iter()
            .zip(&a.weights)
            .chain(other.control_points.iter().zip(&other.weights))
            .flat_map(|(p, w)| [p[0], p[1], *w])
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect();
        // These leaves are the current represented span definitions, including
        // their actual weights. No normalized/rounded control net is admitted.
        let source = SourceArena::authored("represented-planar-span-pair", 1, values)
            .map_err(|e| crate::input(e.to_string()))?;
        let refs = |start: usize, count: usize| {
            (start..start + count)
                .map(|i| std::array::from_fn(|k| source.leaf(i * 3 + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let context = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &context, Limits::default(), None);
        let decision = planar_bezier_scalar_roots(
            &mut ctx,
            &refs(0, a.control_points.len()),
            &refs(a.control_points.len(), other.control_points.len()),
            parameter_tolerance,
        )
        .map_err(|e| crate::input(e.to_string()))?;
        match decision.outcome {
            PlanarScalarOutcome::NotReducible | PlanarScalarOutcome::Indeterminate(_) => continue,
            PlanarScalarOutcome::Coincident {
                reduction: PlanarReduction::CommonCoordinate(_) | PlanarReduction::CommonProjection,
            } => {
                return Ok(Some(Relation::Coincident {
                    reversed,
                    linear: a.degree == 1 || b.degree == 1,
                }));
            }
            PlanarScalarOutcome::Coincident {
                reduction: PlanarReduction::LineCarrier { .. },
            } => return carrier_overlap(a, b, tolerance),
            PlanarScalarOutcome::Roots { reduction, roots } => {
                let mut points = Vec::new();
                for root in roots {
                    let s = (root.interval[0] + root.interval[1]) * 0.5;
                    let (first, second) = match reduction {
                        PlanarReduction::CommonCoordinate(_)
                        | PlanarReduction::CommonProjection => {
                            (parameter(a, s), map_back(b, parameter(&other, s), reversed))
                        }
                        PlanarReduction::LineCarrier { first } => {
                            let (line, curve) = if first { (a, &other) } else { (&other, a) };
                            let ct = parameter(curve, s);
                            let p = curve.evaluate(ct)?.point;
                            let start = &line.control_points[0];
                            let end = line.control_points.last().unwrap();
                            let axis =
                                usize::from((end[1] - start[1]).abs() > (end[0] - start[0]).abs());
                            if p[axis] < start[axis].min(end[axis]) - tolerance
                                || p[axis] > start[axis].max(end[axis]) + tolerance
                            {
                                continue;
                            }
                            let Some(lt) = invert(
                                line,
                                axis,
                                p[axis]
                                    .clamp(start[axis].min(end[axis]), start[axis].max(end[axis])),
                            )?
                            else {
                                return Ok(None);
                            };
                            if first {
                                (lt, map_back(b, ct, reversed))
                            } else {
                                (ct, map_back(b, lt, reversed))
                            }
                        }
                    };
                    if residual(a, b, first, second)? > tolerance {
                        return Ok(None);
                    }
                    points.push(Point {
                        first,
                        second,
                        multiplicity: root.multiplicity,
                    });
                }
                return Ok(Some(Relation::Points(points)));
            }
        }
    }
    super::planar_coincidence::overlap(a, b, tolerance)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cubic(y: [f64; 4]) -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: (0..4).map(|i| vec![i as f64, y[i]]).collect(),
            weights: vec![1.; 4],
            periodic: false,
        }
    }
    #[test]
    fn resolves_interior_second_and_third_order_contacts_without_chords() {
        let zero = cubic([0.; 4]);
        for (y, order) in [([3., -1., -1., 3.], 2), ([-1., 1., -1., 1.], 3)] {
            let other = cubic(y);
            let Some(Relation::Points(events)) = intersect(&zero, &other, 1e-7).unwrap() else {
                panic!("Missing scalar reduction")
            };
            assert_eq!(events.len(), 1);
            assert!((events[0].first - 0.5).abs() < 1e-10);
            assert_eq!(events[0].multiplicity, order);
        }
    }
    #[test]
    fn line_curve_tangency_has_rational_line_parameters_and_finite_segment_ownership() {
        let mut line = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
        line.weights = vec![1., 2.];
        let curve = cubic([3., -1., -1., 3.]);
        let Some(Relation::Points(events)) = intersect(&line, &curve, 1e-7).unwrap() else {
            panic!("Missing line reduction")
        };
        assert_eq!(events.len(), 1);
        assert!((events[0].first - 1. / 3.).abs() < 1e-10);
        assert_eq!(events[0].multiplicity, 2);
        let short = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        assert!(
            matches!(intersect(&short,&curve,1e-7).unwrap(),Some(Relation::Points(p)) if p.is_empty())
        );
    }
    #[test]
    fn coincident_reversed_and_scaled_weight_definitions_keep_their_identity() {
        let a = cubic([0., 1., 2., 1.]);
        let mut b = a.reverse().unwrap();
        b.weights = vec![8.; 4];
        assert!(matches!(
            intersect(&a, &b, 1e-7).unwrap(),
            Some(Relation::Coincident { reversed: true, .. })
        ));
    }
}

/// Root census of the actual source, including derivative multiplicities.
fn stationary_definition(
    c: &Curve,
    axis: usize,
) -> Result<Option<cad_predicates::PlanarStationaryOutcome>> {
    c.validate()?;
    check(axis < 2, "Planar stationary axis must be X or Y")?;
    if !admit(c) {
        return Ok(None);
    }
    let values = c
        .control_points
        .iter()
        .zip(&c.weights)
        .flat_map(|(p, w)| [p[0], p[1], *w])
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let source = SourceArena::authored("represented-planar-stationary", 1, values)
        .map_err(|e| crate::input(e.to_string()))?;
    let refs = (0..c.control_points.len())
        .map(|i| std::array::from_fn(|k| source.leaf(i * 3 + k).unwrap()))
        .collect::<Vec<_>>();
    let context = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&source, &context, Limits::default(), None);
    let decision = cad_predicates::planar_bezier_stationary_roots(&mut ctx, &refs, axis, 1e-14)
        .map_err(|e| crate::input(e.to_string()))?;
    Ok(Some(decision.outcome))
}
/// Complete represented-definition stationary parameters; None means the
/// predicate did not establish completeness and must not become an empty list.
pub fn stationary(c: &Curve, axis: usize) -> Result<Option<Vec<f64>>> {
    let Some(outcome) = stationary_definition(c, axis)? else {
        return Ok(None);
    };
    match outcome {
        cad_predicates::PlanarStationaryOutcome::Constant => Ok(Some(Vec::new())),
        cad_predicates::PlanarStationaryOutcome::Indeterminate(_) => Ok(None),
        cad_predicates::PlanarStationaryOutcome::Roots(roots) => Ok(Some(
            roots
                .into_iter()
                .filter_map(|r| {
                    let t = (r.interval[0] + r.interval[1]) * 0.5;
                    (t > 0. && t < 1.).then(|| parameter(c, t))
                })
                .collect(),
        )),
    }
}
#[cfg(test)]
mod stationary_tests {
    use super::*;
    #[test]
    fn rotated_curved_tangency_keeps_its_multiplicity_and_reversed_parameter() {
        let curve = |ys: [f64; 4]| Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: (0..4)
                .map(|i| vec![17. + 10. * i as f64 - ys[i], -9. + 10. * i as f64 + ys[i]])
                .collect(),
            weights: vec![1.; 4],
            periodic: false,
        };
        let a = curve([0., 0., 1., 3.]);
        let b = curve([1., -1., 1., 7.]).reverse().unwrap();
        let Some(Relation::Points(points)) = intersect(&a, &b, 1e-7).unwrap() else {
            panic!("Missing rotated contact")
        };
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].multiplicity, 2);
        assert!((points[0].first - 1. / 3.).abs() < 1e-11);
        assert!((points[0].second - 2. / 3.).abs() < 1e-11);
    }
    #[test]
    fn exact_monotonicity_does_not_require_an_ordered_control_polygon() {
        let mut c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.1], vec![0.9, 0.8], vec![1.9, 2.1]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let before = c.clone();
        for axis in 0..2 {
            assert!(c.control_points[1][axis] > c.control_points[2][axis]);
            assert!(stationary(&c, axis).unwrap().unwrap().is_empty());
            assert!(monotone(&c, axis));
        }
        let b = c.trim(0.25, 0.75).unwrap().reverse().unwrap();
        assert!(matches!(
            intersect(&c, &b, 1e-7).unwrap(),
            Some(Relation::Overlap { .. })
        ));
        assert_eq!(c.control_points, before.control_points);
        assert_eq!(c.weights, before.weights);
        // The proof must use the original rational weights: these weights
        // make the same control polygon reverse in both coordinates.
        c.weights = vec![1., 100., 100., 1.];
        assert!(!monotone(&c, 0));
        assert!(!monotone(&c, 1));
    }
    #[test]
    fn stationary_even_multiplicity_preserves_monotonicity_but_reversals_do_not() {
        let mut c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.], vec![-1., 2.], vec![3., 3.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(monotone(&c, 0));
        assert!(monotone(&c.reverse().unwrap(), 0));
        c.control_points[3][0] = 0.1;
        assert!(!monotone(&c, 0));
    }
    #[test]
    fn stationary_parameters_find_a_non_dyadic_cusp_without_mutating_its_definition() {
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![1., -1.], vec![-1., 2.], vec![0., -4.], vec![4., 8.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let before = c.control_points.clone();
        for axis in 0..2 {
            let roots = stationary(&c, axis).unwrap().unwrap();
            assert_eq!(roots.len(), 1);
            assert!((roots[0] - 1. / 3.).abs() < 1e-13);
        }
        assert_eq!(c.control_points, before);
    }
}
