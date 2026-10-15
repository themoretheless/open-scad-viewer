//! Jet restrictions of the original open Bishop normal cover. The transport
//! is constructed once by the caller from this same immutable original path.
use super::*;
use crate::sweeps::progressive_miter::authored_frame_certificate as frame;

pub(in crate::sweeps::progressive_sweep) fn frame_jets(
    transport: &OriginalRmfTransportReport,
    path: &Curve,
    twist: &Curve,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<frame::Report> {
    check(
        max_cells <= 100000
            && traversal.iter().all(|x| x.is_finite())
            && 0. <= traversal[0]
            && traversal[0] <= traversal[1]
            && traversal[1] <= 1.,
        "Invalid original Bishop jet restriction",
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
    if transport.status != Status::Certified
        || transport.closing_angle.is_some()
        || transport.arc_corrected_normals.is_some()
        || transport.arc_corrected_binormals.is_some()
    {
        return Ok(unresolved(0, "bishop-open-original-cover-required"));
    }
    let Some(cover) = &transport.cover else {
        return Ok(unresolved(0, "bishop-open-original-cover-required"));
    };
    let mut cells = 0;
    let mut normals: Option<Vec3> = None;
    for cell in cover {
        if cell.traversal[1] < traversal[0] || cell.traversal[0] > traversal[1] {
            continue;
        }
        if cells == max_cells {
            return Ok(unresolved(cells, "bishop-cover-query-work-limit"));
        }
        cells += 1;
        let n = decode(cell.normal)?;
        if let Some(acc) = &mut normals {
            for k in 0..3 {
                acc[k] = I::new(acc[k].lo.min(n[k].lo), acc[k].hi.max(n[k].hi))?;
            }
        } else {
            normals = Some(n);
        }
    }
    let Some(n) = normals else {
        return Ok(unresolved(cells, "bishop-cover-query-missing"));
    };
    let mut result =
        frame::certify_bishop_jets(path, encode(n), twist, traversal, max_cells - cells)?;
    result.cells += cells;
    Ok(result)
}

#[cfg(test)]
mod tests;
