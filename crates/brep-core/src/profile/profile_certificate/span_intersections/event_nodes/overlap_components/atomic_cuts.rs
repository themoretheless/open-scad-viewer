//! Exact ordering of source cut proposals and the atomic intervals between
//! adjacent cuts. Shared by the chart interval, periodic piece and unified
//! source carrier derivations, which differ only in their cut origins.
use super::source_carriers::compare;
use super::*;
use std::cmp::Ordering;
/// Allowance on the running work total after ordering every source's cuts.
const CUT_ORDERING_WORK: usize = 131072;
/// Allowance on the running work total after matching intervals to cells.
const CELL_COVERAGE_WORK: usize = 262144;
/// Knot boundaries of one source: the start of each cell, then the end of the last.
pub(super) fn knots(cs: &[Cell]) -> impl Iterator<Item = &Q> {
    cs.iter()
        .map(|c| &c.domain[0])
        .chain(cs.last().map(|c| &c.domain[1]))
}
/// Sorts cut proposals by exact parameter order and merges the origins of
/// equal parameters. Every comparison is charged to `work`.
pub(super) fn merge_ordered<O>(
    input: Vec<(OriginalSourceParameter, O)>,
    limits: ParameterOrderLimits,
    work: &mut usize,
    exhausted: &'static str,
) -> Result<Vec<(OriginalSourceParameter, Vec<O>)>> {
    let mut ordered: Vec<(OriginalSourceParameter, Vec<O>)> = Vec::new();
    for (value, origin) in input {
        let mut pos = 0;
        let mut joined = false;
        while pos < ordered.len() {
            *work += 1;
            if *work > CUT_ORDERING_WORK {
                return Err(refused(exhausted));
            }
            match compare(&ordered[pos].0, &value, limits)? {
                Ordering::Equal => {
                    joined = true;
                    break;
                }
                Ordering::Greater => break,
                Ordering::Less => pos += 1,
            }
        }
        if joined {
            ordered[pos].1.push(origin);
        } else {
            ordered.insert(pos, (value, vec![origin]));
        }
    }
    Ok(ordered)
}
/// Whether the first and last cuts are exactly the two ends of the source domain.
pub(super) fn spans_domain(
    [first, last]: [&OriginalSourceParameter; 2],
    cs: &[Cell],
    limits: ParameterOrderLimits,
) -> Result<bool> {
    Ok(
        compare(first, &chart_intervals::rational(&cs[0].domain[0]), limits)? == Ordering::Equal
            && compare(
                last,
                &chart_intervals::rational(&cs.last().unwrap().domain[1]),
                limits,
            )? == Ordering::Equal,
    )
}
/// The single source cell that contains the interval between two adjacent
/// cuts. `messages` are the refusals for an exhausted allowance and for an
/// interval that does not lie in exactly one cell.
pub(super) fn owning_cell(
    cs: &[Cell],
    [lower, upper]: [&OriginalSourceParameter; 2],
    limits: ParameterOrderLimits,
    work: &mut usize,
    [exhausted, not_one]: [&'static str; 2],
) -> Result<usize> {
    let mut found = Vec::new();
    for (cell, c) in cs.iter().enumerate() {
        *work += 1;
        if *work > CELL_COVERAGE_WORK {
            return Err(refused(exhausted));
        }
        if compare(lower, &chart_intervals::rational(&c.domain[0]), limits)? != Ordering::Less
            && compare(upper, &chart_intervals::rational(&c.domain[1]), limits)?
                != Ordering::Greater
        {
            found.push(cell);
        }
    }
    match found[..] {
        [cell] => Ok(cell),
        _ => Err(refused(not_one)),
    }
}
