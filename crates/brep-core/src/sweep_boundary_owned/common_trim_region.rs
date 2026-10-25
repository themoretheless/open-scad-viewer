//! Exact positive affine UV pullback between original authored trim regions.
//! No rounded normalized curve or caller-provided correspondence is trusted.
use super::*;
use cad_predicates::{
    AuthoredScalar, Limits as PredicateLimits, Outcome, PredicateContext, Sign, SourceArena,
    ToleranceContext,
};
fn domain(s: &nurbs_core::surface::Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
pub(super) fn certify(caps: [&LoftCap; 2], spent: &mut u64, max: u64) -> Result<bool> {
    let domains = caps.map(|c| domain(&c.surface));
    if caps[0].trims == caps[1].trims && domains[0] == domains[1] {
        return Ok(true);
    }
    if caps[0].trims.len() != caps[1].trims.len() {
        return Ok(false);
    }
    for (a, b) in caps[0].trims.iter().zip(&caps[1].trims) {
        if a.len() != b.len() {
            return Ok(false);
        }
        for (a, b) in a.iter().zip(b) {
            if !curve(a, b, domains, spent, max)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn curve(a: &Curve, b: &Curve, d: [[[f64; 2]; 2]; 2], spent: &mut u64, max: u64) -> Result<bool> {
    a.validate()?;
    b.validate()?;
    if a.degree != b.degree
        || a.periodic != b.periodic
        || a.knots.len() != b.knots.len()
        || a.control_points.len() != b.control_points.len()
        || a.control_points
            .iter()
            .chain(&b.control_points)
            .any(|p| p.len() != 2)
        || *spent == max
    {
        return Ok(false);
    }
    let mut points = vec![
        [d[0][0][0], d[1][0][0]],
        [d[0][0][1], d[1][0][1]],
        [d[0][1][0], d[1][1][0]],
        [d[0][1][1], d[1][1][1]],
    ];
    let mut checks = Vec::new();
    for (p, q) in a.control_points.iter().zip(&b.control_points) {
        for k in 0..2 {
            let i = points.len();
            points.push([p[k], q[k]]);
            checks.push([2 * k, 2 * k + 1, i]);
        }
    }
    if a.weights != b.weights {
        let zero = points.len();
        points.push([0., 0.]);
        let first = points.len();
        points.push([a.weights[0], b.weights[0]]);
        for (&p, &q) in a.weights.iter().zip(&b.weights) {
            let i = points.len();
            points.push([p, q]);
            checks.push([zero, first, i]);
        }
    }
    if a.knots != b.knots {
        let lo = points.len();
        points.push([a.domain()[0], b.domain()[0]]);
        let hi = points.len();
        points.push([a.domain()[1], b.domain()[1]]);
        for (&p, &q) in a.knots.iter().zip(&b.knots) {
            let i = points.len();
            points.push([p, q]);
            checks.push([lo, hi, i]);
        }
    }
    if points.len() > 32768 {
        return Ok(false);
    }
    let Ok(arena) = SourceArena::authored(
        "original-affine-common-trim-region",
        1,
        points
            .iter()
            .flatten()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    ) else {
        return Ok(false);
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        PredicateLimits {
            max_work: max - *spent,
            ..PredicateLimits::default()
        },
        None,
    );
    let refs = |i: usize| [arena.leaf(2 * i).unwrap(), arena.leaf(2 * i + 1).unwrap()];
    let mut proven = true;
    for [a, b, c] in checks {
        if !matches!(
            cad_predicates::orient2d(&mut ctx, refs(a), refs(b), refs(c)).map(|r| r.outcome),
            Ok(Outcome::Sign(Sign::Zero))
        ) {
            proven = false;
            break;
        }
    }
    *spent += ctx.work_used();
    Ok(proven)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn caps() -> [LoftCap; 2] {
        std::array::from_fn(|i| {
            let (u, v, t) = if i == 0 {
                ([-3., 5.], [7., 11.], [-2., 3.])
            } else {
                ([17., 29.], [-7., 13.], [11., 31.])
            };
            let surface = nurbs_core::surface::Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![u[0], u[0], u[1], u[1]],
                knots_v: vec![v[0], v[0], v[1], v[1]],
                control_points: vec![
                    vec![vec![0.; 3], vec![0., 1., 0.]],
                    vec![vec![1., 0., 0.], vec![1., 1., 0.]],
                ],
                weights: vec![vec![1.; 2]; 2],
                periodic_u: false,
                periodic_v: false,
            };
            let corners = [
                [u[0], v[0]],
                [u[1], v[0]],
                [u[1], v[1]],
                [u[0], v[1]],
                [u[0], v[0]],
            ];
            let trims = vec![
                (0..4)
                    .map(|j| Curve {
                        degree: 1,
                        knots: vec![t[0], t[0], t[1], t[1]],
                        control_points: vec![corners[j].to_vec(), corners[j + 1].to_vec()],
                        weights: vec![if i == 0 { 1. } else { 2. }; 2],
                        periodic: false,
                    })
                    .collect(),
            ];
            LoftCap { surface, trims }
        })
    }
    #[test]
    fn original_uv_domain_and_curve_phase_pullbacks_own_exact_work() {
        let c = caps();
        let mut used = 0;
        assert!(certify([&c[0], &c[1]], &mut used, 1000000).unwrap());
        assert!(used > 0);
        let mut exact = 0;
        assert!(certify([&c[0], &c[1]], &mut exact, used).unwrap());
        assert_eq!(exact, used);
        for max in [0, used - 1] {
            let mut short = 0;
            assert!(!certify([&c[0], &c[1]], &mut short, max).unwrap());
            assert!(short <= max);
        }
    }
    #[test]
    fn a_sub_tolerance_trim_change_never_receives_region_identity() {
        let mut c = caps();
        c[1].trims[0][0].control_points[0][0] = 17_f64.next_up();
        let mut used = 0;
        assert!(!certify([&c[0], &c[1]], &mut used, 1000000).unwrap());
    }
}
