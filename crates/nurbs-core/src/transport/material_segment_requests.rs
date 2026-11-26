use super::*;
pub(super) fn build(
    v: &Value,
    op: &str,
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    work: u64,
    prior: u64,
) -> Result<Value> {
    let opts = section::PlanarSectionWindingOptions {
        winding_tolerance: field(v, "tolerance_uv")?,
        max_winding_cells: field(v, "max_winding_cells")?,
        max_work: work,
    };
    let r = section::build_quadratic_planar_section_geometry(
        left,
        right,
        opts,
        field(v, "endpoint_tolerance")?,
    )?;
    let mut used = r.work_used;
    use section::PlanarSectionGeometryOutcome::*;
    let mut out = match r.outcome {
        Transverse(data) => {
            let mut result = json!({"status":"transverse","section":material::section_data(*data.section),"points":data.points,"possible":data.possible});
            if op == "material_segment_curve" {
                let index: usize = field(v, "edge_index")?;
                let edge = data
                    .edges
                    .get(index)
                    .ok_or_else(|| input("Unknown material segment"))?;
                crate::check(used < work, "Material segment storage work limit reached")?;
                let stored = section::store_planar_intersection_segment(
                    &edge.segment,
                    [&left.projection, &right.projection],
                    field(v, "tolerance_mm")?,
                    work - used,
                )?;
                used += stored.work_used;
                let section::PlanarSegmentStorage::Stored {
                    spatial,
                    pcurves,
                    squared_error: e,
                } = stored.outcome
                else {
                    return Err(input(format!(
                        "Material segment storage refused: {:?}",
                        stored.outcome
                    )));
                };
                result["curve"] = json!(spatial);
                result["storedPcurves"] = json!(pcurves);
                result["selectedEdge"] = json!(index);
                result["storageErrorSquared"] = json!({"spatial":e.spatial.to_string(),"liftedPcurves":e.lifted_pcurves.map(|q|q.to_string()),"correspondence":e.correspondence.map(|q|q.to_string())});
            }
            result["edges"]=json!(data.edges.into_iter().map(|e|json!({"events":e.events,"endpointErrorSquared":e.endpoint_error_squared.map(|q|q.to_string()),"parameter":e.segment.parameter.map(|q|q.to_string()),"xyz":e.segment.xyz.map(|p|p.map(|q|q.to_string())),"pcurves":e.segment.pcurves.map(|c|c.map(|p|p.map(|q|q.to_string())))})).collect::<Vec<_>>());
            result
        }
        Parallel => json!({"status":"parallel"}),
        Coincident => json!({"status":"coincident"}),
        Refused(e) => json!({"status":"refused","reason":format!("{e:?}")}),
    };
    if op == "material_segment_curve" && out.get("curve").is_none() {
        return Err(input(format!("No material segment: {out}")));
    }
    out["workUsed"] = json!(prior + used);
    out["windingCells"] = json!(r.winding_cells);
    Ok(out)
}
