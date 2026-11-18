//! Exact complete original-cell census on a transverse normal segment.
use super::*;
use event_coordinates::{sign_at, zero_at};
pub(in crate::profile::profile_certificate) mod radius;
/// A transverse normal segment through the selected original cell of `nets`:
/// centred at parameter `t`, of half-length `radius`, compared with up to
/// `refinements` interval refinements.
#[derive(Clone, Copy)]
pub(in crate::profile::profile_certificate) struct TransverseSegment<'a> {
    pub nets: &'a [Vec<[Q; 3]>],
    pub selected: usize,
    pub t: &'a Q,
    pub radius: &'a Q,
    pub refinements: usize,
}
pub(in crate::profile::profile_certificate) fn probes(
    segment: TransverseSegment<'_>,
) -> Result<[[Q; 2]; 2]> {
    let own = Ownership {
        centers: None,
        intervals: None,
        groups: None,
        matrix: 64,
    };
    probes_impl(segment, own)
}
#[derive(Clone)]
pub(in crate::profile::profile_certificate) struct CenterInterval {
    pub cell: usize,
    /// Actual local original-cell parameter bounds, in independent extensions.
    pub bounds: [branch_neighbourhood::ParameterValue; 2],
}
pub(in crate::profile::profile_certificate) fn probes_with_centers(
    segment: TransverseSegment<'_>,
    centers: &[CenterInterval],
    matrix: usize,
) -> Result<[[Q; 2]; 2]> {
    let nets = segment.nets;
    if centers.is_empty()
        || centers.len() > 4096
        || !(1..=128).contains(&matrix)
        || centers.iter().any(|c| c.cell >= nets.len())
    {
        return Err(refused("Invalid original common center interval census"));
    }
    let own = Ownership {
        centers: Some(centers),
        intervals: None,
        groups: None,
        matrix,
    };
    probes_impl(segment, own)
}
/// Complete transverse census restricted to every actual selected original cell
/// interval. Native-cell continuations outside these bounds are not geometry.
#[cfg(test)]
pub(in crate::profile::profile_certificate) fn probes_with_owned_intervals(
    segment: TransverseSegment<'_>,
    centers: &[CenterInterval],
    matrix: usize,
    intervals: &[[branch_neighbourhood::ParameterValue; 2]],
) -> Result<[[Q; 2]; 2]> {
    owned_groups(
        segment,
        centers,
        matrix,
        intervals,
        None,
    )
}
/// Each source carrier has several translated chart candidates. Exactly ONE
/// actual center root must be owned across all candidates of EACH group.
pub(in crate::profile::profile_certificate) fn probes_with_owned_center_groups(
    segment: TransverseSegment<'_>,
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
        segment,
        centers,
        matrix,
        intervals,
        Some(groups),
    )
}
fn owned_groups(
    segment: TransverseSegment<'_>,
    centers: &[CenterInterval],
    matrix: usize,
    intervals: &[[branch_neighbourhood::ParameterValue; 2]],
    groups: Option<&[usize]>,
) -> Result<[[Q; 2]; 2]> {
    let TransverseSegment { nets, refinements, .. } = segment;
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
            parameter_order::ParameterRef::new(&a.primitive, &a.value, &a.interval),
            parameter_order::ParameterRef::new(&b.primitive, &b.value, &b.interval),
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
    let own = Ownership {
        centers: Some(centers),
        intervals: Some(intervals),
        groups,
        matrix,
    };
    probes_impl(segment, own)
}
const MAX_REFINEMENTS: usize = 512;
/// Which crossings of the normal segment the census may treat as its own.
#[derive(Clone, Copy)]
struct Ownership<'a> {
    /// Candidate center intervals; without them only the selected cell at `t` is the center.
    centers: Option<&'a [CenterInterval]>,
    /// Actual owned parameter interval of every cell; roots outside are not geometry.
    intervals: Option<&'a [[branch_neighbourhood::ParameterValue; 2]]>,
    /// Source carrier of every center candidate; exactly one root per carrier.
    groups: Option<&'a [usize]>,
    matrix: usize,
}
/// The regular center point and its normal line, with the segment half-length
/// scaled by the squared normal length.
struct NormalLine {
    point: [Q; 2],
    tangent: [Q; 2],
    normal: [Q; 2],
    bound: Q,
}
fn normal_line(segment: TransverseSegment<'_>) -> Result<NormalLine> {
    let TransverseSegment {
        nets,
        selected,
        t,
        radius,
        ..
    } = segment;
    let net = &nets[selected];
    let powers = [power(net, 0), power(net, 1), power(net, 2)];
    let h = powers.clone().map(|p| at(&p, t));
    if h[2] <= Q::zero() {
        return Err(refused("Original carrier denominator is not positive"));
    }
    let point = [&h[0] / &h[2], &h[1] / &h[2]];
    let tangent: [Q; 2] = std::array::from_fn(|axis| {
        at(&derivative(&powers[axis]), t) * &h[2] - &h[axis] * at(&derivative(&powers[2]), t)
    });
    let normal = [-tangent[1].clone(), tangent[0].clone()];
    let length = &normal[0] * &normal[0] + &normal[1] * &normal[1];
    if length.is_zero() {
        return Err(refused(
            "Carrier transverse witness needs a regular original point",
        ));
    }
    Ok(NormalLine {
        bound: radius * &length,
        point,
        tangent,
        normal,
    })
}
/// One cell against the normal line: `f` vanishes where the cell meets the
/// line, `lateral` is the signed offset along it, and `upper`/`lower` compare
/// that offset with the two segment ends.
struct CellLine {
    f: Poly,
    lateral: Poly,
    upper: Poly,
    lower: Poly,
}
/// `None` when the whole cell lies on the normal line but provably outside
/// the closed segment.
fn cell_line(net: &[[Q; 3]], line: &NormalLine) -> Result<Option<CellLine>> {
    if net.iter().any(|h| h[2] <= Q::zero()) {
        return Err(refused("Original transverse cell has nonpositive weights"));
    }
    let NormalLine {
        point,
        tangent,
        normal,
        bound,
    } = line;
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
    let f = linear(&tangent[0], &tangent[1]);
    if f.is_empty() {
        // Coincident normal-line cells may be discarded only by a complete
        // positive-weight Bernstein bound outside the closed segment.
        let offset = |h: &[Q; 3]| {
            (&h[0] - &point[0] * &h[2]) * &normal[0] + (&h[1] - &point[1] * &h[2]) * &normal[1]
        };
        let all_above = net.iter().all(|h| offset(h) > bound * &h[2]);
        let all_below = net.iter().all(|h| offset(h) < -bound * &h[2]);
        if !all_above && !all_below {
            return Err(refused(
                "Coincident normal-line cell is not excluded from the transverse segment",
            ));
        }
        return Ok(None);
    }
    let lateral = linear(&normal[0], &normal[1]);
    let upper = sub(&lateral, &w.iter().map(|w| w * bound).collect::<Poly>());
    let lower = sub(&lateral, &w.iter().map(|w| -w * bound).collect::<Poly>());
    Ok(Some(CellLine {
        f,
        lateral,
        upper,
        lower,
    }))
}
/// Running census of the crossings of one normal segment.
struct Census<'a> {
    segment: TransverseSegment<'a>,
    own: Ownership<'a>,
    center_count: usize,
    /// Center crossings owned by every candidate (or by every group).
    alias_counts: Vec<usize>,
}
impl Census<'_> {
    /// Order of the root of `f` in `root` against an algebraic bound.
    fn compare_root(
        &self,
        f: &Poly,
        root: &[Q; 2],
        bound: &branch_neighbourhood::ParameterValue,
    ) -> Result<std::cmp::Ordering> {
        parameter_order::compare(
            parameter_order::ParameterRef::new(f, &vec![Q::zero(), Q::one()], root),
            parameter_order::ParameterRef::new(&bound.primitive, &bound.value, &bound.interval),
            self.segment.refinements,
            self.own.matrix,
        )
    }
    /// The single center candidate (or group) whose open interval holds this root.
    fn center_owner(
        &self,
        centers: &[CenterInterval],
        cell: usize,
        f: &Poly,
        root: &[Q; 2],
    ) -> Result<usize> {
        let mut matched = Vec::new();
        for (alias, c) in centers.iter().enumerate().filter(|(_, c)| c.cell == cell) {
            if self.compare_root(f, root, &c.bounds[0])? == std::cmp::Ordering::Greater
                && self.compare_root(f, root, &c.bounds[1])? == std::cmp::Ordering::Less
            {
                let group = self.own.groups.map_or(alias, |g| g[alias]);
                if !matched.contains(&group) {
                    matched.push(group);
                }
            }
        }
        match matched[..] {
            [owner] => Ok(owner),
            _ => Err(refused(
                "Original center crossing lacks one unique common carrier owner",
            )),
        }
    }
    /// Counts one crossing of the normal line as a center, skips it when it
    /// is outside the owned interval, or refuses it when it meets the segment.
    fn classify(&mut self, cell: usize, line: &CellLine, root: &[Q; 2]) -> Result<()> {
        let TransverseSegment {
            selected,
            t,
            refinements,
            ..
        } = self.segment;
        let f = &line.f;
        if let Some(intervals) = self.own.intervals
            && (self.compare_root(f, root, &intervals[cell][0])? == std::cmp::Ordering::Less
                || self.compare_root(f, root, &intervals[cell][1])? == std::cmp::Ordering::Greater)
        {
            return Ok(());
        }
        match self.own.centers {
            Some(centers) => {
                if zero_at(&line.lateral, f, root)? {
                    let owner = self.center_owner(centers, cell, f, root)?;
                    self.alias_counts[owner] += 1;
                    return Ok(());
                }
            }
            None => {
                if cell == selected && zero_at(&vec![-t.clone(), Q::one()], f, root)? {
                    self.center_count += 1;
                    return Ok(());
                }
            }
        }
        if sign_at(&line.upper, f, root, refinements)? <= 0
            && sign_at(&line.lower, f, root, refinements)? >= 0
        {
            return Err(refused(
                "Another original branch meets the closed transverse segment",
            ));
        }
        Ok(())
    }
    /// Exactly one center crossing overall, or one per candidate or group.
    fn owns_one_center(&self) -> bool {
        if self.own.centers.is_some() {
            self.alias_counts.iter().all(|&n| n == 1)
        } else {
            self.center_count == 1
        }
    }
}
fn probes_impl(segment: TransverseSegment<'_>, own: Ownership<'_>) -> Result<[[Q; 2]; 2]> {
    let TransverseSegment {
        nets,
        selected,
        t,
        radius,
        refinements,
    } = segment;
    if selected >= nets.len()
        || t <= &Q::zero()
        || t >= &Q::one()
        || radius <= &Q::zero()
        || refinements > MAX_REFINEMENTS
    {
        return Err(refused("Invalid original carrier transverse query"));
    }
    let line = normal_line(segment)?;
    let aliases = match (own.groups, own.centers) {
        (Some(groups), _) => groups.iter().max().map_or(0, |max| max + 1),
        (None, Some(centers)) => centers.len(),
        (None, None) => 0,
    };
    let mut census = Census {
        segment,
        own,
        center_count: 0,
        alias_counts: vec![0usize; aliases],
    };
    let width = Q::new(1.into(), 1024.into());
    for (cell, net) in nets.iter().enumerate() {
        let Some(cell_line) = cell_line(net, &line)? else {
            continue;
        };
        for root in algebraic_events::isolates(&cell_line.f, Q::zero(), Q::one(), &width)? {
            census.classify(cell, &cell_line, &root)?;
        }
    }
    if !census.owns_one_center() {
        return Err(refused(
            "Transverse census does not own exactly one original center crossing",
        ));
    }
    let NormalLine { point, normal, .. } = line;
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
pub(in crate::profile::profile_certificate) fn interior(
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
            probes_with_owned_intervals(
                TransverseSegment {
                    nets: &nets,
                    selected: 0,
                    t: &half,
                    radius: &radius,
                    refinements: 192,
                },
                &centers,
                64,
                &intervals,
            )
                .is_err()
        );
        assert_eq!(
            probes_with_owned_center_groups(
                TransverseSegment {
                    nets: &nets,
                    selected: 0,
                    t: &half,
                    radius: &radius,
                    refinements: 192,
                },
                &centers,
                64,
                &intervals,
                &[0, 0, 0],
            )
            .unwrap(),
            [
                [half.clone(), radius.clone()],
                [half.clone(), -radius.clone()]
            ]
        );
        assert!(
            probes_with_owned_center_groups(
                TransverseSegment {
                    nets: &nets,
                    selected: 0,
                    t: &half,
                    radius: &radius,
                    refinements: 192,
                },
                &centers,
                64,
                &intervals,
                &[0, 2, 2],
            )
            .is_err()
        );
        assert!(
            probes_with_owned_center_groups(
                TransverseSegment {
                    nets: &nets,
                    selected: 0,
                    t: &half,
                    radius: &radius,
                    refinements: 192,
                },
                &centers,
                64,
                &intervals,
                &[usize::MAX; 3],
            )
            .is_err()
        );
        let duplicate = [nets[0].clone(), nets[0].clone()];
        let bounds = [intervals[0].clone(), intervals[0].clone()];
        assert!(
            probes_with_owned_center_groups(
                TransverseSegment {
                    nets: &duplicate,
                    selected: 0,
                    t: &half,
                    radius: &radius,
                    refinements: 192,
                },
                &centers[..2],
                64,
                &bounds,
                &[0, 0],
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
        assert!(probes_with_centers(
            TransverseSegment {
                nets: &nets,
                selected: 0,
                t: &t,
                radius: &radius,
                refinements: 192,
            },
            &centers,
            64,
        ).is_err());
        let root = branch_neighbourhood::ParameterValue {
            primitive: vec![-t.clone(), q(0), q(1)],
            value: vec![q(0), q(1)],
            interval: [Q::new(7.into(), 10.into()), Q::new(4.into(), 5.into())],
        };
        let intervals = [[value(q(0)), value(q(1))], [root, value(q(1))]];
        let p = probes_with_owned_intervals(
            TransverseSegment {
                nets: &nets,
                selected: 0,
                t: &t,
                radius: &radius,
                refinements: 192,
            },
            &centers,
            64,
            &intervals,
        )
            .unwrap();
        assert_eq!(p, [[t.clone(), radius.clone()], [t, -radius]]);
        let mut invalid = intervals.clone();
        invalid[1][1] = value(q(2));
        assert!(
            probes_with_owned_intervals(
                TransverseSegment {
                    nets: &nets,
                    selected: 0,
                    t: &Q::new(1.into(), 2.into()),
                    radius: &Q::one(),
                    refinements: 192,
                },
                &centers,
                64,
                &invalid,
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
        assert!(probes_with_centers(
            TransverseSegment {
                nets: &nets,
                selected: 0,
                t: &q(1, 2),
                radius: &q(1, 8),
                refinements: 192,
            },
            &centers,
            64,
        ).is_ok());
        assert!(probes_with_centers(
            TransverseSegment {
                nets: &nets,
                selected: 0,
                t: &q(1, 2),
                radius: &q(1, 1),
                refinements: 192,
            },
            &centers,
            64,
        ).is_err());
        assert!(probes_with_centers(
            TransverseSegment {
                nets: &nets,
                selected: 0,
                t: &q(1, 2),
                radius: &q(1, 8),
                refinements: 192,
            },
            &centers[..1],
            64,
        ).is_err());
        let mut wrong = centers.clone();
        wrong[1].bounds[1] = parameter(q(1, 4));
        assert!(probes_with_centers(
            TransverseSegment {
                nets: &nets,
                selected: 0,
                t: &q(1, 2),
                radius: &q(1, 8),
                refinements: 192,
            },
            &wrong,
            64,
        ).is_err());
    }
}
