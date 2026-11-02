//! Shared original-parameter root relation proof; never world-distance welding.
use super::Limits;
use crate::{
    Result,
    surface::Surface,
    surface_contact::{Verdict, Witness},
    surface_offset::{self, PairDomain},
};
use std::collections::VecDeque;
pub(crate) fn overlap(a: PairDomain, b: PairDomain) -> Option<PairDomain> {
    let mut out = a;
    for s in 0..2 {
        for k in 0..2 {
            out[s][k] = [a[s][k][0].max(b[s][k][0]), a[s][k][1].min(b[s][k][1])];
            if out[s][k][0] > out[s][k][1] {
                return None;
            }
        }
    }
    Some(out)
}
pub(crate) enum Relation {
    Common(Witness),
    Disjoint,
    Unresolved,
}
pub(crate) struct Work {
    pub(crate) steps: usize,
    pub(crate) sections: usize,
}
pub(crate) fn common(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    initial: PairDomain,
    limits: Limits,
    work: &mut Work,
) -> Result<Relation> {
    let mut queue = VecDeque::from([initial]);
    let mut unresolved = false;
    let widths = initial.map(|d| d.map(|r| r[1] - r[0]));
    while let Some(cell) = queue.pop_front() {
        if work.steps == limits.connection_steps {
            return Ok(Relation::Unresolved);
        }
        work.steps += 1;
        let a = surface_offset::bounds(surfaces[0], cell[0], distances[0], limits.spans_per_query)?;
        let b = surface_offset::bounds(surfaces[1], cell[1], distances[1], limits.spans_per_query)?;
        if let (Some(a), Some(b)) = (a.image, b.image) {
            if crate::surface_distance::enclosure_distance(&a, &b)?.0 > 0. {
                continue;
            }
        }
        for axis in 0..2 {
            if cell[0][1 - axis][0] >= cell[0][1 - axis][1] || cell[1].iter().any(|r| r[0] >= r[1])
            {
                continue;
            }
            let r = cell[0][axis];
            let samples = [r[0], r[0] * 0.5 + r[1] * 0.5, r[1]];
            for (i, &fixed) in samples.iter().enumerate() {
                if samples[..i].contains(&fixed) {
                    continue;
                }
                if work.sections == limits.section_queries {
                    break;
                }
                work.sections += 1;
                if let Verdict::Witness(w) = surface_offset::certify_contact_section(
                    surfaces,
                    distances,
                    axis,
                    fixed,
                    cell[0][1 - axis],
                    cell[1],
                    limits.spans_per_query,
                )? {
                    return Ok(Relation::Common(w));
                }
            }
        }
        // Section exclusion is NOT whole-box exclusion. Only the complete
        // bounds above may remove a box; otherwise split or retain ambiguity.
        let (side, axis) = (0..2)
            .flat_map(|s| (0..2).map(move |k| (s, k)))
            .max_by(|&(s, k), &(t, j)| {
                let width = |s: usize, k: usize| {
                    if widths[s][k] == 0. {
                        0.
                    } else {
                        (cell[s][k][1] - cell[s][k][0]) / widths[s][k]
                    }
                };
                width(s, k).total_cmp(&width(t, j))
            })
            .unwrap();
        let [lo, hi] = cell[side][axis];
        let mid = lo * 0.5 + hi * 0.5;
        if mid <= lo || mid >= hi {
            unresolved = true;
            continue;
        }
        for interval in [[lo, mid], [mid, hi]] {
            let mut next = cell;
            next[side][axis] = interval;
            queue.push_back(next);
        }
    }
    Ok(if unresolved {
        Relation::Unresolved
    } else {
        Relation::Disjoint
    })
}
