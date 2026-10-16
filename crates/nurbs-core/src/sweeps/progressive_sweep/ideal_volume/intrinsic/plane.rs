//! Exact original transverse-plane premise; no tolerance or rounded normal.
use super::*;
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};
pub(super) fn certify(path: &Curve, cap: &Surface, spent: &mut u64, max: u64) -> Result<bool> {
    certify_frame(path, cap, None, spent, max)
}
pub(super) fn certify_frame(
    path: &Curve,
    cap: &Surface,
    longitudinal: Option<&Curve>,
    spent: &mut u64,
    max: u64,
) -> Result<bool> {
    // Clamped endpoints bind the initial longitudinal to original source
    // poles. Authored frame A0 and path tangent P1-P0 are distinct premises.
    if let Some(a) = longitudinal {
        let [lo, hi] = a.domain();
        if a.periodic
            || a.knots[..=a.degree].iter().any(|u| *u != lo)
            || a.knots[a.control_points.len()..].iter().any(|u| *u != hi)
        {
            return Ok(false);
        }
    }
    let count = cap.control_points.iter().map(Vec::len).sum::<usize>();
    let prefix = if longitudinal.is_some() { 9 } else { 6 };
    if count > (65536 - prefix) / 3 || *spent == max {
        return Ok(false);
    }
    let mut values = Vec::new();
    if let Some(a) = longitudinal {
        values.extend((0..3).map(|_| AuthoredScalar::RationalConstant {
            numerator: 0,
            denominator: 1,
        }));
        values.extend(
            a.control_points[0]
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits())),
        );
        values.extend(
            path.control_points[0]
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits())),
        );
    } else {
        values.extend(
            path.control_points[..2]
                .iter()
                .flatten()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits())),
        );
    }
    values.extend(
        cap.control_points
            .iter()
            .flatten()
            .flatten()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits())),
    );
    let Ok(arena) = SourceArena::authored("original-bishop-filled-transverse-plane", 1, values)
    else {
        return Ok(false);
    };
    let refs = |i| std::array::from_fn(|k| arena.leaf(i + k).unwrap());
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: max - *spent,
            ..Limits::default()
        },
        None,
    );
    let mut proved = true;
    for i in 0..count {
        let d = cad_predicates::direction_dot3d(
            &mut ctx,
            refs(0),
            refs(3),
            refs(if longitudinal.is_some() { 6 } else { 0 }),
            refs(prefix + 3 * i),
        );
        if !matches!(d.map(|v| v.outcome), Ok(Outcome::Sign(Sign::Zero))) {
            proved = false;
            break;
        }
    }
    *spent += ctx.work_used();
    Ok(proved)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> (Curve, Surface) {
        let path = Curve {
            degree: 1,
            knots: vec![-3., -3., 5., 5.],
            control_points: vec![vec![8., -4., 2.], vec![9., -2., 4.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        let origin = [8., -4., 2.];
        let a = [2., -1., 0.];
        let b = [2., 0., -1.];
        let cap = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![7., 7., 11., 11.],
            knots_v: vec![-2., -2., 3., 3.],
            control_points: (0..2)
                .map(|i| {
                    (0..2)
                        .map(|j| {
                            (0..3)
                                .map(|k| {
                                    origin[k]
                                        + (2 * i - 1) as f64 * a[k]
                                        + (2 * j - 1) as f64 * b[k]
                                })
                                .collect()
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1., 0.5], vec![2., 3.]],
            periodic_u: false,
            periodic_v: false,
        };
        (path, cap)
    }
    #[test]
    fn translated_nonaxial_original_plane_requires_exact_coefficients_and_budget() {
        let (path, cap) = sources();
        path.validate().unwrap();
        cap.validate().unwrap();
        let original = (path.clone(), cap.clone());
        let mut used = 0;
        assert!(certify(&path, &cap, &mut used, 1000000).unwrap());
        assert!(used > 0);
        for max in [0, used - 1] {
            let mut spent = 0;
            assert!(!certify(&path, &cap, &mut spent, max).unwrap());
            assert!(spent <= max);
        }
        let mut off = cap.clone();
        off.control_points[1][1][2] = off.control_points[1][1][2].next_up();
        assert!(!certify(&path, &off, &mut 0, 1000000).unwrap());
        assert_eq!((path, cap), original);
    }
    #[test]
    fn authored_source_plane_is_owned_by_frame_longitudinal_not_path_tangent() {
        let (mut path, cap) = sources();
        path.control_points[1] = vec![8., -4., 3.];
        let a = crate::progressive_sweep::constant_vector_law([1., 2., 2.]).unwrap();
        assert!(!certify(&path, &cap, &mut 0, 1000000).unwrap());
        let mut work = 0;
        assert!(certify_frame(&path, &cap, Some(&a), &mut work, 1000000).unwrap());
        assert!(work > 0);
        for max in [0, work - 1] {
            let mut used = 0;
            assert!(!certify_frame(&path, &cap, Some(&a), &mut used, max).unwrap());
            assert!(used <= max);
        }
        let mut off = cap;
        off.control_points[1][1][2] = off.control_points[1][1][2].next_up();
        assert!(!certify_frame(&path, &off, Some(&a), &mut 0, 1000000).unwrap());
    }
}
