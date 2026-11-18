//! Shore derivation: preserves the original arithmetic and proof budgets.
use super::*;

const MAX_ALLOWANCE: usize = 1000000;
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
/// The four frame corners are used twice each before any source point.
const CORNER_ENDPOINT_USES: usize = 8;
const FRAME_SHORES: usize = 4;
/// Source copies on each fixed axis's sides, grouped by source vertex.
type SideGroups = [BTreeMap<usize, Vec<usize>>; 2];
fn validate(o: &OwnedPeriodicNativeChartShores) -> Result<()> {
    if [
        o.limits.max_endpoint_uses,
        o.limits.max_intervals,
        o.limits.max_comparisons,
    ]
    .iter()
    .any(|n| !(1..=MAX_ALLOWANCE).contains(n))
        || o.limits.max_endpoint_uses < CORNER_ENDPOINT_USES
        || o.limits.max_intervals < FRAME_SHORES
        || o.parameter_limits.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&o.parameter_limits.max_resultant_size)
    {
        return Err(refused("Invalid native shore construction allowance"));
    }
    Ok(())
}
/// Corner source uses are retained through their proved native corner links.
/// Noncorner points on a wrapped side are shared with the opposite side.
fn side_groups(copies: &[OriginalPeriodicSourceChartCopy]) -> SideGroups {
    let mut groups: SideGroups = std::array::from_fn(|_| BTreeMap::new());
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
    groups
}
/// Corner uses plus one use of every side group, doubled on a wrapped axis.
fn endpoint_uses(groups: &SideGroups, wrapped: [bool; 2], max: usize) -> Result<usize> {
    let mut uses = CORNER_ENDPOINT_USES;
    for axis in 0..2 {
        let n = groups[axis]
            .len()
            .checked_mul(if wrapped[axis] { 2 } else { 1 })
            .ok_or_else(|| refused("Native shore endpoint census overflow"))?;
        uses = uses
            .checked_add(n)
            .filter(|n| *n <= max)
            .ok_or_else(|| refused("Native shore endpoint census exceeds allowance"))?;
    }
    Ok(uses)
}
/// The two actual corners of one shore, at the lower and upper end of its
/// varying coordinate.
fn shore_corners(
    owner: &OwnedPeriodicNativeChartCorners,
    axis: usize,
    end: usize,
) -> Result<[OriginalPeriodicNativeShorePoint; 2]> {
    let on_shore = owner
        .corners()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.ends[axis] == end)
        .collect::<Vec<_>>();
    let at = |varying_end: usize| {
        on_shore
            .iter()
            .find(|(_, c)| c.ends[1 - axis] == varying_end)
            .map(|(i, _)| OriginalPeriodicNativeShorePoint::Corner(*i))
    };
    match (on_shore.len(), at(0), at(1)) {
        (2, Some(lower), Some(upper)) => Ok([lower, upper]),
        _ => Err(refused("Native shore lacks its two actual corners")),
    }
}
/// The point one source vertex group contributes to the shore at `end` of
/// `axis`, after proving every copy of the group has the same varying
/// coordinate. `None` when an unwrapped group lies on the opposite side.
fn group_point(
    o: &OwnedPeriodicNativeChartShores,
    group: &[usize],
    [axis, end]: [usize; 2],
    wrapped: bool,
    cmp: &mut Compare,
) -> Result<Option<OriginalPeriodicNativeShorePoint>> {
    let copies = o.owner.owner().copies();
    let varying = 1 - axis;
    let actual = copies[group[0]].bounds[axis]
        .ok_or_else(|| refused("Native shore source group is not on an actual side"))?;
    if !wrapped
        && group
            .iter()
            .any(|&i| copies[i].bounds[axis] != Some(actual))
    {
        return Err(refused(
            "Original shore source identity changes an unwrapped side",
        ));
    }
    if !wrapped && actual != end {
        return Ok(None);
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
    Ok(Some(p))
}
/// One frame shore: its two corners and every source point on it, strictly
/// ordered by the native varying coordinate.
fn shore(
    o: &OwnedPeriodicNativeChartShores,
    groups: &BTreeMap<usize, Vec<usize>>,
    [axis, end]: [usize; 2],
    wrapped: bool,
    cmp: &mut Compare,
) -> Result<OriginalPeriodicNativeShore> {
    let varying = 1 - axis;
    let [lower, upper] = shore_corners(&o.owner, axis, end)?;
    let mut points = vec![lower, upper];
    for group in groups.values() {
        if let Some(p) = group_point(o, group, [axis, end], wrapped, cmp)? {
            points.push(p);
        }
    }
    let values = points
        .iter()
        .map(|&p| scalar(&o.owner, p, varying))
        .collect::<Result<Vec<_>>>()?;
    let ordered = order(&values, cmp)?;
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
    Ok(OriginalPeriodicNativeShore {
        fixed_axis: axis,
        end,
        role: if wrapped {
            OriginalPeriodicNativeShoreRole::AuxiliarySeam
        } else {
            OriginalPeriodicNativeShoreRole::NaturalSupport
        },
        direction: if (axis, end) == (0, 0) || (axis, end) == (1, 1) {
            -1
        } else {
            1
        },
        intervals: (0..points.len() - 1).map(|i| [i, i + 1]).collect(),
        points,
    })
}
pub(super) fn derive(o: &OwnedPeriodicNativeChartShores) -> Result<Data> {
    let _replay = super::super::periodic_replay::scope();
    validate(o)?;
    super::super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
    let cover = support(&o.owner).augmented().surface().pieces();
    let wrapped = cover.wrapped();
    let groups = side_groups(o.owner.owner().copies());
    let endpoint_uses = endpoint_uses(&groups, wrapped, o.limits.max_endpoint_uses)?;
    let mut cmp = Compare {
        limits: o.parameter_limits,
        cap: o.limits.max_comparisons,
        work: 0,
    };
    let mut shores = Vec::with_capacity(FRAME_SHORES);
    let mut intervals = 0usize;
    for axis in 0..2 {
        for end in 0..2 {
            let shore = shore(o, &groups[axis], [axis, end], wrapped[axis], &mut cmp)?;
            intervals = intervals
                .checked_add(shore.intervals.len())
                .filter(|n| *n <= o.limits.max_intervals)
                .ok_or_else(|| refused("Native shore interval census exceeds allowance"))?;
            shores.push(shore);
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
