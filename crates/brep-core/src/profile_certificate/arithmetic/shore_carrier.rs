//! Original algebraic carrier interval confined to one monotone chart shore.
use super::*;
use branch_neighbourhood::ParameterValue;
use std::cmp::Ordering;
pub(super) fn bound_sign(bound: &ParameterValue, q: &Q, refinements: usize) -> Result<i8> {
    let mut p = bound.value.clone();
    if p.is_empty() {
        p.push(-q.clone());
    } else {
        p[0] -= q;
    }
    event_coordinates::sign_at(&p, &bound.primitive, &bound.interval, refinements)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::profile_certificate) enum Membership {
    Entire,
    Absent,
    Mixed,
}
pub(in crate::profile_certificate) fn classify(
    cells: &[(Vec<[Q; 3]>, [Q; 2])],
    bounds: &[ParameterValue; 2],
    axis: usize,
    shore: &Q,
    refinements: usize,
) -> Result<(Membership, Vec<(usize, bool)>)> {
    if axis > 1 || refinements > 512 {
        return Err(refused("Invalid original shore classification allowance"));
    }
    let mut active = Vec::new();
    for (index, (net, domain)) in cells.iter().enumerate() {
        if bound_sign(&bounds[0], &domain[1], refinements)? >= 0
            || bound_sign(&bounds[1], &domain[0], refinements)? <= 0
        {
            continue;
        }
        if net.iter().any(|p| p[2] <= Q::zero()) {
            return Err(refused(
                "Original shore classification has nonpositive weights",
            ));
        }
        active.push((index, net.iter().all(|p| p[axis] == shore * &p[2])));
    }
    if active.is_empty() {
        return Err(refused(
            "Original shore classification has no actual carrier interior",
        ));
    }
    let class = if active.iter().all(|(_, same)| *same) {
        Membership::Entire
    } else if active.iter().all(|(_, same)| !*same) {
        Membership::Absent
    } else {
        Membership::Mixed
    };
    Ok((class, active))
}
pub(in crate::profile_certificate) fn prove(
    cells: &[(Vec<[Q; 3]>, [Q; 2])],
    bounds: &[ParameterValue; 2],
    axis: usize,
    shore: &Q,
    refinements: usize,
    matrix: usize,
) -> Result<(Vec<usize>, i8)> {
    if axis > 1 || refinements > 512 || !(1..=128).contains(&matrix) {
        return Err(refused("Invalid original shore carrier allowance"));
    }
    let mut active = Vec::new();
    let mut direction = 0;
    let mut previous: Option<[Q; 2]> = None;
    for (index, (net, domain)) in cells.iter().enumerate() {
        if bound_sign(&bounds[0], &domain[1], refinements)? >= 0
            || bound_sign(&bounds[1], &domain[0], refinements)? <= 0
        {
            continue;
        }
        if net
            .iter()
            .any(|p| p[2] <= Q::zero() || p[axis] != shore * &p[2])
        {
            return Err(refused(
                "Actual original carrier is not identically on this shore",
            ));
        }
        let endpoint = |h: &[Q; 3]| [&h[0] / &h[2], &h[1] / &h[2]];
        if previous.as_ref().is_some_and(|p| p != &endpoint(&net[0])) {
            return Err(refused(
                "Original shore carrier has a discontinuous knot join",
            ));
        }
        previous = Some(endpoint(net.last().unwrap()));
        let y = power(net, 1 - axis);
        let w = power(net, 2);
        let g = sub(&product(&derivative(&y), &w), &product(&y, &derivative(&w)));
        if g.is_empty() {
            return Err(refused(
                "Original shore carrier contains a constant image cell",
            ));
        }
        for root in
            algebraic_events::isolates(&g, Q::zero(), Q::one(), &Q::new(1.into(), 1024.into()))?
        {
            let lower =
                selected_winding::compare(&g, &root, domain, &bounds[0], refinements, matrix)?;
            let upper =
                selected_winding::compare(&g, &root, domain, &bounds[1], refinements, matrix)?;
            if lower == Ordering::Greater && upper == Ordering::Less {
                return Err(refused(
                    "Original shore carrier has an interior transverse critical parameter",
                ));
            }
        }
        let natural = carrier_sides::interior(
            &vec![(net.clone(), domain.clone())],
            bounds.clone(),
            refinements,
        )?;
        let local = (&natural - &domain[0]) / (&domain[1] - &domain[0]);
        let value = at(&g, &local);
        let sign = if value > Q::zero() {
            1
        } else if value < Q::zero() {
            -1
        } else {
            return Err(refused("Original shore carrier regular sign is unresolved"));
        };
        if direction != 0 && direction != sign {
            return Err(refused(
                "Original shore carrier reverses across a knot join",
            ));
        }
        direction = sign;
        active.push(index);
    }
    if active.is_empty() {
        return Err(refused(
            "Original shore carrier has no actual source cell interior",
        ));
    }
    Ok((active, direction))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn bound(q: Q) -> ParameterValue {
        ParameterValue {
            primitive: vec![-q.clone(), Q::one()],
            value: vec![Q::zero(), Q::one()],
            interval: [q.clone(), q],
        }
    }
    #[test]
    fn shore_proof_rejects_interior_turns_discontinuities_and_knot_reversals() {
        let q = |i: i64| Q::from_integer(i.into());
        let half = Q::new(1.into(), 2.into());
        let bounds = [bound(q(0)), bound(q(1))];
        let turn = vec![(
            vec![[q(0), q(0), q(1)], [q(0), q(2), q(1)], [q(0), q(0), q(1)]],
            [q(0), q(1)],
        )];
        assert!(prove(&turn, &bounds, 0, &q(0), 192, 64).is_err());
        let mut cells = vec![
            (
                vec![[q(0), q(0), q(1)], [q(0), half.clone(), q(1)]],
                [q(0), half.clone()],
            ),
            (
                vec![[q(0), half.clone(), q(1)], [q(0), q(1), q(1)]],
                [half.clone(), q(1)],
            ),
        ];
        assert_eq!(
            prove(&cells, &bounds, 0, &q(0), 192, 64).unwrap(),
            (vec![0, 1], 1)
        );
        cells[1].0[0][1] = Q::new(3.into(), 4.into());
        assert!(prove(&cells, &bounds, 0, &q(0), 192, 64).is_err());
        cells[1].0[0][1] = half;
        cells[1].0[1][1] = q(0);
        assert!(prove(&cells, &bounds, 0, &q(0), 192, 64).is_err());
    }
}
#[cfg(test)]
mod classification_tests {
    use super::*;
    #[test]
    fn classification_distinguishes_mixed_identity_and_keeps_budget_failures_errors() {
        let q = |n: i64| Q::from_integer(n.into());
        let bound = |v: Q| ParameterValue {
            primitive: vec![-v.clone(), q(1)],
            value: vec![q(0), q(1)],
            interval: [v.clone(), v],
        };
        let half = Q::new(1.into(), 2.into());
        let bounds = [bound(q(0)), bound(q(1))];
        let cells = vec![
            (
                vec![[q(0), q(0), q(1)], [q(0), half.clone(), q(1)]],
                [q(0), half.clone()],
            ),
            (
                vec![[q(0), half.clone(), q(1)], [q(1), q(1), q(1)]],
                [half, q(1)],
            ),
        ];
        let (class, active) = classify(&cells, &bounds, 0, &q(0), 192).unwrap();
        assert_eq!(class, Membership::Mixed);
        assert_eq!(active, vec![(0, true), (1, false)]);
        assert_eq!(
            classify(&cells, &bounds, 0, &q(2), 192).unwrap().0,
            Membership::Absent
        );
        assert!(classify(&cells, &bounds, 0, &q(0), 513).is_err());
        let algebraic = [
            ParameterValue {
                primitive: vec![q(-2), q(0), q(1)],
                value: vec![q(0), q(1)],
                interval: [q(1), q(2)],
            },
            bound(q(2)),
        ];
        let source = vec![(vec![[q(0), q(0), q(1)], [q(0), q(1), q(1)]], [q(0), q(2)])];
        assert!(classify(&source, &algebraic, 0, &q(0), 0).is_err());
        assert_eq!(
            classify(&source, &algebraic, 0, &q(0), 192).unwrap().0,
            Membership::Entire
        );
    }
}
