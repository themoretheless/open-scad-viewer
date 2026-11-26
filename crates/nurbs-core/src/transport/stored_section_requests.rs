use super::*;

fn errors(e: section::PlanarSegmentError) -> Value {
    json!({"spatial":e.spatial.to_string(),"liftedPcurves":e.lifted_pcurves.map(|q|q.to_string()),"correspondence":e.correspondence.map(|q|q.to_string())})
}

pub(super) fn store(
    v: &Value,
    op: &str,
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: CoplanarRayPartitionOptions,
    prior: u64,
) -> Result<Value> {
    let index: usize = field(v, "edge_index")?;
    let tolerance = field(v, "tolerance_mm")?;
    let limit = options.max_work;
    if op == "store_section_edges" {
        let r = section::store_trimmed_planar_intersection(left, right, options, tolerance)?;
        let section::StoredPlanarIntersection::Transverse {
            line: l,
            edges,
            possible,
        } = r.outcome
        else {
            return Err(input(format!(
                "Section edge storage refused: {:?}",
                r.outcome
            )));
        };
        let curve = edges
            .get(index)
            .ok_or_else(|| input("Unknown section edge"))?
            .spatial
            .clone();
        let edges=edges.into_iter().map(|e|json!({"parameter":e.parameter.map(|q|q.to_string()),"spatial":e.spatial,"pcurves":e.pcurves,"squaredError":errors(e.squared_error)})).collect::<Vec<_>>();
        return Ok(
            json!({"curve":curve,"line":line(l),"edges":edges,"possible":ranges(possible),"selectedEdge":index,"workUsed":prior+r.work_used,"windingCells":r.winding_cells}),
        );
    }
    let r = section::build_planar_intersection_segments(left, right, options)?;
    let section::PlanarIntersectionSegments::Transverse { segments, .. } = r.outcome else {
        return Err(input(format!(
            "Section construction refused: {:?}",
            r.outcome
        )));
    };
    let s = segments
        .get(index)
        .ok_or_else(|| input("Unknown section edge"))?;
    crate::check(r.work_used < limit, "Section storage work limit reached")?;
    let charts = [&left.projection, &right.projection];
    let stored = if op == "optimize_section_edge" {
        section::optimize_planar_intersection_segment_storage(
            s,
            charts,
            tolerance,
            limit - r.work_used,
        )?
    } else {
        section::store_planar_intersection_segment(s, charts, tolerance, limit - r.work_used)?
    };
    let section::PlanarSegmentStorage::Stored {
        spatial,
        pcurves,
        squared_error,
    } = stored.outcome
    else {
        return Err(input(format!(
            "Section storage refused: {:?}",
            stored.outcome
        )));
    };
    Ok(
        json!({"curve":spatial,"pcurves":pcurves,"parameter":s.parameter.each_ref().map(|q|q.to_string()),
        "squaredError":errors(squared_error),"selectedEdge":index,"edgeCount":segments.len(),"workUsed":prior+r.work_used+stored.work_used,"windingCells":r.winding_cells}),
    )
}
