//! Read-only original-source audit. Reports do not authorize body edits.
use crate::{Result, Value, field, require_exact_fields};
use nurbs_core::{
    surface::Surface,
    surface_self_intersection::{self as audit, Status},
};
use value_codec::json;

pub(crate) fn run(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &["op", "surface", "maxSpans"],
        "surface self-intersection audit",
    )?;
    let surface: Surface = field(&v, "surface")?;
    let max_spans: usize = field(&v, "maxSpans")?;
    let report = audit::inspect(&surface, max_spans)?;
    let exact =
        report.exact_witness.as_ref().map(|pair| {
            pair.iter()
                .map(|uv| {
                    uv.iter().map(|x| json!({
            "numerator":x.numer().to_string(),"denominator":x.denom().to_string()
        })).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        });
    let status = match report.status {
        Status::Absent => "absent",
        Status::Present => "present",
        Status::Unresolved => "unresolved",
    };
    let crossing = report.certified_crossing.as_ref().map(|c| {
        json!({
            "method":"original-planar-krawczyk",
            "parameterBoxes":c.parameter_boxes,"positionBounds":c.position_bounds,"cells":c.cells
        })
    });
    let attempts = report
        .probe_search
        .attempts
        .iter()
        .map(|p| {
            json!({
                "axis":p.axis,"parameter":p.parameter,"chargedCells":p.cells,
                "isoCoverageComplete":p.iso_coverage_complete,"refusal":p.refusal
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema":"nurbs-surface-self-intersection/1",
        "status":status,
        "absenceProven":report.status==Status::Absent,
        "presenceProven":report.status==Status::Present,
        "allContactBranchesEnumerated":false,
        "witness":report.witness,"exactWitness":exact,
        "certifiedCrossing":crossing,
        "probeSearch":{"attempts":attempts,"chargedCells":report.probe_search.cells,
            "budgetExhausted":report.probe_search.exhausted,"surfaceCoverageComplete":false},
        "witnessCoordinates":"normalized-original-active-domain",
        "reflectedAxis":report.reflected_axis,
        "injectivity":{"proven":report.injectivity.proven,
            "spans":report.injectivity.spans,"reason":report.injectivity.reason},
        "maxSpans":max_spans
    }))
}

/// A caller-selected interior fiber is not a whole-surface coverage claim.
pub(crate) fn run_iso(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &[
            "op",
            "surface",
            "axis",
            "parameter",
            "tolerance",
            "maxSpans",
        ],
        "surface iso self-intersection audit",
    )?;
    let surface: Surface = field(&v, "surface")?;
    let axis: usize = field(&v, "axis")?;
    let parameter: f64 = field(&v, "parameter")?;
    let tolerance: f64 = field(&v, "tolerance")?;
    let max_spans: usize = field(&v, "maxSpans")?;
    let report = nurbs_core::intersection::intersect_surface_iso_at_certified_report(
        &surface, axis, parameter, tolerance, max_spans,
    )?;
    let domain = if axis == 0 {
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ]
    } else {
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ]
    };
    use nurbs_core::interval_eval::Interval as I;
    let width = I::point(domain[1]).sub(I::point(domain[0]))?;
    let mut crossings = Vec::new();
    for c in &report.crossings {
        let mut boxes = [[[parameter, parameter]; 2]; 2];
        let mut interior = true;
        for (i, d) in c.parameters.into_iter().enumerate() {
            let t = I::new(d[0], d[1])?.sub(I::point(domain[0]))?.div(width)?;
            if t.lo <= 0. || t.hi >= 1. {
                interior = false;
                break;
            }
            boxes[i][axis] = [t.lo, t.hi];
        }
        if interior
            && (boxes[0][axis][1] < boxes[1][axis][0] || boxes[1][axis][1] < boxes[0][axis][0])
        {
            crossings.push(json!({"method":"original-planar-krawczyk","parameterBoxes":boxes,"positionBounds":c.position_bounds}));
        }
    }
    let presence = !crossings.is_empty();
    let iso_absence =
        report.complete && !report.identical_parameter_overlap && report.crossings.is_empty();
    let coverage = report.complete
        && !report.identical_parameter_overlap
        && crossings.len() == report.crossings.len();
    Ok(json!({"schema":"nurbs-surface-iso-self-intersection/1",
     "status":if presence{"present"}else if iso_absence{"iso-simple"}else{"unresolved"},
     "axis":axis,"parameter":parameter,"maxSpans":max_spans,"cells":report.cells,
     "presenceProven":presence,"isoAbsenceProven":iso_absence,"surfaceAbsenceProven":false,
     "isoCoverageComplete":coverage,"allContactBranchesEnumerated":false,
     "witnessCoordinates":"normalized-original-active-domain","crossings":crossings,
     "unresolvedCurveParameters":report.unresolved
    }))
}
