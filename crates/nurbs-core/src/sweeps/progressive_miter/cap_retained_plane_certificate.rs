//! Exact original retained plane and outward unnormalized normal enclosure.
use crate::distance_bounds::Interval as I;
#[derive(Clone, Debug)]
pub struct Report {
    pub planar_control_hull_certified: bool,
    pub normal: Option<[[f64; 2]; 3]>,
    pub work: u64,
}
pub fn inspect(surface: &crate::surface::Surface, max_work: u64) -> Report {
    if surface.validate().is_err() {
        return unproved();
    }
    let points = surface.control_points.iter().flatten().collect::<Vec<_>>();
    let anchors = [
        0,
        (surface.control_points.len() - 1) * surface.control_points[0].len(),
        surface.control_points[0].len() - 1,
    ];
    inspect_refs(&points, anchors, max_work)
}
fn unproved() -> Report {
    Report {
        planar_control_hull_certified: false,
        normal: None,
        work: 0,
    }
}

/// Exact parallelism to an original direction b-a. Planarity and two
/// independent retained plane directions are checked; near-parallel is not
/// accepted. No endpoint source/frame ownership is asserted here.
#[derive(Clone, Debug)]
pub struct ParallelReport {
    pub parallel: bool,
    pub work: u64,
    pub reason: Option<&'static str>,
}
pub fn inspect_parallel_axis(
    surface: &crate::surface::Surface,
    a: [f64; 3],
    b: [f64; 3],
    max_work: u64,
) -> ParallelReport {
    use cad_predicates::{
        AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
    };
    let mut out = ParallelReport {
        parallel: false,
        work: 0,
        reason: Some("axis-unproved"),
    };
    if a == b || a.iter().chain(b.iter()).any(|x| !x.is_finite()) {
        return out;
    }
    let plane = inspect(surface, max_work);
    out.work = plane.work;
    if !plane.planar_control_hull_certified {
        out.reason = Some("retained-plane-unproved");
        return out;
    }
    let points = [
        a.to_vec(),
        b.to_vec(),
        surface.control_points[0][0].clone(),
        surface.control_points.last().unwrap()[0].clone(),
        surface.control_points[0].last().unwrap().clone(),
    ];
    let values = points
        .iter()
        .flatten()
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("retained-cap-parallel-axis", 1, values) else {
        return out;
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: max_work - out.work,
            ..Limits::default()
        },
        None,
    );
    let refs = |i: usize| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap());
    let mut parallel = true;
    for i in [3, 4] {
        match cad_predicates::direction_dot3d(&mut ctx, refs(0), refs(1), refs(2), refs(i))
            .map(|d| d.outcome)
        {
            Ok(Outcome::Sign(Sign::Zero)) => (),
            Ok(Outcome::Sign(_)) => {
                parallel = false;
                out.reason = Some("planes-not-parallel");
                break;
            }
            _ => {
                parallel = false;
                out.reason = Some("parallelism-unproved");
                break;
            }
        }
    }
    out.work += ctx.work_used();
    out.parallel = parallel;
    if parallel {
        out.reason = None;
    }
    out
}
/// Original control points and proposed anchors; proposals confer no proof.
pub fn inspect_points(points: &[Vec<f64>], anchors: [usize; 3], max_work: u64) -> Report {
    let refs = points.iter().collect::<Vec<_>>();
    inspect_refs(&refs, anchors, max_work)
}
fn inspect_refs(points: &[&Vec<f64>], anchors: [usize; 3], max_work: u64) -> Report {
    let mut out = unproved();
    use cad_predicates::{
        AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
    };
    if points.len() < 3
        || points.len() > 16384
        || points
            .iter()
            .any(|p| p.len() != 3 || p.iter().any(|x| !x.is_finite()))
        || anchors.iter().any(|i| *i >= points.len())
        || max_work == 0
    {
        return out;
    }
    let values = points
        .iter()
        .flat_map(|p| p.iter())
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("retained-cap-plane", 1, values) else {
        return out;
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: max_work,
            ..Limits::default()
        },
        None,
    );
    let refs = |i: usize| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap());
    let result = (|| -> Option<bool> {
        let mut noncollinear = false;
        for axes in [[0, 1], [1, 2], [0, 2]] {
            let p = |i: usize| axes.map(|k| source.leaf(3 * i + k).unwrap());
            match cad_predicates::orient2d(&mut ctx, p(anchors[0]), p(anchors[1]), p(anchors[2]))
                .ok()?
                .outcome
            {
                Outcome::Sign(Sign::Positive | Sign::Negative) => {
                    noncollinear = true;
                    break;
                }
                Outcome::Sign(Sign::Zero) => (),
                _ => return None,
            }
        }
        if !noncollinear {
            return Some(false);
        }
        for i in 0..points.len() {
            if cad_predicates::orient3d(
                &mut ctx,
                refs(anchors[0]),
                refs(anchors[1]),
                refs(anchors[2]),
                refs(i),
            )
            .ok()?
            .outcome
                != Outcome::Sign(Sign::Zero)
            {
                return Some(false);
            }
        }
        Some(true)
    })()
    .unwrap_or(false);
    out.work = ctx.work_used();
    out.planar_control_hull_certified = result;
    if result {
        out.normal = normal(points[anchors[0]], points[anchors[1]], points[anchors[2]]).ok();
    }
    out
}
fn normal(a: &[f64], b: &[f64], c: &[f64]) -> crate::Result<[[f64; 2]; 3]> {
    let mut u = [I::point(0.); 3];
    let mut v = u;
    for k in 0..3 {
        u[k] = I::point(b[k]).sub(I::point(a[k]))?;
        v[k] = I::point(c[k]).sub(I::point(a[k]))?;
    }
    let mut n = [[0.; 2]; 3];
    for k in 0..3 {
        let j = (k + 1) % 3;
        let l = (k + 2) % 3;
        let x = u[j].mul(v[l])?.sub(u[l].mul(v[j])?)?;
        n[k] = [x.lo, x.hi];
    }
    Ok(n)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn surface() -> crate::surface::Surface {
        crate::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 3., 0.]],
                vec![vec![2., 0., 2.], vec![2., 3., 2.]],
            ],
            weights: vec![vec![1., 0.5], vec![2., 1.]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn exact_parallel_axis_and_one_ulp_tilt() {
        let s = surface();
        let before = s.clone();
        // Retained plane x=z; normal direction (-1,0,1). Translation of
        // direction endpoints does not change the original exact difference.
        let a = [2., 4., 6.];
        let b = [1., 4., 7.];
        let r = inspect_parallel_axis(&s, a, b, 1000000);
        assert!(r.parallel);
        assert_eq!(r.reason, None);
        assert_eq!(s, before);
        assert!(!inspect_parallel_axis(&s, a, b, r.work - 1).parallel);
        let tilted = [1., 4., 7f64.next_up()];
        let r = inspect_parallel_axis(&s, a, tilted, 1000000);
        assert!(!r.parallel);
        assert_eq!(r.reason, Some("planes-not-parallel"));
        assert!(!inspect_parallel_axis(&s, a, a, 1000000).parallel);
        assert!(!inspect_parallel_axis(&s, a, b, 0).parallel);
        let mut warped = s;
        warped.control_points[1][1][2] = 2f64.next_up();
        assert!(!inspect_parallel_axis(&warped, a, b, 1000000).parallel);
    }
    #[test]
    fn original_oblique_normal_and_projection() {
        let s = surface();
        let before = s.clone();
        let r = inspect(&s, 1000000);
        assert!(r.planar_control_hull_certified);
        let n = r.normal.unwrap();
        for k in 0..3 {
            let x = [-6., 0., 6.][k];
            assert!(n[k][0] <= x && x <= n[k][1]);
        }
        let p =
            super::super::cap_projection_certificate::certify([[0., 0.], [0., 0.], [1., 1.]], n, 1)
                .unwrap();
        assert!(p.projection_regular);
        assert_eq!(s, before);
        assert!(!inspect(&s, r.work - 1).planar_control_hull_certified);
    }
    #[test]
    fn warped_degenerate_and_exhausted_have_no_normal() {
        let mut s = surface();
        s.control_points[1][1][2] = 2f64.next_up();
        let r = inspect(&s, 1000000);
        assert!(!r.planar_control_hull_certified);
        assert!(r.normal.is_none());
        let mut s = surface();
        s.control_points[1] = s.control_points[0].clone();
        assert!(inspect(&s, 1000000).normal.is_none());
        assert!(inspect(&surface(), 0).normal.is_none());
    }
}
