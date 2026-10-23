//! Shore derivation: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn derive(o: &OwnedPeriodicNativeChartShores) -> Result<Data> {
    let _replay = super::super::periodic_replay::scope();
    if [
        o.limits.max_endpoint_uses,
        o.limits.max_intervals,
        o.limits.max_comparisons,
    ]
    .iter()
    .any(|n| !(1..=1000000).contains(n))
        || o.limits.max_endpoint_uses < 8
        || o.limits.max_intervals < 4
        || o.parameter_limits.max_refinements > 512
        || !(1..=128).contains(&o.parameter_limits.max_resultant_size)
    {
        return Err(refused("Invalid native shore construction allowance"));
    }
    super::super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
    let cover = support(&o.owner).augmented().surface().pieces();
    let wrapped = cover.wrapped();
    let copies = o.owner.owner().copies();
    // Corner source uses are retained through their proved native corner links.
    // Noncorner points on a wrapped side are shared with the opposite side.
    let mut groups: [BTreeMap<usize, Vec<usize>>; 2] = std::array::from_fn(|_| BTreeMap::new());
    for (i, c) in copies.iter().enumerate() {
        if c.bounds.iter().all(Option::is_some) {
            continue;
        }
        for axis in 0..2 {
            if c.bounds[axis].is_some() {
                groups[axis].entry(c.source_vertex).or_default().push(i);
            }
        }
    }
    let mut endpoint_uses = 8usize;
    for axis in 0..2 {
        let n = groups[axis]
            .len()
            .checked_mul(if wrapped[axis] { 2 } else { 1 })
            .ok_or_else(|| refused("Native shore endpoint census overflow"))?;
        endpoint_uses = endpoint_uses
            .checked_add(n)
            .filter(|n| *n <= o.limits.max_endpoint_uses)
            .ok_or_else(|| refused("Native shore endpoint census exceeds allowance"))?;
    }
    if endpoint_uses > o.limits.max_endpoint_uses || o.limits.max_intervals < 4 {
        return Err(refused("Native shore frame census exceeds allowance"));
    }
    let mut cmp = Compare {
        limits: o.parameter_limits,
        cap: o.limits.max_comparisons,
        work: 0,
    };
    let mut shores = Vec::with_capacity(4);
    let mut intervals = 0usize;
    for axis in 0..2 {
        for end in 0..2 {
            let varying = 1 - axis;
            let mut points = o
                .owner
                .corners()
                .iter()
                .enumerate()
                .filter(|(_, c)| c.ends[axis] == end)
                .map(|(i, _)| OriginalPeriodicNativeShorePoint::Corner(i))
                .collect::<Vec<_>>();
            if points.len() != 2 {
                return Err(refused("Native shore lacks its two actual corners"));
            }
            let lower = points
                .iter()
                .copied()
                .find(|p| match *p {
                    OriginalPeriodicNativeShorePoint::Corner(i) => {
                        o.owner.corners()[i].ends[varying] == 0
                    }
                    _ => false,
                })
                .unwrap();
            let upper = points
                .iter()
                .copied()
                .find(|p| match *p {
                    OriginalPeriodicNativeShorePoint::Corner(i) => {
                        o.owner.corners()[i].ends[varying] == 1
                    }
                    _ => false,
                })
                .unwrap();
            for group in groups[axis].values() {
                let actual = copies[group[0]].bounds[axis]
                    .ok_or_else(|| refused("Native shore source group is not on an actual side"))?;
                if !wrapped[axis]
                    && group
                        .iter()
                        .any(|&i| copies[i].bounds[axis] != Some(actual))
                {
                    return Err(refused(
                        "Original shore source identity changes an unwrapped side",
                    ));
                }
                if !wrapped[axis] && actual != end {
                    continue;
                }
                let copy = group
                    .iter()
                    .copied()
                    .find(|&i| copies[i].bounds[axis] == Some(end))
                    .unwrap_or(group[0]);
                let p = OriginalPeriodicNativeShorePoint::SourceCopy {
                    copy,
                    transported: copies[copy].bounds[axis] != Some(end),
                };
                let s = scalar(&o.owner, p, varying)?;
                for &other in group {
                    let q = scalar(
                        &o.owner,
                        OriginalPeriodicNativeShorePoint::SourceCopy {
                            copy: other,
                            transported: false,
                        },
                        varying,
                    )?;
                    if cmp.compare(&s, &q)? != Ordering::Equal {
                        return Err(refused(
                            "Actual opposite-shore source vertex changes its native varying coordinate",
                        ));
                    }
                }
                points.push(p);
            }
            let values = points
                .iter()
                .map(|&p| scalar(&o.owner, p, varying))
                .collect::<Result<Vec<_>>>()?;
            let ordered = order(&values, &mut cmp)?;
            for p in ordered.windows(2) {
                if cmp.compare(&values[p[0]], &values[p[1]])? != Ordering::Less {
                    return Err(refused(
                        "Native shore has equal endpoints without a proved original corner/source identity",
                    ));
                }
            }
            let points = ordered.iter().map(|&i| points[i]).collect::<Vec<_>>();
            if points.first() != Some(&lower) || points.last() != Some(&upper) {
                return Err(refused(
                    "Native shore ordering escaped actual native corner bounds",
                ));
            }
            let count = points.len() - 1;
            intervals = intervals
                .checked_add(count)
                .filter(|n| *n <= o.limits.max_intervals)
                .ok_or_else(|| refused("Native shore interval census exceeds allowance"))?;
            shores.push(OriginalPeriodicNativeShore {
                fixed_axis: axis,
                end,
                role: if wrapped[axis] {
                    OriginalPeriodicNativeShoreRole::AuxiliarySeam
                } else {
                    OriginalPeriodicNativeShoreRole::NaturalSupport
                },
                direction: if (axis, end) == (0, 0) || (axis, end) == (1, 1) {
                    -1
                } else {
                    1
                },
                intervals: (0..count).map(|i| [i, i + 1]).collect(),
                points,
            });
        }
    }
    Ok(Data {
        shores,
        endpoint_uses,
        intervals,
        comparisons: cmp.work,
    })
}

pub(super) fn scalar(
    owner: &OwnedPeriodicNativeChartCorners,
    p: OriginalPeriodicNativeShorePoint,
    axis: usize,
) -> Result<Scalar> {
    match p {
        OriginalPeriodicNativeShorePoint::Corner(i) => {
            let c = owner
                .corners()
                .get(i)
                .ok_or_else(|| refused("Native shore lost actual corner reference"))?;
            Ok(Scalar {
                f: vec![Q::zero(), Q::one()],
                root: [Q::zero(), Q::zero()],
                n: vec![decode_ratio(&c.uv[axis])?],
                d: vec![Q::one()],
            })
        }
        OriginalPeriodicNativeShorePoint::SourceCopy { copy, .. } => {
            let c = owner
                .owner()
                .copies()
                .get(copy)
                .ok_or_else(|| refused("Native shore lost original source-copy reference"))?;
            Ok(Scalar {
                f: decode_polynomial(&c.parameter.primitive)?,
                root: [
                    decode_ratio(&c.parameter.interval[0])?,
                    decode_ratio(&c.parameter.interval[1])?,
                ],
                n: decode_polynomial(&c.uv[axis][0])?,
                d: decode_polynomial(&c.uv[axis][1])?,
            })
        }
    }
}

/// Fallible stable merge sort on indices: a failed exact root comparison must
/// never become an Equality branch inside an infallible sorting closure.
pub(super) fn order(values: &[Scalar], cmp: &mut Compare) -> Result<Vec<usize>> {
    let n = values.len();
    let mut indices = (0..n).collect::<Vec<_>>();
    let mut scratch = vec![0; n];
    let mut width = 1usize;
    while width < n {
        let step = width
            .checked_mul(2)
            .ok_or_else(|| refused("Native shore sort work overflow"))?;
        for begin in (0..n).step_by(step) {
            let mid = (begin + width).min(n);
            let end = (begin + step).min(n);
            let (mut a, mut b) = (begin, mid);
            for out in begin..end {
                if a < mid
                    && (b == end
                        || cmp.compare(&values[indices[a]], &values[indices[b]])?
                            != Ordering::Greater)
                {
                    scratch[out] = indices[a];
                    a += 1;
                } else {
                    scratch[out] = indices[b];
                    b += 1;
                }
            }
        }
        std::mem::swap(&mut indices, &mut scratch);
        width = step;
    }
    Ok(indices)
}
