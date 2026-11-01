//! Exact source images under a freshly derived positive projective clock.
//! This never substitutes for normalized pointwise correspondence.
use super::*;
pub fn verify_projective_image(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    max_work: u64,
) -> Result<Option<cad_predicates::ProjectiveImageDecision>> {
    use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
    c.validate()?;
    p.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3
            && p.control_points[0].len() == 2
            && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Projective images require XYZ and UV sources",
    )?;
    let bezier = |knots: &[f64], degree: usize, n: usize| {
        n == degree + 1
            && knots.len() == 2 * n
            && knots[..n].iter().all(|u| *u == knots[degree])
            && knots[n..].iter().all(|u| *u == knots[n])
    };
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || !(1..=32).contains(&c.degree)
        || !(1..=8).contains(&p.degree)
        || !(1..=8).contains(&s.degree_u)
        || !(1..=8).contains(&s.degree_v)
        || !bezier(&c.knots, c.degree, c.control_points.len())
        || !bezier(&p.knots, p.degree, p.control_points.len())
        || !bezier(&s.knots_u, s.degree_u, nu)
        || !bezier(&s.knots_v, s.degree_v, nv)
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for (row, w) in c.control_points.iter().zip(&c.weights) {
        values.extend(row.iter().copied());
        values.push(*w);
    }
    for (row, w) in p.control_points.iter().zip(&p.weights) {
        values.extend(row.iter().copied());
        values.push(*w);
    }
    for i in 0..nu {
        for j in 0..nv {
            values.extend(s.control_points[i][j].iter().copied());
            values.push(s.weights[i][j]);
        }
    }
    values.extend([
        s.knots_u[s.degree_u],
        s.knots_u[nu],
        s.knots_v[s.degree_v],
        s.knots_v[nv],
    ]);
    let arena = SourceArena::authored(
        "nurbs-curve-surface-projective-image",
        1,
        values
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| {
        crate::Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid projective image source scalars",
        )
    })?;
    let cc: Vec<_> = (0..c.control_points.len())
        .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
        .collect();
    let o = 4 * c.control_points.len();
    let pp: Vec<_> = (0..p.control_points.len())
        .map(|i| std::array::from_fn(|k| arena.leaf(o + 3 * i + k).unwrap()))
        .collect();
    let o = o + 3 * p.control_points.len();
    let ss: Vec<Vec<_>> = (0..nu)
        .map(|i| {
            (0..nv)
                .map(|j| std::array::from_fn(|k| arena.leaf(o + 4 * (i * nv + j) + k).unwrap()))
                .collect()
        })
        .collect();
    let o = o + 4 * nu * nv;
    let dd = std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(o + 2 * i + j).unwrap()));
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    cad_predicates::rational_bezier_projective_image(&mut ctx, &cc, &pp, &ss, dd)
        .map(Some)
        .map_err(|_| {
            crate::Error::new(
                "NURBS_INVALID_INPUT",
                "Invalid projective image predicate request",
            )
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curved_nonplanar_images_keep_distinct_original_clocks_and_no_map_authority() {
        let c = Curve {
            degree: 3,
            knots: vec![13., 13., 13., 13., 19., 19., 19., 19.],
            control_points: vec![
                vec![0., 0.75, 0.],
                vec![1., 1.25, 0.25],
                vec![2., 1.75, 1.],
                vec![3., 2.25, 2.25],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        let p = Curve {
            degree: 1,
            knots: vec![101., 101., 109., 109.],
            control_points: vec![vec![2., -1.], vec![6., 3.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![2., 2., 6., 6.],
            knots_v: vec![-3., -3., 5., 5.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 3., 0.]],
                vec![vec![3., 0., 0.], vec![3., 3., 3.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let before = (c.clone(), p.clone(), s.clone());
        assert_eq!(
            verify_exact(&c, &p, &s, false, 1000000)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Different
        );
        assert!(matches!(
            verify_axis_line_image(&c, &p, &s, 1000000)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::LineImageOutcome::Unproved(_)
        ));
        let r = verify_projective_image(&c, &p, &s, 1000000)
            .unwrap()
            .unwrap();
        assert_eq!(
            r.outcome,
            cad_predicates::ProjectiveImageOutcome::SameOrientedProjectiveImage
        );
        assert_eq!(
            verify_projective_image(&c, &p, &s, r.work_used)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::ProjectiveImageOutcome::SameOrientedProjectiveImage
        );
        let empty = verify_projective_image(&c, &p, &s, 0).unwrap().unwrap();
        assert!(matches!(
            empty.outcome,
            cad_predicates::ProjectiveImageOutcome::Unproved(_)
        ));
        assert_eq!(empty.work_used, 0);
        assert_eq!((c, p, s), before);
    }
}
