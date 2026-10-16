//! Derivatives of the owned ORIGINAL arc-holonomy-corrected Bishop solution.
//! A raw open normal and a corrected normal solve DIFFERENT angular ODEs.
use super::*;
use crate::sweeps::progressive_miter::authored_frame_certificate as frame;

pub(in crate::sweeps::progressive_sweep) fn closed_frame_jets(
    transport: &OriginalRmfTransportReport,
    path: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<frame::Report> {
    closed_jets(
        transport,
        path,
        twist,
        traversal,
        frame::PathPhase::identity(traversal),
        max_cells,
        false,
    )
}
/// `phase` MUST be the SAME original path's normalized arc inverse owner.
pub(in crate::sweeps::progressive_sweep) fn closed_phase_jets(
    transport: &OriginalRmfTransportReport,
    path: &Curve,
    twist: &Curve,
    station: [f64; 2],
    phase: frame::PathPhase,
    max_cells: usize,
) -> Result<frame::Report> {
    closed_jets(transport, path, twist, station, phase, max_cells, true)
}
fn closed_jets(
    transport: &OriginalRmfTransportReport,
    path: &Curve,
    twist: &Curve,
    station: [f64; 2],
    phase: frame::PathPhase,
    max_cells: usize,
    original_arc: bool,
) -> Result<frame::Report> {
    let traversal = phase.traversal;
    check(
        max_cells <= 100000
            && traversal.iter().all(|x| x.is_finite())
            && 0. <= traversal[0]
            && traversal[0] <= traversal[1]
            && traversal[1] <= 1.,
        "Invalid original closed Bishop jet restriction",
    )?;
    let unresolved = |cells, reason| frame::Report {
        status: Status::Unresolved,
        cells,
        longitudinal: None,
        transverse: None,
        binormal: None,
        single_span: false,
        reason: Some(reason),
    };
    let (Some(cover), Some(normals), Some(binormals), Some(angle), Some(length)) = (
        &transport.cover,
        &transport.arc_corrected_normals,
        &transport.arc_corrected_binormals,
        transport.closing_angle,
        transport.original_arc_length,
    ) else {
        return Ok(unresolved(0, "bishop-complete-corrected-cover-required"));
    };
    if transport.status != Status::Certified
        || cover.is_empty()
        || cover.len() != normals.len()
        || cover.len() != binormals.len()
        || length[0] <= 0.
        || length[0] > length[1]
        || length.iter().any(|x| !x.is_finite())
    {
        return Ok(unresolved(0, "bishop-complete-corrected-cover-required"));
    }
    let mut cells = 0;
    let mut covered = traversal[0];
    let mut normal: Option<Vec3> = None;
    for (cell, n) in cover.iter().zip(normals) {
        if cell.traversal[1] < traversal[0] || cell.traversal[0] > traversal[1] {
            continue;
        }
        if cells == max_cells {
            return Ok(unresolved(cells, "bishop-corrected-cover-query-work-limit"));
        }
        cells += 1;
        if cell.traversal[0] > covered || cell.traversal[0] >= cell.traversal[1] {
            return Ok(unresolved(cells, "bishop-corrected-cover-query-gap"));
        }
        covered = covered.max(cell.traversal[1]);
        let n = decode(*n)?;
        if let Some(acc) = &mut normal {
            for k in 0..3 {
                acc[k] = I::new(acc[k].lo.min(n[k].lo), acc[k].hi.max(n[k].hi))?;
            }
        } else {
            normal = Some(n);
        }
    }
    let Some(normal) = normal.filter(|_| covered >= traversal[1]) else {
        return Ok(unresolved(cells, "bishop-corrected-cover-query-missing"));
    };
    // Original length and holonomy were charged ONCE when this transport was
    // built. Only selected cover cells and fresh path/twist restrictions spend
    // query work here. No sampled-chord length or fitted correction is used.
    let mut result = if original_arc {
        frame::certify_bishop_phase_jets(
            path,
            encode(normal),
            twist,
            station,
            phase,
            max_cells - cells,
            Some(frame::BishopCorrection::OriginalArc {
                angle: I::new(angle[0], angle[1])?,
            }),
        )?
    } else {
        frame::certify_arc_corrected_bishop_jets(
            path,
            encode(normal),
            twist,
            traversal,
            max_cells - cells,
            angle,
            length,
        )?
    };
    result.cells += cells;
    Ok(result)
}
#[cfg(test)]
mod tests;
