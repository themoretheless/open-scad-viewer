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
        first: [f64; 2],
        second: [f64; 2],
        reversed: bool,
    },
}
fn admit(c: &Curve) -> bool {
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
fn monotone(c: &Curve, k: usize) -> bool {
    let a = c.control_points[0][k];
    let b = c.control_points.last().unwrap()[k];
    a != b
        && c.control_points.windows(2).all(|p| {
            if b > a {
                p[1][k] >= p[0][k]
            } else {
                p[1][k] <= p[0][k]
            }
        })
}
fn invert(c: &Curve, k: usize, value: f64) -> Result<Option<f64>> {
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
                reduction: PlanarReduction::CommonCoordinate(_),
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
                        PlanarReduction::CommonCoordinate(_) => {
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
    Ok(None)
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
