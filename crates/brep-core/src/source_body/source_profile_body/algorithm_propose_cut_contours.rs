//! source profile body algorithm: propose_cut_contours.
use super::*;
use crate::source_boundary_fragment::{Endpoint, Fragment, Role};

/// Extra fragments charged beyond the wire: two selected halves per arc.
const CONTOUR_COST_OVERHEAD: usize = 4;

pub fn propose_cut_contours(
    a: &CutPartition<'_, '_, '_>,
    b: &CutPartition<'_, '_, '_>,
    max_fragments: usize,
) -> Result<CutContourReport> {
    if max_fragments > span_work::MAX_WORK {
        return Err(Error::new(
            "BREP_PROFILE_CUT",
            "Bound cut contour fragments",
        ));
    }
    let mut out = CutContourReport {
        proposal: None,
        fragments: 0,
        reason: "profile-cut-contour-source-unproven",
    };
    if !opposite_ends_of_one_cut(a, b) {
        return Ok(out);
    }
    let regions = a
        .endpoint
        .proposal
        .body
        .geometry()
        .shell()
        .regions()
        .ok_or_else(|| Error::new("BREP_PROFILE_CUT", "Bound cut contour fragments"))?;
    let region = &regions[a.endpoint.face];
    if region.loops().len() != 1 {
        out.reason = "profile-cut-contour-holes-unclassified";
        return Ok(out);
    }
    let wire = &region.loops()[0];
    let (Some(ia), Some(ib)) = (locate(wire, a), locate(wire, b)) else {
        return Ok(out);
    };
    if ia == ib {
        return Ok(out);
    }
    let cost = wire.len() + CONTOUR_COST_OVERHEAD;
    if cost > max_fragments {
        out.reason = "profile-cut-contour-work-limit";
        return Ok(out);
    }
    let loops = [arc(wire, ia, ib, a, b)?, arc(wire, ib, ia, b, a)?];
    out.fragments = cost;
    if loops
        .iter()
        .any(|wire| (0..wire.len()).any(|i| !wire[i].joins(&wire[(i + 1) % wire.len()])))
    {
        out.reason = "profile-cut-contour-joins-unproven";
        return Ok(out);
    }
    out.proposal = Some(CutContourProposal { loops });
    out.reason = "profile-cut-contour-source-joins-qualified";
    Ok(out)
}

/// Both partitions cut the same section of one proposal at opposite ends.
fn opposite_ends_of_one_cut(a: &CutPartition<'_, '_, '_>, b: &CutPartition<'_, '_, '_>) -> bool {
    std::ptr::eq(a.endpoint.proposal, b.endpoint.proposal)
        && a.endpoint.face == b.endpoint.face
        && a.endpoint.axis == b.endpoint.axis
        && a.endpoint.parameter.to_bits() == b.endpoint.parameter.to_bits()
        && a.endpoint.high != b.endpoint.high
        && a.point.contact() == b.point.contact()
        && a.point.surface() == b.point.surface()
}

/// Index of the unique wire fragment split by `p`.
fn locate(wire: &[Fragment], p: &CutPartition<'_, '_, '_>) -> Option<usize> {
    let matches = wire
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.surface() == p.point.surface()
                && f.curve() == p.point.boundary()
                && f.endpoints()
                    .iter()
                    .zip(
                        p.selected[0]
                            .endpoints()
                            .iter()
                            .take(1)
                            .chain(p.selected[1].endpoints().iter().skip(1)),
                    )
                    .all(|(x, y)| {
                        // Original retained endpoints must agree as definitions.
                        match (x, y) {
                            (Endpoint::Parameter(x), Endpoint::Parameter(y)) => {
                                x.to_bits() == y.to_bits()
                            }
                            _ => false,
                        }
                    })
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        Some(matches[0])
    } else {
        None
    }
}

fn bridge(start: &CutPartition<'_, '_, '_>, end: &CutPartition<'_, '_, '_>) -> Result<Fragment> {
    Fragment::new(
        start.point.surface(),
        start.point.contact(),
        Endpoint::Crossing {
            point: start.point.clone(),
            role: Role::Contact,
        },
        Endpoint::Crossing {
            point: end.point.clone(),
            role: Role::Contact,
        },
    )
}

/// Closed contour from `head`'s retained half around the wire to `tail`,
/// closed by the cut bridge.
fn arc(
    wire: &[Fragment],
    start: usize,
    end: usize,
    head: &CutPartition<'_, '_, '_>,
    tail: &CutPartition<'_, '_, '_>,
) -> Result<Vec<Fragment>> {
    let mut fragments = vec![head.selected[1].clone()];
    let mut index = (start + 1) % wire.len();
    while index != end {
        fragments.push(wire[index].clone());
        index = (index + 1) % wire.len();
    }
    fragments.push(tail.selected[0].clone());
    fragments.push(bridge(tail, head)?);
    Ok(fragments)
}
