use super::*;
pub(super) fn section_data(s: section::ExactPlanarSection) -> Value {
    let coordinates=s.events.into_iter().map(|e|json!({"clock":events::clock(e.clock),"parameterBounds":e.parameter_bounds.map(|x|x.to_string()),"onBoundary":e.on_boundary})).collect::<Vec<_>>();
    json!({"line":line(s.line),"events":coordinates,"samples":s.samples.into_iter().map(|q|q.to_string()).collect::<Vec<_>>()})
}
pub(super) fn measure(
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
    let (mut out, used, cells) = match op {
        "classify_section_gaps" => {
            use section::ClassifiedPlanarSection::*;
            let r = section::classify_quadratic_planar_section(left, right, opts)?;
            let out = match r.outcome {
                Transverse { section, gaps } => {
                    json!({"status":"transverse","section":section_data(*section),"gaps":gaps.into_iter().map(|g|g.map(|loc|format!("{loc:?}"))).collect::<Vec<_>>()})
                }
                Parallel => json!({"status":"parallel"}),
                Coincident => json!({"status":"coincident"}),
                Refused(e) => json!({"status":"refused","reason":format!("{e:?}")}),
            };
            (out, r.work_used, r.winding_cells)
        }
        "assemble_section_material" => {
            use section::PlanarSectionMaterial::*;
            let r = section::assemble_quadratic_planar_section_material(left, right, opts)?;
            let out = match r.outcome {
                Transverse {
                    section,
                    proven,
                    possible,
                } => {
                    json!({"status":"transverse","section":section_data(*section),"proven":proven,"possible":possible})
                }
                Parallel => json!({"status":"parallel"}),
                Coincident => json!({"status":"coincident"}),
                Refused(e) => json!({"status":"refused","reason":format!("{e:?}")}),
            };
            (out, r.work_used, r.winding_cells)
        }
        "measure_section_material" => {
            use section::PlanarSectionLength::*;
            let r = section::measure_quadratic_planar_section_material(
                left,
                right,
                opts,
                field(v, "endpoint_tolerance")?,
            )?;
            let out = match r.outcome {
                Transverse {
                    clock_length,
                    xyz_length_squared,
                } => {
                    json!({"status":"transverse","clockLength":clock_length.map(|q|q.to_string()),"xyzLengthSquared":xyz_length_squared.map(|q|q.to_string())})
                }
                Parallel => json!({"status":"parallel"}),
                Coincident => json!({"status":"coincident"}),
                Refused(e) => json!({"status":"refused","reason":format!("{e:?}")}),
            };
            (out, r.work_used, r.winding_cells)
        }
        _ => return Err(input("Unknown section material operation")),
    };
    out["workUsed"] = json!(prior + used);
    out["windingCells"] = json!(cells);
    Ok(out)
}
