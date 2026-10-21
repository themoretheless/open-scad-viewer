//! Exact complete original-cell census on a transverse normal segment.
use super::*;
use event_coordinates::{sign_at, zero_at};
pub(in crate::profile_certificate) fn probes(
    nets: &[Vec<[Q; 3]>],
    selected: usize,
    t: &Q,
    radius: &Q,
    refinements: usize,
) -> Result<[[Q; 2]; 2]> {
    probes_impl(nets, selected, t, radius, refinements, None, 64, None, None)
}
#[derive(Clone)]
pub(in crate::profile_certificate) struct CenterInterval {
    pub cell: usize,
    /// Actual local original-cell parameter bounds, in independent extensions.
    pub bounds: [branch_neighbourhood::ParameterValue; 2],
}
pub(in crate::profile_certificate) fn probes_with_centers(
    nets: &[Vec<[Q; 3]>],
    selected: usize,
    t: &Q,
    radius: &Q,
    refinements: usize,
    centers: &[CenterInterval],
    matrix: usize,
) -> Result<[[Q; 2]; 2]> {
    if centers.is_empty()
        || centers.len() > 4096
        || !(1..=128).contains(&matrix)
        || centers.iter().any(|c| c.cell >= nets.len())
    {
        return Err(refused("Invalid original common center interval census"));
    }
    probes_impl(
        nets,
        selected,
        t,
        radius,
        refinements,
        Some(centers),
        matrix,
        None,
        None,
    )
}
/// Complete transverse census restricted to every actual selected original cell
/// interval. Native-cell continuations outside these bounds are not geometry.
#[cfg(test)]
pub(in crate::profile_certificate) fn probes_with_owned_intervals(
    nets: &[Vec<[Q; 3]>],
    selected: usize,
    t: &Q,
    radius: &Q,
    refinements: usize,
    centers: &[CenterInterval],
    matrix: usize,
    intervals: &[[branch_neighbourhood::ParameterValue; 2]],
) -> Result<[[Q; 2]; 2]> {
    owned_groups(
        nets,
        selected,
        t,
        radius,
        refinements,
        centers,
        matrix,
        intervals,
        None,
    )
}
/// Each source carrier has several translated chart candidates. Exactly ONE
/// actual center root must be owned across all candidates of EACH group.
pub(in crate::profile_certificate) fn probes_with_owned_center_groups(
    nets: &[Vec<[Q; 3]>],
    selected: usize,
    t: &Q,
    radius: &Q,
    refinements: usize,
    centers: &[CenterInterval],
    matrix: usize,
    intervals: &[[branch_neighbourhood::ParameterValue; 2]],
    groups: &[usize],
) -> Result<[[Q; 2]; 2]> {
    if groups.len() != centers.len() || groups.is_empty() {
        return Err(refused("Invalid periodic center candidate groups"));
    }
    let maximum = *groups.iter().max().unwrap();
    if maximum >= 4096 {
        return Err(refused("Periodic center group exceeds allowance"));
    }
    let n = maximum + 1;
    if n > 4096 || (0..n).any(|g| !groups.contains(&g)) {
        return Err(refused(
            "Periodic center groups omit an original source carrier",
        ));
    }
    owned_groups(
        nets,
        selected,
        t,
        radius,
        refinements,
        centers,
        matrix,
        intervals,
        Some(groups),
    )
}
fn owned_groups(
    nets: &[Vec<[Q; 3]>],
    selected: usize,
    t: &Q,
    radius: &Q,
    refinements: usize,
    centers: &[CenterInterval],
    matrix: usize,
    intervals: &[[branch_neighbourhood::ParameterValue; 2]],
    groups: Option<&[usize]>,
) -> Result<[[Q; 2]; 2]> {
    if intervals.len() != nets.len()
        || centers.is_empty()
        || centers.len() > if groups.is_some() { 36864 } else { 4096 }
        || !(1..=128).contains(&matrix)
        || centers.iter().any(|c| c.cell >= nets.len())
    {
        return Err(refused(
            "Invalid selected transverse source interval census",
        ));
    }
    let compare = |a: &branch_neighbourhood::ParameterValue,
                   b: &branch_neighbourhood::ParameterValue| {
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
    let constant = |q: Q| branch_neighbourhood::ParameterValue {
        primitive: vec![Q::zero(), Q::one()],
        value: trim(vec![q]),
        interval: [Q::zero(), Q::zero()],
    };
    for bounds in intervals {
        if compare(&bounds[0], &bounds[1])? != std::cmp::Ordering::Less
            || compare(&bounds[0], &constant(Q::zero()))? == std::cmp::Ordering::Less
            || compare(&bounds[1], &constant(Q::one()))? == std::cmp::Ordering::Greater
        {
            return Err(refused(
                "Selected transverse interval is outside its original cell",
            ));
        }
    }
    probes_impl(
        nets,
        selected,
        t,
        radius,
        refinements,
        Some(centers),
        matrix,
        Some(intervals),
        groups,
    )
}
fn probes_impl(
    nets: &[Vec<[Q; 3]>],
    selected: usize,
    t: &Q,
    radius: &Q,
    refinements: usize,
    centers: Option<&[CenterInterval]>,
    matrix: usize,
    intervals: Option<&[[branch_neighbourhood::ParameterValue; 2]]>,
    groups: Option<&[usize]>,
) -> Result<[[Q; 2]; 2]> {
    if selected >= nets.len()
        || t <= &Q::zero()
        || t >= &Q::one()
        || radius <= &Q::zero()
        || refinements > 512
    {
        return Err(refused("Invalid original carrier transverse query"));
    }
    let net = &nets[selected];
    let powers = [power(net, 0), power(net, 1), power(net, 2)];
    let h = powers.clone().map(|p| at(&p, t));
    if h[2] <= Q::zero() {
        return Err(refused("Original carrier denominator is not positive"));
    }
    let point = [&h[0] / &h[2], &h[1] / &h[2]];
    let d: [Q; 2] = std::array::from_fn(|axis| {
        at(&derivative(&powers[axis]), t) * &h[2] - &h[axis] * at(&derivative(&powers[2]), t)
    });
    let normal = [-d[1].clone(), d[0].clone()];
    let length = &normal[0] * &normal[0] + &normal[1] * &normal[1];
    if length.is_zero() {
        return Err(refused(
            "Carrier transverse witness needs a regular original point",
        ));
    }
    let bound = radius * &length;
    let mut center_count = 0;
    let mut alias_counts = vec![
        0usize;
        groups.map_or_else(
            || centers.map_or(0, |c| c.len()),
            |g| g.iter().max().unwrap() + 1
        )
    ];
    for (cell, net) in nets.iter().enumerate() {
        if net.iter().any(|h| h[2] <= Q::zero()) {
            return Err(refused("Original transverse cell has nonpositive weights"));
        }
        let w = power(net, 2);
        let x = sub(
            &power(net, 0),
            &w.iter().map(|w| w * &point[0]).collect::<Poly>(),
        );
        let y = sub(
            &power(net, 1),
            &w.iter().map(|w| w * &point[1]).collect::<Poly>(),
        );
        let linear = |a: &Q, b: &Q| -> Poly {
            let mut p = vec![Q::zero(); x.len().max(y.len())];
            for (i, c) in x.iter().enumerate() {
                p[i] += c * a;
            }
            for (i, c) in y.iter().enumerate() {
                p[i] += c * b;
            }
            trim(p)
        };
        let f = linear(&d[0], &d[1]);
        if f.is_empty() {
            // Coincident normal-line cells may be discarded only by a complete
            // positive-weight Bernstein bound outside the closed segment.
            let all_above = net.iter().all(|h| {
                (&h[0] - &point[0] * &h[2]) * &normal[0] + (&h[1] - &point[1] * &h[2]) * &normal[1]
                    > &bound * &h[2]
            });
            let all_below = net.iter().all(|h| {
                (&h[0] - &point[0] * &h[2]) * &normal[0] + (&h[1] - &point[1] * &h[2]) * &normal[1]
                    < -&bound * &h[2]
            });
            if !all_above && !all_below {
                return Err(refused(
                    "Coincident normal-line cell is not excluded from the transverse segment",
                ));
            }
            continue;
        }
        let lateral = linear(&normal[0], &normal[1]);
        let upper = sub(&lateral, &w.iter().map(|w| w * &bound).collect::<Poly>());
        let lower = sub(&lateral, &w.iter().map(|w| -w * &bound).collect::<Poly>());
        let roots =
            algebraic_events::isolates(&f, Q::zero(), Q::one(), &Q::new(1.into(), 1024.into()))?;
        for root in roots {
            if let Some(intervals) = intervals {
                let compare = |b: &branch_neighbourhood::ParameterValue| {
                    parameter_order::compare(
                        &f,
                        &vec![Q::zero(), Q::one()],
                        &root,
                        &b.primitive,
                        &b.value,
                        &b.interval,
                        refinements,
                        matrix,
                    )
                };
                if compare(&intervals[cell][0])? == std::cmp::Ordering::Less
                    || compare(&intervals[cell][1])? == std::cmp::Ordering::Greater
                {
                    continue;
                }
            }

            if let Some(centers) = centers {
                if zero_at(&lateral, &f, &root)? {
                    let mut matched = Vec::new();
                    for (alias, c) in centers.iter().enumerate().filter(|(_, c)| c.cell == cell) {
                        let compare = |bound: &branch_neighbourhood::ParameterValue| {
                            parameter_order::compare(
                                &f,
                                &vec![Q::zero(), Q::one()],
                                &root,
                                &bound.primitive,
                                &bound.value,
                                &bound.interval,
                                refinements,
                                matrix,
                            )
                        };
                        if compare(&c.bounds[0])? == std::cmp::Ordering::Greater
                            && compare(&c.bounds[1])? == std::cmp::Ordering::Less
                        {
                            let group = groups.map_or(alias, |g| g[alias]);
                            if !matched.contains(&group) {
                                matched.push(group);
                            }
                        }
                    }
                    if matched.len() != 1 {
                        return Err(refused(
                            "Original center crossing lacks one unique common carrier owner",
                        ));
                    }
                    alias_counts[matched[0]] += 1;
                    continue;
                }
            }
            if centers.is_none()
                && cell == selected
                && zero_at(&vec![-t.clone(), Q::one()], &f, &root)?
            {
                center_count += 1;
                continue;
            }
            if sign_at(&upper, &f, &root, refinements)? <= 0
                && sign_at(&lower, &f, &root, refinements)? >= 0
            {
                return Err(refused(
                    "Another original branch meets the closed transverse segment",
                ));
            }
        }
    }
    if if centers.is_some() {
        alias_counts.iter().any(|&n| n != 1)
    } else {
        center_count != 1
    } {
        return Err(refused(
            "Transverse census does not own exactly one original center crossing",
        ));
    }
    Ok(std::array::from_fn(|side| {
        std::array::from_fn(|axis| {
            if side == 0 {
                &point[axis] + radius * &normal[axis]
            } else {
                &point[axis] - radius * &normal[axis]
            }
        })
    }))
}

/// Choose a rational regular interior point strictly between both actual cuts.
pub(in crate::profile_certificate) fn interior(
    cells: &[(Vec<[Q; 3]>, [Q; 2])],
    mut bounds: [branch_neighbourhood::ParameterValue; 2],
    refinements: usize,
) -> Result<Q> {
    if refinements > 512 {
        return Err(refused("Original carrier interior allowance exceeded"));
    }
    for step in 0..=refinements {
        let lower = parameter_order::range(&bounds[0].value, &bounds[0].interval)[1].clone();
        let upper = parameter_order::range(&bounds[1].value, &bounds[1].interval)[0].clone();
        for (net, domain) in cells {
            let lo = lower.clone().max(domain[0].clone());
            let hi = upper.clone().min(domain[1].clone());
            if lo >= hi {
                continue;
            }
            // More distinct trials than the maximum number of simultaneous
            // derivative zeros of this actual rational source knot cell.
            let trials = 2 * net.len() + 1;
            for i in 1..trials {
                let t = &lo + (&hi - &lo) * Q::new(i.into(), trials.into());
                let local = (&t - &domain[0]) / (&domain[1] - &domain[0]);
                let w = power(net, 2);
                let weight = at(&w, &local);
                let dw = at(&derivative(&w), &local);
                if (0..2).any(|axis| {
                    let x = power(net, axis);
                    !(at(&derivative(&x), &local) * &weight - at(&x, &local) * &dw).is_zero()
                }) {
                    return Ok(t);
                }
            }
        }
        if step < refinements {
            for bound in &mut bounds {
                parameter_order::refine(&bound.primitive, &mut bound.interval)?;
            }
        }
    }
    Err(refused(
        "Regular original carrier interior remains unproven within allowance",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_center_groups_own_one_root_across_all_views_and_refuse_duplicate_ownership() {
        let q = |n: i64| Q::from_integer(n.into());
        let b = |x: Q| branch_neighbourhood::ParameterValue {
            primitive: vec![Q::zero(), Q::one()],
            value: trim(vec![x]),
            interval: [Q::zero(), Q::zero()],
        };
        let nets = [0, 2, -2].map(|y| vec![[q(0), q(y), q(1)], [q(1), q(y), q(1)]]);
        let intervals = std::array::from_fn::<_, 3, _>(|_| [b(q(0)), b(q(1))]);
        let centers = (0..3)
            .map(|cell| CenterInterval {
                cell,
                bounds: intervals[cell].clone(),
            })
            .collect::<Vec<_>>();
        let half = Q::new(1.into(), 2.into());
        let radius = Q::new(1.into(), 8.into());
        assert!(
            probes_with_owned_intervals(&nets, 0, &half, &radius, 192, &centers, 64, &intervals)
                .is_err()
        );
        assert_eq!(
            probes_with_owned_center_groups(
                &nets,
                0,
                &half,
                &radius,
                192,
                &centers,
                64,
                &intervals,
                &[0, 0, 0]
            )
            .unwrap(),
            [
                [half.clone(), radius.clone()],
                [half.clone(), -radius.clone()]
            ]
        );
        assert!(
            probes_with_owned_center_groups(
                &nets,
                0,
                &half,
                &radius,
                192,
                &centers,
                64,
                &intervals,
                &[0, 2, 2]
            )
            .is_err()
        );
        assert!(
            probes_with_owned_center_groups(
                &nets,
                0,
                &half,
                &radius,
                192,
                &centers,
                64,
                &intervals,
                &[usize::MAX; 3]
            )
            .is_err()
        );
        let duplicate = [nets[0].clone(), nets[0].clone()];
        let bounds = [intervals[0].clone(), intervals[0].clone()];
        assert!(
            probes_with_owned_center_groups(
                &duplicate,
                0,
                &half,
                &radius,
                192,
                &centers[..2],
                64,
                &bounds,
                &[0, 0]
            )
            .is_err()
        );
    }
    #[test]
    fn selected_transverse_census_ignores_excluded_supporting_center_in_an_independent_field() {
        let q = |n: i64| Q::from_integer(n.into());
        let value = |x: Q| branch_neighbourhood::ParameterValue {
            primitive: vec![Q::zero(), Q::one()],
            value: trim(vec![x]),
            interval: [Q::zero(), Q::zero()],
        };
        let nets = vec![
            vec![[q(0), q(0), q(1)], [q(1), q(0), q(1)]],
            vec![[q(0), q(-1), q(1)], [q(1), q(1), q(1)]],
        ];
        let centers = [CenterInterval {
            cell: 0,
            bounds: [value(q(0)), value(q(1))],
        }];
        let t = Q::new(1.into(), 2.into());
        let radius = Q::new(1.into(), 16.into());
        assert!(probes_with_centers(&nets, 0, &t, &radius, 192, &centers, 64).is_err());
        let root = branch_neighbourhood::ParameterValue {
            primitive: vec![-t.clone(), q(0), q(1)],
            value: vec![q(0), q(1)],
            interval: [Q::new(7.into(), 10.into()), Q::new(4.into(), 5.into())],
        };
        let intervals = [[value(q(0)), value(q(1))], [root, value(q(1))]];
        let p = probes_with_owned_intervals(&nets, 0, &t, &radius, 192, &centers, 64, &intervals)
            .unwrap();
        assert_eq!(p, [[t.clone(), radius.clone()], [t, -radius]]);
        let mut invalid = intervals.clone();
        invalid[1][1] = value(q(2));
        assert!(
            probes_with_owned_intervals(
                &nets,
                0,
                &Q::new(1.into(), 2.into()),
                &Q::one(),
                192,
                &centers,
                64,
                &invalid
            )
            .is_err()
        );
    }
    #[test]
    fn exact_interior_refines_overlapping_selected_parameter_enclosures() {
        let f = vec![Q::from_integer((-2).into()), Q::zero(), Q::one()];
        let root = [Q::one(), Q::from_integer(2.into())];
        let gap = Q::new(1.into(), num_bigint::BigInt::from(1) << 44);
        let bounds = [
            branch_neighbourhood::ParameterValue {
                primitive: f.clone(),
                value: vec![Q::zero(), Q::one()],
                interval: root.clone(),
            },
            branch_neighbourhood::ParameterValue {
                primitive: f.clone(),
                value: vec![gap.clone(), Q::one()],
                interval: root.clone(),
            },
        ];
        let cells = vec![(
            vec![
                [Q::zero(), Q::zero(), Q::one()],
                [Q::one(), Q::zero(), Q::one()],
            ],
            [Q::one(), Q::from_integer(2.into())],
        )];
        assert!(interior(&cells, bounds.clone(), 0).is_err());
        let t = interior(&cells, bounds, 192).unwrap();
        assert_eq!(
            sign_at(&vec![-t.clone(), Q::one()], &f, &root, 192).unwrap(),
            -1
        );
        assert_eq!(
            sign_at(&vec![gap - t, Q::one()], &f, &root, 192).unwrap(),
            1
        );
    }
}
#[cfg(test)]
mod common_center_tests {
    use super::*;
    #[test]
    fn common_center_census_requires_every_alias_and_excludes_other_branches() {
        let q = |n: i64, d: i64| Q::new(n.into(), d.into());
        let net = vec![[q(0, 1), q(0, 1), q(1, 1)], [q(2, 1), q(0, 1), q(1, 1)]];
        let other = vec![[q(0, 1), q(1, 1), q(1, 1)], [q(2, 1), q(1, 1), q(1, 1)]];
        let parameter = |value: Q| branch_neighbourhood::ParameterValue {
            primitive: vec![Q::zero(), Q::one()],
            value: vec![value],
            interval: [Q::zero(), Q::zero()],
        };
        let centers = (0..2)
            .map(|cell| CenterInterval {
                cell,
                bounds: [parameter(Q::zero()), parameter(Q::one())],
            })
            .collect::<Vec<_>>();
        let nets = vec![net.clone(), net, other];
        assert!(probes_with_centers(&nets, 0, &q(1, 2), &q(1, 8), 192, &centers, 64).is_ok());
        assert!(probes_with_centers(&nets, 0, &q(1, 2), &q(1, 1), 192, &centers, 64).is_err());
        assert!(probes_with_centers(&nets, 0, &q(1, 2), &q(1, 8), 192, &centers[..1], 64).is_err());
        let mut wrong = centers.clone();
        wrong[1].bounds[1] = parameter(q(1, 4));
        assert!(probes_with_centers(&nets, 0, &q(1, 2), &q(1, 8), 192, &wrong, 64).is_err());
    }
}
