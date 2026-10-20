//! Complete original-cell ray census clipped to owned algebraic carrier cuts.
use super::*;
use branch_neighbourhood::ParameterValue;
use std::cmp::Ordering;
pub(in crate::profile_certificate) fn compare(
    f: &Poly,
    root: &[Q; 2],
    domain: &[Q; 2],
    bound: &ParameterValue,
    refinements: usize,
    matrix: usize,
) -> Result<Ordering> {
    parameter_order::compare(
        f,
        &vec![domain[0].clone(), &domain[1] - &domain[0]],
        root,
        &bound.primitive,
        &bound.value,
        &bound.interval,
        refinements,
        matrix,
    )
}
fn germ(g: &Poly, root: &[Q; 2], right: bool, refinements: usize) -> Result<i8> {
    let mut d = g.clone();
    let mut order = 0;
    loop {
        let sign = event_coordinates::sign_at(&d, g, root, refinements)?;
        if sign != 0 {
            return Ok(if right || order % 2 == 0 { sign } else { -sign });
        }
        d = derivative(&d);
        order += 1;
        if d.is_empty() {
            return Err(refused("Selected ray has no finite original cut germ"));
        }
    }
}
pub(in crate::profile_certificate) fn contribution(
    net: &[[Q; 3]],
    domain: &[Q; 2],
    bounds: &[ParameterValue; 2],
    point: &[Q; 2],
    refinements: usize,
    matrix: usize,
) -> Result<i64> {
    contribution_status(net, domain, bounds, point, refinements, matrix)?
        .ok_or_else(|| refused("Winding query lies on the selected original source curve"))
}
/// None is a PROVED boundary point on the closed selected interval. A full-cell
/// boundary outside the owned interval is not a selected boundary or failure.
pub(in crate::profile_certificate) fn contribution_status(
    net: &[[Q; 3]],
    domain: &[Q; 2],
    bounds: &[ParameterValue; 2],
    point: &[Q; 2],
    refinements: usize,
    matrix: usize,
) -> Result<Option<i64>> {
    if net.len() < 2
        || net.len() > 9
        || net.iter().any(|p| p[2] <= Q::zero())
        || domain[0] >= domain[1]
        || refinements > 512
        || !(1..=128).contains(&matrix)
    {
        return Err(refused("Invalid selected original ray context"));
    }
    let order = |a: &ParameterValue, b: &ParameterValue| {
        parameter_order::compare(
            &a.primitive,
            &a.value,
            &a.interval,
            &b.primitive,
            &b.value,
            &b.interval,
            refinements,
            matrix,
        )
    };
    let rational = |q: &Q| ParameterValue {
        primitive: vec![Q::zero(), Q::one()],
        value: trim(vec![q.clone()]),
        interval: [Q::zero(), Q::zero()],
    };
    if order(&bounds[0], &bounds[1])? != Ordering::Less
        || order(&bounds[0], &rational(&domain[0]))? == Ordering::Less
        || order(&bounds[1], &rational(&domain[1]))? == Ordering::Greater
    {
        return Err(refused(
            "Selected ray interval is not a positive owned subinterval of its original cell",
        ));
    }
    let w = power(net, 2);
    let g = sub(
        &power(net, 1),
        &w.iter().map(|w| w * &point[1]).collect::<Poly>(),
    );
    if g.is_empty() {
        let roots = shore_events::produce(
            net,
            domain,
            0,
            &point[0],
            &Q::new(1.into(), 1024.into()),
            refinements,
        )?;
        let Some(roots) = roots else {
            return Ok(None);
        };
        for root in roots {
            let lower = compare(
                &root.primitive,
                &root.root,
                domain,
                &bounds[0],
                refinements,
                matrix,
            )?;
            let upper = compare(
                &root.primitive,
                &root.root,
                domain,
                &bounds[1],
                refinements,
                matrix,
            )?;
            if lower != Ordering::Less && upper != Ordering::Greater {
                return Ok(None);
            }
        }
        return Ok(Some(0));
    }
    let census = ray_winding::produce_for_clipping(net, point)?;
    ray_winding::verify_for_clipping(net, point, &census)?;
    let mut total = 0i64;
    for root in census.roots {
        let interval = [
            super::super::algebraic_intersections::decode_ratio(&root.interval[0])?,
            super::super::algebraic_intersections::decode_ratio(&root.interval[1])?,
        ];
        let lower = compare(&g, &interval, domain, &bounds[0], refinements, matrix)?;
        let upper = compare(&g, &interval, domain, &bounds[1], refinements, matrix)?;
        if lower == Ordering::Less || upper == Ordering::Greater || root.sign_x < 0 {
            continue;
        }
        if root.sign_x == 0 {
            return Ok(None);
        }
        let jump = if lower == Ordering::Equal {
            if event_coordinates::zero_at(&vec![-Q::one(), Q::one()], &g, &interval)? {
                0
            } else {
                i8::from(germ(&g, &interval, true, refinements)? > 0)
            }
        } else if upper == Ordering::Equal {
            if event_coordinates::zero_at(&vec![Q::zero(), Q::one()], &g, &interval)? {
                0
            } else {
                -i8::from(germ(&g, &interval, false, refinements)? > 0)
            }
        } else {
            root.jump
        };
        total = total
            .checked_add(i64::from(jump))
            .ok_or_else(|| refused("Selected original ray winding overflow"))?;
    }
    Ok(Some(total))
}
/// Signed change of winding across a finite axis-aligned path segment.
/// The source retains its native clock and exact algebraic cuts. None proves
/// a selected-source endpoint contact. A coincident supporting cell is an
/// explicit nongeneric path refusal, never silently counted as zero crossings.
#[cfg(test)]
pub(in crate::profile_certificate) fn axis_segment(
    net: &[[Q; 3]],
    domain: &[Q; 2],
    bounds: &[ParameterValue; 2],
    endpoints: &[[Q; 2]; 2],
    refinements: usize,
    matrix: usize,
) -> Result<Option<i64>> {
    if endpoints[0][0] != endpoints[1][0] && endpoints[0][1] != endpoints[1][1] {
        return Err(refused("Exact selected crossing path is not axis aligned"));
    }
    segment(net, domain, bounds, endpoints, refinements, matrix)
}
/// Arbitrary finite rational path with a determinant-positive exact source view.
/// No normalization, square root or rounded transformed Curve is used.
pub(in crate::profile_certificate) fn segment(
    net: &[[Q; 3]],
    domain: &[Q; 2],
    bounds: &[ParameterValue; 2],
    endpoints: &[[Q; 2]; 2],
    refinements: usize,
    matrix: usize,
) -> Result<Option<i64>> {
    let [a, b] = endpoints;
    if a == b {
        return Ok(contribution_status(net, domain, bounds, a, refinements, matrix)?.map(|_| 0));
    }
    let d = [&b[0] - &a[0], &b[1] - &a[1]];
    // (X',Y')=(d.X,det(d,X)). Determinant |d|^2 is positive,
    // hence source orientation and native parameter germs are unchanged.
    let transform = |p: &[Q; 2]| [&d[0] * &p[0] + &d[1] * &p[1], &d[0] * &p[1] - &d[1] * &p[0]];
    let transformed = net
        .iter()
        .map(|p| {
            [
                &d[0] * &p[0] + &d[1] * &p[1],
                &d[0] * &p[1] - &d[1] * &p[0],
                p[2].clone(),
            ]
        })
        .collect::<Vec<_>>();
    let points = [transform(a), transform(b)];
    let net = transformed.as_slice();
    let left = contribution_status(net, domain, bounds, &points[0], refinements, matrix)?;
    let right = contribution_status(net, domain, bounds, &points[1], refinements, matrix)?;
    let (Some(left), Some(right)) = (left, right) else {
        return Ok(None);
    };
    let level = sub(
        &power(net, 1),
        &power(net, 2)
            .iter()
            .map(|w| w * &points[0][1])
            .collect::<Poly>(),
    );
    if level.is_empty() {
        return Err(refused(
            "Finite crossing path is coincident with an original supporting cell; choose a generic path",
        ));
    }
    Ok(Some(right.checked_sub(left).ok_or_else(|| {
        refused("Selected finite crossing count overflow")
    })?))
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
    fn oblique_nonlinear_crossings_keep_native_cuts_and_even_tangencies() {
        let q = |n: i64| Q::from_integer(n.into());
        let half = Q::new(1.into(), 2.into());
        let net = [
            [q(0), q(0), q(1)],
            [half.clone(), q(0), q(1)],
            [q(1), q(1), q(1)],
        ];
        let path = [
            [Q::new(1.into(), 4.into()), Q::new(1.into(), 8.into())],
            [Q::new(3.into(), 4.into()), Q::new(3.into(), 8.into())],
        ];
        let domain = [q(2), q(6)];
        let full = [bound(q(3)), bound(q(6))];
        let count = |cuts: &[ParameterValue; 2], p: &[[Q; 2]; 2]| {
            segment(&net, &domain, cuts, p, 192, 64).unwrap().unwrap()
        };
        assert_eq!(count(&full, &path), -1);
        assert_eq!(count(&full, &[path[1].clone(), path[0].clone()]), 1);
        assert_eq!(
            count(&[bound(q(3)), bound(q(4))], &path) + count(&[bound(q(4)), bound(q(6))], &path),
            -1
        );
        let tangent = [
            [q(-1), Q::new((-5).into(), 4.into())],
            [q(2), Q::new(7.into(), 4.into())],
        ];
        assert_eq!(count(&full, &tangent), 0);
        let contact = [[q(-1), q(-1)], [half.clone(), Q::new(1.into(), 4.into())]];
        assert_eq!(
            segment(&net, &domain, &full, &contact, 192, 64).unwrap(),
            None
        );
    }
    #[test]
    fn finite_axis_paths_preserve_orientation_composition_and_closed_wire_winding() {
        let q = |n: i64| Q::from_integer(n.into());
        let half = Q::new(1.into(), 2.into());
        let p = |x: Q, y: Q| [x, y, q(1)];
        let edges = [
            vec![p(q(0), q(0)), p(q(1), q(0))],
            vec![p(q(1), q(0)), p(q(1), q(1))],
            vec![p(q(1), q(1)), p(q(0), q(1))],
            vec![p(q(0), q(1)), p(q(0), q(0))],
        ];
        let bounds = [bound(q(0)), bound(q(1))];
        let sum = |a: [Q; 2], b: [Q; 2]| -> i64 {
            edges
                .iter()
                .map(|net| {
                    axis_segment(
                        net,
                        &[q(0), q(1)],
                        &bounds,
                        &[a.clone(), b.clone()],
                        192,
                        64,
                    )
                    .unwrap()
                    .unwrap()
                })
                .sum()
        };
        let inside = [half.clone(), half.clone()];
        let left = [q(-1), half.clone()];
        let right = [q(2), half.clone()];
        assert_eq!(sum(left.clone(), inside.clone()), 1);
        assert_eq!(sum(inside.clone(), right.clone()), -1);
        assert_eq!(sum(left.clone(), right.clone()), 0);
        assert_eq!(
            sum(left.clone(), inside.clone()),
            -sum(inside.clone(), left)
        );
        assert_eq!(sum([half.clone(), q(-1)], inside.clone()), 1);
        assert_eq!(sum(inside, [half, q(2)]), -1);
    }
    #[test]
    fn finite_crossings_clip_native_cuts_and_do_not_hide_coincident_paths() {
        let q = |n: i64| Q::from_integer(n.into());
        let half = Q::new(1.into(), 2.into());
        let net = [[q(0), q(0), q(1)], [q(0), q(1), q(1)]];
        let domain = [q(2), q(6)];
        let bounds = [bound(q(4)), bound(q(6))];
        let quarter = Q::new(1.into(), 4.into());
        assert_eq!(
            axis_segment(
                &net,
                &domain,
                &bounds,
                &[[q(-1), quarter.clone()], [q(0), quarter]],
                192,
                64
            )
            .unwrap(),
            Some(0)
        );
        assert_eq!(
            axis_segment(
                &net,
                &domain,
                &bounds,
                &[[q(-1), half.clone()], [q(0), half.clone()]],
                192,
                64
            )
            .unwrap(),
            None
        );
        assert!(
            axis_segment(
                &net,
                &domain,
                &bounds,
                &[[q(0), q(-1)], [q(0), q(2)]],
                192,
                64
            )
            .is_err()
        );
        assert!(
            axis_segment(
                &net,
                &domain,
                &bounds,
                &[[q(-1), q(-1)], [q(2), q(2)]],
                192,
                64
            )
            .is_err()
        );
        assert_eq!(
            axis_segment(
                &net,
                &domain,
                &bounds,
                &[[q(-1), half.clone()], [q(-1), half]],
                192,
                64
            )
            .unwrap(),
            Some(0)
        );
    }
    #[test]
    fn boundary_membership_follows_actual_owned_interval_including_irrational_cuts() {
        let q = |n: i64| Q::from_integer(n.into());
        let half = Q::new(1.into(), 2.into());
        let net = vec![[q(0), q(0), q(1)], [q(0), q(1), q(1)]];
        let selected = [bound(half.clone()), bound(q(1))];
        assert_eq!(
            contribution_status(
                &net,
                &[q(0), q(1)],
                &selected,
                &[q(0), Q::new(1.into(), 4.into())],
                192,
                64
            )
            .unwrap(),
            Some(0)
        );
        assert_eq!(
            contribution(
                &net,
                &[q(0), q(1)],
                &selected,
                &[q(0), Q::new(1.into(), 4.into())],
                192,
                64
            )
            .unwrap(),
            0
        );
        assert_eq!(
            contribution_status(
                &net,
                &[q(0), q(1)],
                &selected,
                &[q(0), Q::new(3.into(), 4.into())],
                192,
                64
            )
            .unwrap(),
            None
        );
        assert!(
            contribution(
                &net,
                &[q(0), q(1)],
                &selected,
                &[q(0), half.clone()],
                192,
                64
            )
            .is_err()
        );
        assert_eq!(
            contribution(
                &net,
                &[q(0), q(1)],
                &selected,
                &[q(-1), Q::new(3.into(), 4.into())],
                192,
                64
            )
            .unwrap(),
            1
        );
        assert_eq!(
            contribution_status(
                &net,
                &[q(2), q(6)],
                &[bound(q(4)), bound(q(6))],
                &[q(0), Q::new(1.into(), 4.into())],
                192,
                64
            )
            .unwrap(),
            Some(0)
        );
        assert_eq!(
            contribution_status(
                &net,
                &[q(2), q(6)],
                &[bound(q(4)), bound(q(6))],
                &[q(0), Q::new(3.into(), 4.into())],
                192,
                64
            )
            .unwrap(),
            None
        );
        let quadratic = vec![[q(0), q(0), q(1)], [q(0), q(0), q(1)], [q(0), q(1), q(1)]];
        let root = ParameterValue {
            primitive: vec![-half.clone(), q(0), q(1)],
            value: vec![q(0), q(1)],
            interval: [Q::new(7.into(), 10.into()), Q::new(4.into(), 5.into())],
        };
        assert_eq!(
            contribution_status(
                &quadratic,
                &[q(0), q(1)],
                &[bound(Q::new(3.into(), 4.into())), bound(q(1))],
                &[q(0), half.clone()],
                192,
                64
            )
            .unwrap(),
            Some(0)
        );
        assert_eq!(
            contribution_status(
                &quadratic,
                &[q(0), q(1)],
                &[root, bound(q(1))],
                &[q(0), half],
                192,
                64
            )
            .unwrap(),
            None
        );
    }
    #[test]
    fn horizontal_and_stationary_boundary_censuses_are_clipped_before_admission() {
        let q = |n: i64| Q::from_integer(n.into());
        let half = Q::new(1.into(), 2.into());
        let bounds = [bound(half.clone()), bound(q(1))];
        let horizontal = vec![[q(0), q(0), q(1)], [q(1), q(0), q(1)]];
        assert_eq!(
            contribution_status(
                &horizontal,
                &[q(0), q(1)],
                &bounds,
                &[Q::new(1.into(), 4.into()), q(0)],
                192,
                64
            )
            .unwrap(),
            Some(0)
        );
        assert_eq!(
            contribution_status(
                &horizontal,
                &[q(0), q(1)],
                &bounds,
                &[Q::new(3.into(), 4.into()), q(0)],
                192,
                64
            )
            .unwrap(),
            None
        );
        assert_eq!(
            contribution_status(&horizontal, &[q(0), q(1)], &bounds, &[q(2), q(0)], 192, 64)
                .unwrap(),
            Some(0)
        );
        let tangent = vec![[q(0), q(1), q(1)], [q(0), q(-1), q(1)], [q(0), q(1), q(1)]];
        assert_eq!(
            contribution_status(&tangent, &[q(0), q(1)], &bounds, &[q(0), q(0)], 192, 64).unwrap(),
            None
        );
        assert_eq!(
            contribution_status(
                &tangent,
                &[q(0), q(1)],
                &[bound(Q::new(3.into(), 4.into())), bound(q(1))],
                &[q(0), q(0)],
                192,
                64
            )
            .unwrap(),
            Some(0)
        );
        assert!(
            contribution_status(
                &horizontal,
                &[q(0), q(1)],
                &[bound(q(1)), bound(half)],
                &[q(2), q(0)],
                192,
                64
            )
            .is_err()
        );
    }
    #[test]
    fn clipped_cut_germs_partition_crossings_and_even_tangencies() {
        let q = |n: i64| Q::from_integer(n.into());
        let half = Q::new(1.into(), 2.into());
        let up = vec![[q(1), q(-1), q(1)], [q(1), q(1), q(1)]];
        let down = vec![[q(1), q(1), q(1)], [q(1), q(-1), q(1)]];
        let tangent = vec![[q(1), q(1), q(1)], [q(1), q(-1), q(1)], [q(1), q(1), q(1)]];
        let domain = [q(0), q(1)];
        let point = [q(0), q(0)];
        let lower = [bound(q(0)), bound(half.clone())];
        let upper = [bound(half), bound(q(1))];
        assert_eq!(
            contribution(&up, &domain, &lower, &point, 192, 64).unwrap(),
            0
        );
        assert_eq!(
            contribution(&up, &domain, &upper, &point, 192, 64).unwrap(),
            1
        );
        assert_eq!(
            contribution(&down, &domain, &lower, &point, 192, 64).unwrap(),
            -1
        );
        assert_eq!(
            contribution(&down, &domain, &upper, &point, 192, 64).unwrap(),
            0
        );
        assert_eq!(
            contribution(&tangent, &domain, &lower, &point, 192, 64).unwrap(),
            -1
        );
        assert_eq!(
            contribution(&tangent, &domain, &upper, &point, 192, 64).unwrap(),
            1
        );
    }
}
