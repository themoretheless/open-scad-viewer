//! Exact authored tangent agreement, with proof work charged to the caller.
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};

pub(super) fn aligned(a: &[f64], p: &[f64], b: &[f64], remaining: usize, used: &mut usize) -> bool {
    let forward = |axis: usize| {
        (a[axis] < p[axis] && p[axis] < b[axis]) || (a[axis] > p[axis] && p[axis] > b[axis])
    };
    if !(0..3).any(forward) {
        return false;
    }
    if (0..3).any(|axis| forward(axis) && (0..3).all(|k| k == axis || a[k] == p[k] && p[k] == b[k]))
    {
        return true;
    }
    if remaining == 0 {
        return false;
    }
    let values = [a, p, b]
        .iter()
        .flat_map(|v| v.iter())
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("profile-frame-join", 1, values) else {
        return false;
    };
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: remaining as u64,
            ..Limits::default()
        },
        None,
    );
    let mut proved = true;
    for axes in [[0, 1], [0, 2], [1, 2]] {
        let refs = |i: usize| axes.map(|k| source.leaf(3 * i + k).unwrap());
        let decision = cad_predicates::orient2d(&mut context, refs(0), refs(1), refs(2));
        if !matches!(decision,Ok(d) if d.outcome == Outcome::Sign(Sign::Zero)) {
            proved = false;
            break;
        }
    }
    *used += context.work_used() as usize;
    proved
}

/// Prove that every original positive-weight control lies in one plane.
/// Interval arithmetic encloses its unit normal; floating coplanarity is never used.
pub(super) fn plane(
    points: &[Vec<f64>],
    remaining: usize,
    used: &mut usize,
) -> Option<[crate::distance_bounds::Interval; 3]> {
    use crate::distance_bounds::Interval as I;
    use crate::sweep_support::interval_vec3::{cross, div, norm, sub};
    let point = |p: &[f64]| std::array::from_fn(|k| I::point(p[k]));
    let first = points.first()?;
    let second = points.iter().position(|p| p != first)?;
    let direction = sub(point(&points[second]), point(first)).ok()?;
    let mut candidate = None;
    let mut selection = 0;
    for (i, p) in points.iter().enumerate() {
        if selection >= remaining {
            *used += selection;
            return None;
        }
        selection += 1;
        let normal = cross(direction, sub(point(p), point(first)).ok()?).ok()?;
        let length = norm(normal).ok()?;
        if length.lo > 0. {
            candidate = Some((i, div(normal, length).ok()?));
            break;
        }
    }
    *used += selection;
    let (third, normal) = candidate?;
    let values = points
        .iter()
        .flat_map(|p| p.iter())
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let source = SourceArena::authored("profile-frame-plane", 1, values).ok()?;
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: (remaining - selection) as u64,
            ..Limits::default()
        },
        None,
    );
    let refs = |i: usize| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap());
    let mut proved = true;
    for i in 0..points.len() {
        let decision =
            cad_predicates::orient3d(&mut context, refs(0), refs(second), refs(third), refs(i));
        if !matches!(decision,Ok(d) if d.outcome==Outcome::Sign(Sign::Zero)) {
            proved = false;
            break;
        }
    }
    *used += context.work_used() as usize;
    proved.then_some(normal)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oblique_plane_checks_all_original_controls_and_work_limit() {
        let points = vec![
            vec![0., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 0., 1.],
            vec![1., 1., 2.],
        ];
        let original = points.clone();
        let mut used = 0;
        assert!(plane(&points, 10000, &mut used).is_some());
        assert!(used > 0 && used <= 10000);
        let mut changed = points.clone();
        changed[3][1] = changed[3][1].next_up();
        assert!(plane(&changed, 10000, &mut 0).is_none());
        for limit in [0, 1, 2, 5] {
            let mut work = 0;
            assert!(plane(&points, limit, &mut work).is_none());
            assert!(work <= limit);
        }
        assert_eq!(points, original);
    }
    #[test]
    fn exact_oblique_join_refuses_ulp_changes_reversal_and_exhaustion() {
        let a = [-2., -4., -6.];
        let p = [0., 0., 0.];
        let b = [3., 6., 9.];
        let mut used = 0;
        assert!(aligned(&a, &p, &b, 10000, &mut used));
        assert!(used > 0 && used <= 10000);
        let mut changed = b;
        changed[2] = changed[2].next_up();
        assert!(!aligned(&a, &p, &changed, 10000, &mut 0));
        assert!(!aligned(&a, &p, &a, 10000, &mut 0));
        assert!(!aligned(&a, &p, &b, 0, &mut 0));
        let mut limited = 0;
        assert!(!aligned(&a, &p, &b, 1, &mut limited));
        assert!(limited <= 1);
        assert_eq!(b, [3., 6., 9.]);
    }
}
