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
