//! Exact original line image and whole-source forward motion.
//! Positive weights and clamped endpoints alone do not prove monotonicity.
use crate::sweeps::progressive_miter::{scalar_certificate, scalar_certificate::Status};
use crate::{check, curve::Curve, Result};
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};

#[derive(Clone, Debug)]
pub(super) struct Report {
    pub certified: bool,
    pub cells: usize,
    pub reason: Option<&'static str>,
}
pub(super) fn certify(path: &Curve, max_cells: usize) -> Result<Report> {
    path.validate()?;
    check(
        max_cells <= 100000 && path.control_points.iter().all(|p| p.len() == 3),
        "Invalid original line certificate input",
    )?;
    let mut out = Report {
        certified: false,
        cells: 0,
        reason: Some("source-line-correspondence-unproved"),
    };
    let p = path.degree;
    let n = path.control_points.len();
    let [a, b] = path.domain();
    if path.periodic
        || path.knots[..=p].iter().any(|u| *u != a)
        || path.knots[path.knots.len() - p - 1..]
            .iter()
            .any(|u| *u != b)
    {
        return Ok(out);
    }
    let Some(axis) = (0..3).find(|&k| path.control_points[0][k] != path.control_points[n - 1][k])
    else {
        return Ok(out);
    };
    if p == 1 && n == 2 {
        out.certified = true;
        out.reason = None;
        return Ok(out);
    }
    if max_cells == 0 || n > 65536 / 3 {
        return Ok(out);
    }
    let values = path
        .control_points
        .iter()
        .flatten()
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("sweep-original-line-image", 1, values) else {
        return Ok(out);
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: max_cells as u64,
            ..Limits::default()
        },
        None,
    );
    for i in 1..n - 1 {
        for k in 0..3 {
            let axes = [k, (k + 1) % 3];
            let refs = |i| axes.map(|k| source.leaf(3 * i + k).unwrap());
            let decision = cad_predicates::orient2d(&mut ctx, refs(0), refs(n - 1), refs(i));
            out.cells = ctx.work_used() as usize;
            match decision.map(|d| d.outcome) {
                Ok(Outcome::Sign(Sign::Zero)) => {}
                Ok(Outcome::Sign(_)) => {
                    out.reason = Some("source-line-coefficients-noncollinear");
                    return Ok(out);
                }
                _ => return Ok(out),
            }
        }
    }
    let forward = path.control_points[n - 1][axis] > path.control_points[0][axis];
    // On one clamped rational Bezier span, expand X'W-XW' in
    // unordered Bernstein pairs. Each term has sign (x_j-x_i)(j-i)
    // times positive weights and nonnegative Bernstein factors. Strictly
    // ordered source coordinates make every interior term positive; the
    // adjacent endpoint terms prove strict endpoint speed as well. Binary64
    // comparisons here are exact order comparisons, with no rounded gap.
    if n == p + 1 {
        let mut ordered = true;
        for pair in path.control_points.windows(2) {
            if out.cells == max_cells {
                return Ok(out);
            }
            out.cells += 1;
            ordered &= if forward {
                pair[1][axis] > pair[0][axis]
            } else {
                pair[1][axis] < pair[0][axis]
            };
        }
        if ordered {
            out.certified = true;
            out.reason = None;
            return Ok(out);
        }
    }
    let mut coordinate = path.clone();
    for pole in &mut coordinate.control_points {
        *pole = vec![pole[axis], 0., 0.];
    }
    let mut pending = vec![[0., 1.]];
    out.reason = Some("source-line-forward-motion-unproved");
    while let Some(interval) = pending.pop() {
        if out.cells == max_cells {
            return Ok(out);
        }
        let r =
            scalar_certificate::certify_traversal(&coordinate, interval, max_cells - out.cells)?;
        out.cells += r.cells;
        if r.status == Status::Certified {
            let first = r.first.unwrap();
            if (forward && first[0] > 0.) || (!forward && first[1] < 0.) {
                continue;
            }
            if (forward && first[1] <= 0.) || (!forward && first[0] >= 0.) {
                return Ok(out);
            }
        }
        let mid = interval[0] + (interval[1] - interval[0]) * 0.5;
        if r.cells == 0 || mid <= interval[0] || mid >= interval[1] {
            return Ok(out);
        }
        pending.push([mid, interval[1]]);
        pending.push([interval[0], mid]);
    }
    out.certified = true;
    out.reason = None;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forward_spans_require_exact_position_at_full_multiplicity_knots() {
        let path = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 0.5, 1., 1.],
            control_points: vec![
                vec![0.; 3],
                vec![1., 0., 0.],
                vec![2., 0., 0.],
                vec![3., 0., 0.],
            ],
            weights: vec![1., 2., 3., 1.],
            periodic: false,
        };
        // Native Curve validation rejects disconnected spans before any
        // positive derivative certificate can be published.
        assert!(path.validate().is_err());
        assert!(certify(&path, 10000).is_err());
        let path = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0.; 3], vec![1., 0., 0.], vec![3., 0., 0.]],
            weights: vec![1., 2., 1.],
            periodic: false,
        };
        let proof = certify(&path, 10000).unwrap();
        assert!(proof.certified, "{proof:?}");
        assert!(!certify(&path, proof.cells - 1).unwrap().certified);
    }
    #[test]
    fn exact_line_image_requires_full_forward_cover_and_shared_work() {
        for weights in [vec![1.; 4], vec![1., 2., 0.75, 1.25]] {
            let path = Curve {
                degree: 3,
                knots: vec![2., 2., 2., 2., 5., 5., 5., 5.],
                control_points: vec![
                    vec![0.; 3],
                    vec![1., 1., 2.],
                    vec![7., 7., 14.],
                    vec![10., 10., 20.],
                ],
                weights,
                periodic: false,
            };
            let proof = certify(&path, 10000).unwrap();
            assert!(proof.certified, "{proof:?}");
            assert!(proof.cells > 0);
            assert!(!certify(&path, 0).unwrap().certified);
            assert!(!certify(&path, proof.cells - 1).unwrap().certified);
            assert!(certify(&path.reverse().unwrap(), 10000).unwrap().certified);
            let mut bent = path.clone();
            bent.control_points[1][2] = 2_f64.next_up();
            assert_eq!(
                certify(&bent, 10000).unwrap().reason,
                Some("source-line-coefficients-noncollinear")
            );
        }
        let reversing = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0.; 3], vec![2., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert!(!certify(&reversing, 10000).unwrap().certified);
        let stationary = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0.; 3],
                vec![1., 0., 0.],
                vec![0., 0., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(!certify(&stationary, 10000).unwrap().certified);
    }
}
