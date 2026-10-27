//! Complete multi-span tensor-border identity; no curve extraction or sampling.
use super::*;
fn clamped(k: &[f64], d: usize, n: usize) -> bool {
    k[..=d].iter().all(|&x| x == k[d]) && k[n..].iter().all(|&x| x == k[n])
}
pub(super) fn verify(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || p.degree != 1
        || p.control_points.len() != 2
        || p.weights[0] != p.weights[1]
        || !clamped(&c.knots, c.degree, c.control_points.len())
        || !clamped(&p.knots, p.degree, p.control_points.len())
        || !clamped(&s.knots_u, s.degree_u, s.control_points.len())
        || !clamped(&s.knots_v, s.degree_v, s.control_points[0].len())
    {
        return Ok(None);
    }
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let a = &p.control_points[0];
    let b = &p.control_points[1];
    for axis in 0..2 {
        let fixed = 1 - axis;
        if a[fixed] != b[fixed] {
            continue;
        }
        let at = if a[fixed] == domain[fixed][0] {
            0
        } else if a[fixed] == domain[fixed][1] {
            if fixed == 0 {
                s.control_points.len() - 1
            } else {
                s.control_points[0].len() - 1
            }
        } else {
            continue;
        };
        let forward = a[axis] == domain[axis][0] && b[axis] == domain[axis][1];
        let backward = a[axis] == domain[axis][1] && b[axis] == domain[axis][0];
        if !forward && !backward {
            continue;
        }
        let (degree, knots) = if axis == 0 {
            (s.degree_u, &s.knots_u)
        } else {
            (s.degree_v, &s.knots_v)
        };
        let target = if axis == 0 {
            s.control_points
                .iter()
                .zip(&s.weights)
                .map(|(row, w)| [row[at][0], row[at][1], row[at][2], w[at]])
                .collect::<Vec<_>>()
        } else {
            s.control_points[at]
                .iter()
                .zip(&s.weights[at])
                .map(|(q, w)| [q[0], q[1], q[2], *w])
                .collect()
        };
        // Existing Bezier composition retains its richer difference semantics.
        if (s.control_points.len() == s.degree_u + 1 && s.control_points[0].len() == s.degree_v + 1)
            || degree != c.degree
            || target.len() != c.control_points.len()
        {
            continue;
        }
        let reverse = backward ^ reversed;
        let ck = if reverse {
            c.knots.iter().rev().map(|x| -x).collect::<Vec<_>>()
        } else {
            c.knots.clone()
        };
        if ck != *knots {
            continue;
        }
        let mut actual = c
            .control_points
            .iter()
            .zip(&c.weights)
            .map(|(q, w)| [q[0], q[1], q[2], *w])
            .collect::<Vec<_>>();
        if reverse {
            actual.reverse();
        }
        // A common positive weight on each polynomial row cancels exactly, but
        // same-definition identity cannot prove that gauge. Let the existing
        // natural-boundary proof handle it instead of returning MissingProof.
        let polynomial_weight_gauge = actual[0][3] != target[0][3]
            && actual.iter().zip(&target).all(|(a, b)| a[..3] == b[..3])
            && actual.iter().all(|q| q[3] == actual[0][3])
            && target.iter().all(|q| q[3] == target[0][3]);
        if polynomial_weight_gauge {
            return Ok(None);
        }
        let mut values = actual
            .iter()
            .chain(&target)
            .flatten()
            .copied()
            .chain(ck.iter().chain(knots).copied())
            .collect::<Vec<_>>();
        // Bind the structural full-border preconditions to the same immutable
        // source context, including the actual pcurve and both support axes.
        values.extend(p.control_points.iter().flatten().copied());
        values.extend(p.weights.iter().copied());
        values.extend(p.knots.iter().copied());
        values.extend(s.knots_u.iter().chain(&s.knots_v).copied());
        let source = SourceArena::authored(
            "owned-multispan-tensor-border",
            1,
            values
                .into_iter()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .map_err(|_| {
            crate::Error::new("NURBS_INVALID_INPUT", "Invalid owned tensor border source")
        })?;
        let mut index = 0;
        let mut leaf = || {
            let r = source.leaf(index).unwrap();
            index += 1;
            r
        };
        let aa = (0..actual.len())
            .map(|_| std::array::from_fn(|_| leaf()))
            .collect::<Vec<_>>();
        let bb = (0..target.len())
            .map(|_| std::array::from_fn(|_| leaf()))
            .collect::<Vec<_>>();
        let ak = (0..ck.len()).map(|_| leaf()).collect::<Vec<_>>();
        let bk = (0..knots.len()).map(|_| leaf()).collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut context = PredicateContext::new(
            &source,
            &tolerance,
            Limits {
                max_work,
                ..Limits::default()
            },
            None,
        );
        return cad_predicates::rational_bspline_same_definition(
            &mut context,
            degree,
            &aa,
            &ak,
            &bb,
            &bk,
        )
        .map(Some)
        .map_err(|_| crate::Error::new("NURBS_INVALID_INPUT", "Invalid owned B-spline identity"));
    }
    Ok(None)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_multispan_rows_are_owned_with_exact_reversal_and_bounded_work() {
        use cad_predicates::{BezierIdentity as B, Reason};
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 0.5, 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            periodic_u: false,
            periodic_v: false,
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.2], vec![1., 1., 0.2]],
                vec![vec![2., 0., 0.], vec![2., 1., 0.]],
            ],
            weights: vec![vec![1., 1.], vec![0.5, 0.7], vec![1., 1.]],
        };
        let c = Curve {
            degree: 1,
            knots: s.knots_u.clone(),
            control_points: s.control_points.iter().map(|r| r[0].clone()).collect(),
            weights: s.weights.iter().map(|r| r[0]).collect(),
            periodic: false,
        };
        let p = Curve {
            degree: 1,
            knots: vec![4., 4., 7., 7.],
            control_points: vec![vec![0., 0.], vec![1., 0.]],
            weights: vec![2., 2.],
            periodic: false,
        };
        let proof = super::super::verify_exact(&c, &p, &s, false, 10000)
            .unwrap()
            .unwrap();
        assert_eq!(proof.outcome, B::Equal);
        assert!(proof.work_used > 0);
        let mut r = c.clone();
        r.control_points.reverse();
        r.weights.reverse();
        r.knots = c.knots.iter().rev().map(|x| -x).collect();
        assert_eq!(
            super::super::verify_exact(&r, &p, &s, true, 10000)
                .unwrap()
                .unwrap()
                .outcome,
            B::Equal
        );
        let mut wrong = c.clone();
        wrong.control_points[1][2] += 1e-5;
        assert_ne!(
            super::super::verify_exact(&wrong, &p, &s, false, 10000)
                .unwrap()
                .unwrap()
                .outcome,
            B::Equal
        );
        let mut wrong = c.clone();
        wrong.weights[1] = 0.6;
        assert_ne!(
            super::super::verify_exact(&wrong, &p, &s, false, 10000)
                .unwrap()
                .unwrap()
                .outcome,
            B::Equal
        );
        let mut partial = p.clone();
        partial.control_points[1][0] = 0.9;
        assert!(
            super::super::verify_exact(&c, &partial, &s, false, 10000)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            super::super::verify_exact(&c, &p, &s, false, 0)
                .unwrap()
                .unwrap()
                .outcome,
            B::Indeterminate(Reason::ResourceLimit)
        );
    }
}
