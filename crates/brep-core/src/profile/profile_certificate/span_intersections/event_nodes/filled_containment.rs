//! Shared material regions and holes from exact boundary containment.
use super::*;
/// Refusal messages of one caller, so each owner keeps its own wording.
pub(super) struct Messages {
    pub jumps: &'static str,
    pub classify: &'static str,
    pub hole: &'static str,
    pub simple: &'static str,
}
/// Winding vectors left and right of every boundary, and the regions.
pub(super) type Containment = (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<OriginalFilledRegion>);
/// Crossing the boundary changes only its own winding, by exactly one.
fn verify_jumps(boundary: usize, n: usize, l: &[i64], r: &[i64], m: &Messages) -> Result<()> {
    let consistent = l.len() == n
        && r.len() == n
        && l.iter().zip(r).enumerate().all(|(other, (a, b))| {
            if other == boundary {
                a.checked_sub(*b) == Some(1)
            } else {
                a == b
            }
        });
    if !consistent {
        return Err(refused(m.jumps));
    }
    Ok(())
}
/// Every hole joins the unique innermost outer boundary that contains it.
fn assign_holes(
    outer: &[usize],
    holes: Vec<usize>,
    left: &[Vec<i64>],
    m: &Messages,
) -> Result<Vec<OriginalFilledRegion>> {
    let mut regions = outer
        .iter()
        .map(|&outer| OriginalFilledRegion {
            outer,
            holes: Vec::new(),
        })
        .collect::<Vec<_>>();
    for hole in holes {
        let candidates = outer
            .iter()
            .copied()
            .filter(|&o| left[hole][o] == 1)
            .collect::<Vec<_>>();
        let innermost = candidates
            .iter()
            .copied()
            .filter(|&c| {
                candidates
                    .iter()
                    .all(|&other| c == other || left[c][other] == 1)
            })
            .collect::<Vec<_>>();
        let [owner] = innermost[..] else {
            return Err(refused(m.hole));
        };
        regions
            .iter_mut()
            .find(|r| r.outer == owner)
            .ok_or_else(|| refused(m.hole))?
            .holes
            .push(hole);
    }
    Ok(regions)
}
/// Classifies `n` boundaries from their side windings, which `sides`
/// computes one boundary at a time, and assembles the regions.
pub(super) fn containment(
    n: usize,
    mut sides: impl FnMut(usize) -> Result<(Vec<i64>, Vec<i64>)>,
    m: &Messages,
) -> Result<Containment> {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut outer = Vec::new();
    let mut holes = Vec::new();
    for boundary in 0..n {
        let (l, r) = sides(boundary)?;
        verify_jumps(boundary, n, &l, &r, m)?;
        match (l[boundary], r[boundary]) {
            (1, 0) => outer.push(boundary),
            (0, -1) => holes.push(boundary),
            _ => return Err(refused(m.classify)),
        }
        left.push(l);
        right.push(r);
    }
    let regions = assign_holes(&outer, holes, &left, m)?;
    if (0..n).any(|i| (0..n).any(|j| i != j && !matches!(left[i][j], -1..=1))) {
        return Err(refused(m.simple));
    }
    Ok((left, right, regions))
}
