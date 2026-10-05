//! Formal exact plane identity of an unchanged original Bezier composition.
//! Neither chart membership nor denominator positivity follows from this report.
use crate::{curve::Curve, surface::Surface, Error, Result};
use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
pub fn verify_algebraic(
    p: &Curve,
    s: &Surface,
    plane: [[f64; 3]; 3],
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    p.validate()?;
    s.validate()?;
    if p.control_points[0].len() != 2
        || plane.iter().flatten().any(|v| !v.is_finite())
        || max_work > cad_predicates::MAX_WORK
    {
        return Err(Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid original plane composition input",
        ));
    }
    let bezier = |knots: &[f64], degree: usize, n: usize| {
        n == degree + 1
            && knots[..=degree].iter().all(|&t| t == knots[degree])
            && knots[n..].iter().all(|&t| t == knots[n])
    };
    if p.periodic
        || s.periodic_u
        || s.periodic_v
        || p.degree > 8
        || s.degree_u > 8
        || s.degree_v > 8
        || !bezier(&p.knots, p.degree, p.control_points.len())
        || !bezier(&s.knots_u, s.degree_u, s.control_points.len())
        || !bezier(&s.knots_v, s.degree_v, s.control_points[0].len())
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for (pole, w) in p.control_points.iter().zip(&p.weights) {
        values.extend(pole.iter().copied());
        values.push(*w);
    }
    for (row, weights) in s.control_points.iter().zip(&s.weights) {
        for (pole, w) in row.iter().zip(weights) {
            values.extend(pole.iter().copied());
            values.push(*w);
        }
    }
    values.extend([
        s.knots_u[s.degree_u],
        s.knots_u[s.control_points.len()],
        s.knots_v[s.degree_v],
        s.knots_v[s.control_points[0].len()],
    ]);
    values.extend(plane.iter().flatten().copied());
    let source = SourceArena::authored(
        "original-plane-composition",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("NURBS_INVALID_INPUT", "Invalid plane source"))?;
    let mut index = 0;
    let mut leaf = || {
        let r = source.leaf(index).unwrap();
        index += 1;
        r
    };
    let pp: Vec<_> = p
        .control_points
        .iter()
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect();
    let ss: Vec<Vec<_>> = s
        .control_points
        .iter()
        .map(|row| {
            row.iter()
                .map(|_| std::array::from_fn(|_| leaf()))
                .collect()
        })
        .collect();
    let domain = std::array::from_fn(|_| std::array::from_fn(|_| leaf()));
    let plane = std::array::from_fn(|_| std::array::from_fn(|_| leaf()));
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    cad_predicates::rational_bezier_composition_plane_identity(&mut ctx, &pp, &ss, domain, plane)
        .map(Some)
        .map_err(|_| Error::new("NURBS_INVALID_INPUT", "Invalid plane identity request"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_predicates::BezierIdentity;
    #[test]
    fn nonlinear_oblique_plane_composition_preserves_original_sources_and_refuses_changes() {
        let plane = [[0.5, 0., 0.], [0.5, 1., 0.], [1.5, 0., 1.]];
        for domain in [[[0., 1.], [0., 1.]], [[2., 4.], [3., 7.]]] {
            let s = Surface {
                degree_u: 1,
                degree_v: 2,
                knots_u: vec![domain[0][0], domain[0][0], domain[0][1], domain[0][1]],
                knots_v: vec![domain[1][0]; 3]
                    .into_iter()
                    .chain(vec![domain[1][1]; 3])
                    .collect(),
                control_points: vec![
                    vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
                    vec![vec![1., 0., 1.], vec![1., 1., 1.], vec![0., 1., 1.]],
                ],
                weights: vec![vec![1.; 3]; 2],
                periodic_u: false,
                periodic_v: false,
            };
            let p = Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: [(0.4375, -0.25), (0.8125, 0.5), (-1.0625, 1.25)]
                    .iter()
                    .map(|&(u, v)| {
                        vec![
                            domain[0][0] + u * (domain[0][1] - domain[0][0]),
                            domain[1][0] + v * (domain[1][1] - domain[1][0]),
                        ]
                    })
                    .collect(),
                weights: vec![1.; 3],
                periodic: false,
            };
            assert_eq!(
                verify_algebraic(&p, &s, plane, 1_000_000)
                    .unwrap()
                    .unwrap()
                    .outcome,
                BezierIdentity::Equal
            );
            let mut reverse = p.clone();
            reverse.control_points.reverse();
            assert_eq!(
                verify_algebraic(&reverse, &s, [plane[0], plane[2], plane[1]], 1_000_000)
                    .unwrap()
                    .unwrap()
                    .outcome,
                BezierIdentity::Equal
            );
            let mut wrong = p.clone();
            wrong.control_points[1][0] += 1e-12;
            assert_eq!(
                verify_algebraic(&wrong, &s, plane, 1_000_000)
                    .unwrap()
                    .unwrap()
                    .outcome,
                BezierIdentity::Different
            );
            let mut weighted = p.clone();
            weighted.weights[1] = 2.;
            assert_eq!(
                verify_algebraic(&weighted, &s, plane, 1_000_000)
                    .unwrap()
                    .unwrap()
                    .outcome,
                BezierIdentity::Different
            );
            assert!(matches!(
                verify_algebraic(&p, &s, plane, 0).unwrap().unwrap().outcome,
                BezierIdentity::Indeterminate(_)
            ));
            assert!(matches!(
                verify_algebraic(&p, &s, [[0.; 3]; 3], 1_000_000)
                    .unwrap()
                    .unwrap()
                    .outcome,
                BezierIdentity::Indeterminate(_)
            ));
        }
    }
}

/// Exact plane membership at a normalized parameter from an unchanged source
/// domain. This is point membership only, not intersection-root uniqueness.
pub fn verify_curve_point(
    curve: &Curve,
    plane: [[f64; 3]; 3],
    parameter: [f64; 3],
    reversed: bool,
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    curve.validate()?;
    if curve.control_points[0].len() != 3
        || plane
            .iter()
            .flatten()
            .chain(parameter.iter())
            .any(|v| !v.is_finite())
        || max_work > cad_predicates::MAX_WORK
    {
        return Err(Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid exact curve plane point input",
        ));
    }
    let d = curve.domain();
    if curve.periodic
        || curve.degree > 32
        || curve.control_points.len() != curve.degree + 1
        || curve.knots[..=curve.degree].iter().any(|&k| k != d[0])
        || curve.knots[curve.control_points.len()..]
            .iter()
            .any(|&k| k != d[1])
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for (p, w) in curve.control_points.iter().zip(&curve.weights) {
        values.extend(p.iter().copied());
        values.push(*w);
    }
    values.extend(plane.iter().flatten().copied());
    values.extend(parameter);
    let source = SourceArena::authored(
        "original-curve-plane-point",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| {
        Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid original curve plane point source",
        )
    })?;
    let mut index = 0;
    let mut leaf = || {
        let r = source.leaf(index).unwrap();
        index += 1;
        r
    };
    let cc: Vec<_> = curve
        .control_points
        .iter()
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect();
    let plane = std::array::from_fn(|_| std::array::from_fn(|_| leaf()));
    let parameter = std::array::from_fn(|_| leaf());
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    cad_predicates::rational_bezier_plane_point_identity(&mut ctx, &cc, plane, parameter, reversed)
        .map(Some)
        .map_err(|_| {
            Error::new(
                "NURBS_INVALID_INPUT",
                "Invalid exact curve point identity request",
            )
        })
}
#[cfg(test)]
mod point_tests {
    use super::*;
    use cad_predicates::BezierIdentity;
    #[test]
    fn rational_fraction_plane_membership_is_exact_without_cartesian_rounding() {
        let c = Curve {
            degree: 1,
            knots: vec![-5., -5., 3., 3.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        let plane = [[0.5, 0., 0.], [0.5, 1., 0.], [0.5, 0., 1.]];
        for (parameter, reverse) in [([1., 0., 3.], false), ([2., 0., 3.], true)] {
            assert_eq!(
                verify_curve_point(&c, plane, parameter, reverse, 1_000_000)
                    .unwrap()
                    .unwrap()
                    .outcome,
                BezierIdentity::Equal
            );
        }
        // At q=1/4 the exact rational coordinate is 2/5; Binary64 0.4
        // cannot substitute for it as an exact authored plane coordinate.
        let rounded = [[0.4, 0., 0.], [0.4, 1., 0.], [0.4, 0., 1.]];
        assert_eq!(
            verify_curve_point(&c, rounded, [0.25, 0., 1.], false, 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Different
        );
        assert_eq!(
            verify_curve_point(&c, plane, [1. + 1e-12, 0., 3.], false, 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Different
        );
        assert!(matches!(
            verify_curve_point(&c, plane, [1., 0., 3.], false, 0)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Indeterminate(_)
        ));
        assert!(matches!(
            verify_curve_point(&c, [[0.; 3]; 3], [1., 0., 3.], false, 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Indeterminate(_)
        ));
    }
}
