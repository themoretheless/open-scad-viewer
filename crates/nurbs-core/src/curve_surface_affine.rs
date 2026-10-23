//! Exact formal agreement with a subinterval of an unchanged canonical curve.
//! Chart membership of the actual pcurve restriction remains a separate proof.
use crate::{
    curve::Curve, curve_surface_agreement::AlgebraicCompositionIdentity, surface::Surface, Error,
    Result,
};
use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
/// `range` gives two exact normalized numerator/denominator pairs. Both
/// endpoints must be in [0,1] and distinct; either traversal direction is valid.
pub fn verify_algebraic(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    range: [[f64; 2]; 2],
    max_work: u64,
) -> Result<Option<AlgebraicCompositionIdentity>> {
    c.validate()?;
    p.validate()?;
    s.validate()?;
    if c.control_points[0].len() != 3
        || p.control_points[0].len() != 2
        || range.iter().flatten().any(|v| !v.is_finite())
        || max_work > cad_predicates::MAX_WORK
    {
        return Err(Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid original affine composition input",
        ));
    }
    let bezier = |k: &[f64], d: usize, n: usize| {
        n == d + 1 && k[..=d].iter().all(|&x| x == k[d]) && k[n..].iter().all(|&x| x == k[n])
    };
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || c.degree > 32
        || p.degree > 8
        || s.degree_u > 8
        || s.degree_v > 8
        || !bezier(&c.knots, c.degree, c.control_points.len())
        || !bezier(&p.knots, p.degree, p.control_points.len())
        || !bezier(&s.knots_u, s.degree_u, s.control_points.len())
        || !bezier(&s.knots_v, s.degree_v, s.control_points[0].len())
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for (points, weights) in [
        (&c.control_points, &c.weights),
        (&p.control_points, &p.weights),
    ] {
        for (point, w) in points.iter().zip(weights) {
            values.extend(point.iter().copied());
            values.push(*w);
        }
    }
    for (row, weights) in s.control_points.iter().zip(&s.weights) {
        for (point, w) in row.iter().zip(weights) {
            values.extend(point.iter().copied());
            values.push(*w);
        }
    }
    values.extend([
        s.knots_u[s.degree_u],
        s.knots_u[s.control_points.len()],
        s.knots_v[s.degree_v],
        s.knots_v[s.control_points[0].len()],
    ]);
    values.extend(range.iter().flatten().copied());
    let source = SourceArena::authored(
        "original-affine-composition",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("NURBS_INVALID_INPUT", "Invalid affine composition source"))?;
    let mut index = 0;
    let mut leaf = || {
        let r = source.leaf(index).unwrap();
        index += 1;
        r
    };
    let cc: Vec<_> = c
        .control_points
        .iter()
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect();
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
    let range = std::array::from_fn(|_| std::array::from_fn(|_| leaf()));
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
    let r = cad_predicates::rational_bezier_composition_affine_identity(
        &mut ctx, &cc, &pp, &ss, domain, range,
    )
    .map_err(|_| {
        Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid affine composition identity request",
        )
    })?;
    Ok(Some(AlgebraicCompositionIdentity {
        outcome: r.outcome,
        work_used: r.work_used,
        context: r.context,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use cad_predicates::BezierIdentity;
    fn plane(fixed: bool) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if fixed {
                                vec![0.609375, u as f64, v as f64]
                            } else {
                                vec![u as f64, v as f64, 0.]
                            }
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn closing_line_coverage_uses_exact_fraction_endpoints_and_preserves_definitions() {
        let c = Curve {
            degree: 1,
            knots: vec![-5., -5., 3., 3.],
            control_points: vec![vec![0.609375, -0.25, 0.], vec![0.609375, 1.25, 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let p = Curve {
            degree: 1,
            knots: vec![10., 10., 18., 18.],
            control_points: vec![vec![0., 0.], vec![0.859375, 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let s = plane(true);
        let range = [[1., 6.], [71., 96.]];
        assert_eq!(
            verify_algebraic(&c, &p, &s, range, 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
        assert_eq!(
            crate::curve_surface_agreement::verify_exact_algebraic(&c, &p, &s, false, 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Different
        );
        let mut reverse = p.clone();
        reverse.control_points.reverse();
        assert_eq!(
            verify_algebraic(&c, &reverse, &s, [range[1], range[0]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
        assert_eq!(
            verify_algebraic(&c, &p, &s, [[1., 6.], [71. + 1e-12, 96.]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Different
        );
        for invalid in [
            [[1., 0.], [71., 96.]],
            [[1., 6.], [1., 6.]],
            [[1., 6.], [97., 96.]],
        ] {
            assert!(matches!(
                verify_algebraic(&c, &p, &s, invalid, 1_000_000)
                    .unwrap()
                    .unwrap()
                    .outcome,
                BezierIdentity::Indeterminate(_)
            ));
        }
        assert!(matches!(
            verify_algebraic(&c, &p, &s, range, 0)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Indeterminate(_)
        ));
    }
    #[test]
    fn curved_and_weighted_canonical_subintervals_have_exact_formal_composition() {
        let s = plane(false);
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let p = Curve {
            degree: 2,
            knots: c.knots.clone(),
            control_points: vec![
                vec![0.9375, 0.4375],
                vec![0.8125, 0.8125],
                vec![0.4375, 0.9375],
            ],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert_eq!(
            verify_algebraic(&c, &p, &s, [[1., 4.], [3., 4.]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        let p = Curve {
            degree: 1,
            knots: c.knots.clone(),
            control_points: vec![vec![0.5, 0.], vec![1., 0.]],
            weights: vec![4., 6.],
            periodic: false,
        };
        assert_eq!(
            verify_algebraic(&c, &p, &s, [[1., 3.], [1., 1.]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
        let mut wrong = p.clone();
        wrong.weights[1] = 6. + 1e-12;
        assert_eq!(
            verify_algebraic(&c, &wrong, &s, [[1., 3.], [1., 1.]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Different
        );
    }
}
