//! Exact original-coefficient point inspection; does not certify a full lift.
use super::*;
/// A nonzero point residual refutes full exact lift identity. Point equality
/// supplies no authority for unvisited intervals, chart images, or solid validity.
pub fn verify_exact_point(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    normalized_parameter: f64,
    max_work: u64,
) -> Result<Option<cad_predicates::SplinePointLiftDecision>> {
    use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
    c.validate()?;
    p.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3 && p.control_points[0].len() == 2,
        "Point lift needs XYZ curve and UV pcurve",
    )?;
    check(
        normalized_parameter.is_finite() && (0. ..=1.).contains(&normalized_parameter),
        "Choose a normalized point parameter",
    )?;
    check(
        max_work <= cad_predicates::MAX_WORK,
        "Point lift work limit exceeded",
    )?;
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || [c.degree, p.degree, s.degree_u, s.degree_v]
            .iter()
            .any(|&d| d > 32)
        || c.control_points.len() > 4096
        || p.control_points.len() > 4096
        || s.control_points.len() * s.control_points[0].len() > 4096
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for (point, w) in c.control_points.iter().zip(&c.weights) {
        values.extend_from_slice(point);
        values.push(*w);
    }
    let pi = values.len();
    for (point, w) in p.control_points.iter().zip(&p.weights) {
        values.extend_from_slice(point);
        values.push(*w);
    }
    let si = values.len();
    for (row, weights) in s.control_points.iter().zip(&s.weights) {
        for (point, w) in row.iter().zip(weights) {
            values.extend_from_slice(point);
            values.push(*w);
        }
    }
    let mut offsets = Vec::new();
    for knots in [&c.knots, &p.knots, &s.knots_u, &s.knots_v] {
        offsets.push(values.len());
        values.extend_from_slice(knots);
    }
    let ti = values.len();
    values.push(normalized_parameter);
    values.push(reversed as u8 as f64);
    let arena = SourceArena::authored(
        "original-spline-point-lift",
        1,
        values
            .iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|e| crate::input(e.to_string()))?;
    let row4 = |base| std::array::from_fn(|k| arena.leaf(base + k).unwrap());
    let curve = (0..c.control_points.len())
        .map(|i| row4(4 * i))
        .collect::<Vec<_>>();
    let pc = (0..p.control_points.len())
        .map(|i| std::array::from_fn(|k| arena.leaf(pi + 3 * i + k).unwrap()))
        .collect::<Vec<_>>();
    let surface = (0..s.control_points.len())
        .map(|u| {
            (0..s.control_points[0].len())
                .map(|v| row4(si + 4 * (u * s.control_points[0].len() + v)))
                .collect()
        })
        .collect::<Vec<_>>();
    let knots = offsets
        .iter()
        .zip([&c.knots, &p.knots, &s.knots_u, &s.knots_v])
        .map(|(offset, k)| {
            (0..k.len())
                .map(|i| arena.leaf(offset + i).unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
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
    cad_predicates::rational_bspline_curve_surface_point_identity(
        &mut ctx,
        &curve,
        &pc,
        &surface,
        [&knots[0], &knots[1], &knots[2], &knots[3]],
        [c.degree, p.degree, s.degree_u, s.degree_v],
        arena.leaf(ti).unwrap(),
        reversed,
    )
    .map(Some)
    .map_err(|e| crate::input(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_predicates::{BezierIdentity as B, Reason};
    fn fixture() -> (Curve, Curve, Surface) {
        let c = Curve {
            degree: 1,
            knots: vec![2., 2., 4., 6., 6.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 0., 0.]],
            weights: vec![1., 2., 3.],
            periodic: false,
        };
        let p = Curve {
            knots: vec![-4., -4., 3., 10., 10.],
            control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 0.]],
            ..c.clone()
        };
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![2.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        (c, p, s)
    }
    #[test]
    fn original_rational_multispans_domains_and_surface_gauge_are_exact() {
        let (c, p, s) = fixture();
        let before = (c.clone(), p.clone(), s.clone());
        for t in [0., 0.25, 0.5, 0.75, 1.] {
            let r = verify_exact_point(&c, &p, &s, false, t, 1_000_000)
                .unwrap()
                .unwrap();
            assert_eq!(r.point_equality, B::Equal);
            assert!(r.work_used <= 1_000_000);
        }
        assert_eq!((c, p, s), before);
    }
    #[test]
    fn original_sub_float_gap_and_wrong_direction_are_not_equal() {
        let (mut c, p, s) = fixture();
        assert_eq!(
            verify_exact_point(&c, &p, &s, true, 0.25, 1_000_000)
                .unwrap()
                .unwrap()
                .point_equality,
            B::Different
        );
        c.control_points[1][2] = 2f64.powi(-100);
        assert_eq!(
            verify_exact_point(&c, &p, &s, false, 0.5, 1_000_000)
                .unwrap()
                .unwrap()
                .point_equality,
            B::Different
        );
    }
    #[test]
    fn equality_at_one_point_never_establishes_a_full_lift() {
        let (mut c, p, s) = fixture();
        c.control_points[2][2] = 1.;
        assert_eq!(
            verify_exact_point(&c, &p, &s, false, 0., 1_000_000)
                .unwrap()
                .unwrap()
                .point_equality,
            B::Equal
        );
        assert_eq!(
            verify_exact_point(&c, &p, &s, false, 0.75, 1_000_000)
                .unwrap()
                .unwrap()
                .point_equality,
            B::Different
        );
    }
    #[test]
    fn exhausted_work_and_outside_chart_remain_unproven() {
        let (c, mut p, s) = fixture();
        for limit in [0, 1, 10] {
            let r = verify_exact_point(&c, &p, &s, false, 0.5, limit)
                .unwrap()
                .unwrap();
            assert_eq!(r.point_equality, B::Indeterminate(Reason::ResourceLimit));
            assert!(r.work_used <= limit);
        }
        for q in &mut p.control_points {
            q[0] = 2.;
        }
        assert_eq!(
            verify_exact_point(&c, &p, &s, false, 0.5, 1_000_000)
                .unwrap()
                .unwrap()
                .point_equality,
            B::Indeterminate(Reason::MissingProof)
        );
    }
    #[test]
    fn discontinuous_internal_knot_does_not_select_a_convenient_side() {
        let (mut c, mut p, s) = fixture();
        c.knots = vec![0., 0., 0.5, 0.5, 1., 1.];
        c.control_points.insert(2, vec![0.75, 0., 0.]);
        c.weights = vec![1.; 4];
        p.knots = c.knots.clone();
        p.control_points.insert(2, vec![0.75, 0.]);
        p.weights = c.weights.clone();
        assert_eq!(
            verify_exact_point(&c, &p, &s, false, 0.5, 1_000_000)
                .unwrap_err().code,
            "NURBS_INVALID_INPUT"
        );
    }
}
