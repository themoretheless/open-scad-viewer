//! A separate exact image-only agreement route. Never use this as evidence of
//! normalized parameter correspondence for retained coedges or error bounds.
use super::*;
/// Complete oriented segment image of an original regular rational Bezier and
/// an original axis-line pcurve on a positive rational bilinear carrier.
/// Unsupported shapes return None; native work exhaustion remains Unproved.
pub fn verify_axis_line_image(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    max_work: u64,
) -> Result<Option<cad_predicates::LineImageDecision>> {
    use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
    c.validate()?;
    p.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3
            && p.control_points[0].len() == 2
            && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Line image needs XYZ curve/carrier and UV pcurve",
    )?;
    let bezier = |knots: &[f64], degree: usize, n: usize| {
        n == degree + 1
            && knots.len() == 2 * n
            && knots[..n].iter().all(|u| *u == knots[degree])
            && knots[n..].iter().all(|u| *u == knots[n])
    };
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || !(1..=32).contains(&c.degree)
        || !(1..=32).contains(&p.degree)
        || s.degree_u != 1
        || s.degree_v != 1
        || s.control_points.len() != 2
        || s.control_points[0].len() != 2
        || !bezier(&c.knots, c.degree, c.control_points.len())
        || !bezier(&p.knots, p.degree, p.control_points.len())
        || !bezier(&s.knots_u, s.degree_u, 2)
        || !bezier(&s.knots_v, s.degree_v, 2)
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for (point, w) in c.control_points.iter().zip(&c.weights) {
        values.extend(point.iter().copied());
        values.push(*w);
    }
    for (point, w) in p.control_points.iter().zip(&p.weights) {
        values.extend(point.iter().copied());
        values.push(*w);
    }
    for i in 0..2 {
        for j in 0..2 {
            values.extend(s.control_points[i][j].iter().copied());
            values.push(s.weights[i][j]);
        }
    }
    values.extend([s.knots_u[1], s.knots_u[2], s.knots_v[1], s.knots_v[2]]);
    let arena = SourceArena::authored(
        "nurbs-curve-surface-axis-line-image",
        1,
        values
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::Error::new("NURBS_INVALID_INPUT", "Invalid original line image scalars"))?;
    let cc: Vec<_> = (0..c.control_points.len())
        .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
        .collect();
    let o = 4 * c.control_points.len();
    let pp: Vec<_> = (0..p.control_points.len())
        .map(|i| std::array::from_fn(|k| arena.leaf(o + 3 * i + k).unwrap()))
        .collect();
    let o = o + 3 * p.control_points.len();
    let ss = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| arena.leaf(o + 4 * (2 * i + j) + k).unwrap())
        })
    });
    let dd =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(o + 16 + 2 * i + j).unwrap()));
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    cad_predicates::rational_bezier_axis_line_image(&mut context, &cc, &pp, ss, dd)
        .map(Some)
        .map_err(|_| {
            crate::Error::new(
                "NURBS_INVALID_INPUT",
                "Invalid original line image predicate request",
            )
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_certificate_does_not_grant_normalized_trace_correspondence() {
        let c = Curve {
            degree: 2,
            knots: vec![13., 13., 13., 19., 19., 19.],
            control_points: vec![
                vec![-0.5, 0.5, -0.25],
                vec![0., 0.5, 0.],
                vec![0.5, 0.5, 0.25],
            ],
            weights: vec![1., 3., 2.],
            periodic: false,
        };
        let p = Curve {
            degree: 2,
            knots: vec![101., 101., 101., 109., 109., 109.],
            control_points: vec![vec![3., 3.], vec![4., 3.], vec![5., 3.]],
            weights: vec![1., 0.25, 2.],
            periodic: false,
        };
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![2., 2., 6., 6.],
            knots_v: vec![-3., -3., 5., 5.],
            control_points: vec![
                vec![vec![-1., -1., 1.], vec![-1., 1., -1.]],
                vec![vec![1., -1., -1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let before = (c.clone(), p.clone(), s.clone());
        let map = verify_exact(&c, &p, &s, false, 1000000).unwrap().unwrap();
        assert_eq!(map.outcome, cad_predicates::BezierIdentity::Different);
        let image = verify_axis_line_image(&c, &p, &s, 1000000)
            .unwrap()
            .unwrap();
        assert_eq!(
            image.outcome,
            cad_predicates::LineImageOutcome::SameOrientedSegment
        );
        assert_eq!(
            verify_axis_line_image(&c, &p, &s, image.work_used)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::LineImageOutcome::SameOrientedSegment
        );
        let stopped = verify_axis_line_image(&c, &p, &s, 0).unwrap().unwrap();
        assert!(matches!(
            stopped.outcome,
            cad_predicates::LineImageOutcome::Unproved(_)
        ));
        assert_eq!(stopped.work_used, 0);
        assert_eq!((c, p, s), before);
    }
}
