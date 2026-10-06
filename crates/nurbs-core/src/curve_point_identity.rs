//! Exact point equality on unchanged original single-span rational UV curves.
use crate::{curve::Curve, Error, Result};
use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
pub fn verify(
    curves: [&Curve; 2],
    parameters: [f64; 2],
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    if parameters.iter().any(|t| !t.is_finite()) || max_work > cad_predicates::MAX_WORK {
        return Err(Error::new(
            "NURBS_INVALID_INPUT",
            "Choose finite point parameters and bounded exact work",
        ));
    }
    for c in curves {
        c.validate()?;
        if c.control_points[0].len() != 2 {
            return Err(Error::new(
                "NURBS_INVALID_INPUT",
                "Choose original UV curves",
            ));
        }
        let n = c.control_points.len();
        if c.periodic
            || c.degree == 0
            || c.degree > 32
            || n != c.degree + 1
            || !c.knots[..=c.degree].iter().all(|&t| t == c.knots[c.degree])
            || !c.knots[n..].iter().all(|&t| t == c.knots[n])
        {
            return Ok(None);
        }
    }
    let mut raw = Vec::new();
    for c in curves {
        for (p, w) in c.control_points.iter().zip(&c.weights) {
            raw.extend([p[0], p[1], *w]);
        }
    }
    for i in 0..2 {
        let d = curves[i].domain();
        raw.extend([parameters[i], d[0], d[1]]);
    }
    let source = SourceArena::authored(
        "original-uv-point-equality",
        1,
        raw.into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("NURBS_INVALID_INPUT", "Invalid exact point inputs"))?;
    let mut cursor = 0;
    let mut poles = Vec::new();
    for c in curves {
        poles.push(
            (0..c.control_points.len())
                .map(|_| {
                    std::array::from_fn(|_| {
                        let r = source.leaf(cursor).unwrap();
                        cursor += 1;
                        r
                    })
                })
                .collect::<Vec<_>>(),
        );
    }
    let parameters = std::array::from_fn(|_| {
        std::array::from_fn(|_| {
            let r = source.leaf(cursor).unwrap();
            cursor += 1;
            r
        })
    });
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
    cad_predicates::rational_bezier_uv_point_identity(&mut ctx, [&poles[0], &poles[1]], parameters)
        .map(Some)
        .map_err(|_| {
            Error::new(
                "NURBS_INVALID_INPUT",
                "Invalid exact point identity request",
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_rational_points_match_without_cartesian_division() {
        let a = Curve {
            degree: 1,
            knots: vec![2., 2., 4., 4.],
            control_points: vec![vec![0., 0.], vec![1., 0.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        let b = Curve {
            degree: 1,
            knots: vec![10., 10., 18., 18.],
            control_points: vec![vec![1., 0.], vec![0., 0.]],
            weights: vec![2., 1.],
            periodic: false,
        };
        assert_eq!(
            verify([&a, &b], [3., 14.], cad_predicates::MAX_WORK)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Equal
        );
        let mut changed = b.clone();
        changed.control_points[0][0] += 1e-12;
        assert_eq!(
            verify([&a, &changed], [3., 14.], cad_predicates::MAX_WORK)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Different
        );
        assert!(matches!(
            verify([&a, &b], [3., 14.], 1).unwrap().unwrap().outcome,
            cad_predicates::BezierIdentity::Indeterminate(_)
        ));
        assert!(matches!(
            verify([&a, &b], [1., 14.], cad_predicates::MAX_WORK)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Indeterminate(_)
        ));
        let multi = Curve::from_polyline(vec![vec![0., 0.], vec![0.5, 0.], vec![1., 0.]]).unwrap();
        assert!(verify([&a, &multi], [3., 0.5], cad_predicates::MAX_WORK)
            .unwrap()
            .is_none());
        assert!(verify([&a, &b], [f64::NAN, 14.], cad_predicates::MAX_WORK).is_err());
        assert_eq!(a.knots, vec![2., 2., 4., 4.]);
    }
}
