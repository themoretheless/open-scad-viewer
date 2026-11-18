//! Original NURBS iso-section proposals, verified on both original supports.
use super::{IntersectionCurve, invalid, unproven};
use nurbs_core::{
    Result,
    curve::Curve,
    curve_surface_agreement as agreement,
    intersection::certified_curve_surface,
    surface::{Axis, Surface},
};

pub struct IsoIntersection {
    pub cut: IntersectionCurve,
    pub fixed_parameter: f64,
    /// Complete coverage of the probe curve only, never full SS coverage.
    pub probe_roots: certified_curve_surface::Report,
}
/// Isolate a transverse root of an original parameter-line probe against the
/// second support, then propose the corresponding full original iso-section.
/// A probe root is not sufficient: the complete saved section must qualify on
/// both supports, including any requested owned endpoint edits. General SS
/// branch coverage and trimmed-domain membership remain separate obligations.
pub fn generate_iso_intersection_curve(
    surfaces: [&Surface; 2],
    fixed_axis: Axis,
    root: usize,
    endpoints: Option<[[f64; 3]; 2]>,
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<IsoIntersection> {
    for s in surfaces {
        s.validate()?;
    }
    if !(8..=100000).contains(&max_cells)
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || surfaces.iter().any(|s| s.periodic_u || s.periodic_v)
    {
        return Err(invalid(
            "Need nonperiodic original charts and bounded iso-section work",
        ));
    }
    let s = surfaces[0];
    let d = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let fixed = usize::from(matches!(fixed_axis, Axis::V));
    let probe = s.iso(if fixed == 0 { Axis::V } else { Axis::U }, d[1 - fixed][0])?;
    let roots = certified_curve_surface::intersections(&probe, surfaces[1], max_cells / 3)?;
    let mut cells = roots.cells;
    if !roots.complete {
        return Err(unproven("Original probe root coverage remains unresolved"));
    }
    let selected = roots
        .roots
        .get(root)
        .ok_or_else(|| invalid("Requested probe root is missing"))?;
    let at = crate::edge_partition::root_representative(selected.parameter_box[0], d[fixed]);
    if !(d[fixed][0] < at && at < d[fixed][1]) {
        return Err(unproven("Section needs an interior isolated probe root"));
    }
    let uv = |along| {
        if fixed == 0 {
            vec![at, along]
        } else {
            vec![along, at]
        }
    };
    let pc = Curve::from_polyline(vec![uv(d[1 - fixed][0]), uv(d[1 - fixed][1])])?;
    let lift = agreement::lift(
        &pc,
        s,
        endpoints,
        tolerance_mm * 0.5,
        (max_cells - cells) / 2,
    )?;
    cells += lift.cells;
    let curve = lift
        .curve
        .ok_or_else(|| unproven("Original iso-section lift is unresolved"))?;
    let first = lift
        .evidence
        .ok_or_else(|| unproven("Missing iso-section lift evidence"))?;
    if cells == max_cells {
        return Err(unproven("Shared iso-section work exhausted"));
    }
    let projected = nurbs_core::curve_pullback::project(
        &curve,
        surfaces[1],
        nurbs_core::curve_pullback::Limits {
            tolerance: tolerance_mm * 0.5,
            max_segments: 256,
            max_iterations: 32,
            agreement_cells: max_cells - cells,
        },
    )?;
    cells += projected.agreement_cells;
    let second = projected
        .agreement
        .ok_or_else(|| unproven("Complete section does not qualify on the second support"))?;
    let other = projected
        .pcurve
        .ok_or_else(|| unproven("Missing saved second-support pcurve"))?;
    if !projected.within_tolerance
        || first.status != agreement::Status::WithinTolerance
        || second.status != agreement::Status::WithinTolerance
    {
        return Err(unproven(
            "Complete saved section disagrees with an original support",
        ));
    }
    Ok(IsoIntersection {
        fixed_parameter: at,
        probe_roots: roots,
        cut: IntersectionCurve {
            curve,
            pcurves: [pc, other],
            agreements: [first, second],
            cells,
        },
    })
}
