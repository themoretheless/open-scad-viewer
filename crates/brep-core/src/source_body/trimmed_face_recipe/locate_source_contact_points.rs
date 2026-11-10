use super::*;

/// Build immutable source-point expressions for freshly found face crossings.
/// Independent phase budgets bound search, point requalification and mapping.
pub fn locate_source_contact_points(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    contact: &Curve,
    tolerance_uv: f64,
    limits: Limits,
    max_cells: usize,
    target_width: [f64; 2],
    max_point_checks: usize,
    max_mapping_cells: usize,
) -> Result<SourcePointSearch> {
    if !(1..=100000).contains(&max_point_checks) || !(1..=100000).contains(&max_mapping_cells) {
        return Err(Error::new(
            "BREP_SOURCE_POINT_WORK",
            "Source point phase budgets must be bounded and positive",
        ));
    }
    let search = locate_contact_boundaries(
        context,
        surface,
        wires,
        contact,
        tolerance_uv,
        limits,
        max_cells,
        target_width,
    )?;
    let mut out = SourcePointSearch {
        complete: search.complete && search.precision_proven,
        search,
        points: vec![],
        point_checks: 0,
        mapping_cells: 0,
    };
    for crossing in &out.search.crossings {
        for (crossing_cell, cell) in crossing.report.cells.iter().enumerate() {
            if cell.state != nurbs_core::uv_curve_crossings::State::Unique {
                continue;
            }
            let report =
                if out.point_checks < max_point_checks && out.mapping_cells < max_mapping_cells {
                    out.point_checks += 1;
                    let r = crate::source_contact_point::qualify(
                        surface,
                        &wires[crossing.loop_index][crossing.boundary_index].pcurve,
                        contact,
                        cell.parameters,
                        max_mapping_cells - out.mapping_cells,
                    )?;
                    out.mapping_cells += r.mapping_cells;
                    Some(r)
                } else {
                    None
                };
            out.complete &= report.as_ref().is_some_and(|r| r.point.is_some());
            out.points.push(AddressedSourcePoint {
                loop_index: crossing.loop_index,
                boundary_index: crossing.boundary_index,
                crossing_cell,
                report,
            });
        }
    }
    Ok(out)
}
