//! Closed guided-frame jets in the ORIGINAL normalized arc station.
//! Periodic unit tangent jets replace raw source speed matching. Both source
//! positions, their regular arc maps and the complete joint frame cover remain
//! mandatory. Nothing here grants retained seams or boundary/solid credit.
use super::super::ideal_volume::{arc::ArcMap, arc_frames, charts};
use super::*;

pub(super) fn certify(
    s: &Sweep<'_>,
    guide: &Curve,
    mut out: ClosedPathFrameSmoothnessReport,
    max_cells: usize,
    max_exact_work: u64,
) -> Result<ClosedPathFrameSmoothnessReport> {
    // For a regular normalized arc curve X, X_s=L*T and
    // X_ss=L^2*T_length. The path frame uses T through order k;
    // the guide offset uses its position through order k, hence T through k-1.
    for (curve, order) in [(s.path, out.order), (guide, out.order - 1)] {
        for endpoints in [false, true] {
            let position = super::super::endpoint_jets::certify_position(
                curve,
                endpoints,
                max_exact_work - out.exact_work,
            )?;
            out.exact_work += position.exact_work;
            if !position.certified {
                out.reason = Some("closed-guided-arc-position-jets-unproved");
                return Ok(out);
            }
            let tangent = super::super::endpoint_jets::certify_direction(
                curve,
                order,
                endpoints,
                true,
                max_exact_work - out.exact_work,
            )?;
            out.exact_work += tangent.exact_work;
            if !tangent.certified {
                out.reason = Some("closed-guided-arc-direction-jets-unproved");
                return Ok(out);
            }
        }
    }
    if !knot_continuity(s.twist, out.order, &mut out.exact_work, max_exact_work)? {
        out.reason = Some("closed-guided-frame-knot-jets-unproved");
        return Ok(out);
    }
    let twist =
        super::super::endpoint_jets::certify(s.twist, out.order, max_exact_work - out.exact_work)?;
    out.exact_work += twist.exact_work;
    if !twist.certified {
        out.reason = Some("closed-guided-frame-endpoint-jets-unproved");
        return Ok(out);
    }
    out.reason = Some("closed-guided-arc-source-speed-unproved");
    let Some(path_arc) = ArcMap::build(s.path, &mut out.cells, max_cells)? else {
        return Ok(out);
    };
    // Literal original coefficient identity proves a common normalized arc
    // clock. Failure consumes work and uses independent source arc maps.
    let identity = super::super::arc_guide::certify(s.path, guide, max_cells - out.cells)?;
    out.cells += identity.cells;
    let shared = identity.scale_upper.is_some();
    let guide_arc = if shared {
        None
    } else {
        let Some(map) = ArcMap::build(guide, &mut out.cells, max_cells)? else {
            return Ok(out);
        };
        Some(map)
    };
    let offset = if shared {
        charts::exact_offset(s.path, guide, &mut out.cells, max_cells)?
    } else {
        None
    };
    let zero = constant_vector_law([0.; 3])?;
    let mut pending = vec![[0., 1.]];
    out.reason = Some("closed-guided-arc-joint-frame-cover-unproved");
    while let Some(station) = pending.pop() {
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        let Some(p) = path_arc.phase(station, &mut out.cells, max_cells)? else {
            return Ok(out);
        };
        let g = if let Some(map) = &guide_arc {
            let Some(g) = map.phase(station, &mut out.cells, max_cells)? else {
                return Ok(out);
            };
            g
        } else {
            p
        };
        let proof = arc_frames::arc_frame(
            s,
            station,
            p,
            Some(g),
            offset.as_ref(),
            None,
            shared,
            &zero,
            max_cells - out.cells,
        )?;
        if let Some(proof) = proof {
            out.cells += proof.cells;
            if proof.status == Status::Certified {
                continue;
            }
        }
        let mid = crate::distance_bounds::midpoint(station[0], station[1]);
        if !(station[0] < mid && mid < station[1]) {
            return Ok(out);
        }
        pending.push([mid, station[1]]);
        pending.push([station[0], mid]);
    }
    out.closed_source_frame_smoothness_certified = true;
    out.reason = None;
    Ok(out)
}

#[cfg(test)]
#[path = "../tests/guided_arc.rs"]
mod tests;
