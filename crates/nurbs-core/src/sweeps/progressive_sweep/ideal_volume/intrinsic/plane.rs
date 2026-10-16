//! Exact original transverse-plane premise; no tolerance or rounded normal.
use super::*;
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};
pub(super) fn certify(path: &Curve, cap: &Surface, spent: &mut u64, max: u64) -> Result<bool> {
    // The owner has already proved clamped positive initial speed, so its
    // exact initial tangent is a positive multiple of P1-P0.
    let count = cap.control_points.iter().map(Vec::len).sum::<usize>();
    if count > (65536 - 6) / 3 || *spent == max {
        return Ok(false);
    }
    let values = path.control_points[..2]
        .iter()
        .chain(cap.control_points.iter().flatten())
        .flat_map(|p| p.iter().map(|v| AuthoredScalar::Binary64Bits(v.to_bits())))
        .collect();
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
        let d =
            cad_predicates::direction_dot3d(&mut ctx, refs(0), refs(3), refs(0), refs(6 + 3 * i));
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
}
