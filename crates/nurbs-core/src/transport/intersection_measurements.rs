//! Original-domain measurements. Unresolved parameter cells are returned to the caller.
use super::*;

pub(super) fn inspect(v: &Value, op: &str) -> Result<Value> {
    let budget = field(v,"max_cells")?;
    match op {
        "curve_plane_roots" => {
            let r = crate::curve_plane::inspect(&field(v,"curve")?, field(v,"plane_normal")?,
                field(v,"plane_offset_mm")?, field(v,"parameter_tolerance")?, budget)?;
            let cells = r.cells.iter().map(|c| json!({"span":c.span,"domain":c.domain,
                "residualBounds":c.residual_bounds,"rootBounds":c.root_parameter_bounds(),"multiplicity":c.root_multiplicity,
                "classification":format!("{:?}",c.classification),"stopReason":format!("{:?}",c.stop_reason),
                "proof":c.proof.map(|p|format!("{p:?}"))})).collect::<Vec<_>>();
            Ok(json!({"cells":cells,"work":r.work,"exactPredicateWork":r.exact_predicate_work,
                "exactBudgetExhausted":r.exact_predicate_budget_exhausted,"allRootsIsolated":r.all_roots_isolated,
                "coverageResolved":r.coverage_resolved}))
        }
        "curve_pair_crossings" => {
            let r = crate::intersection::intersect_curve_curve_certified_report(&field(v,"curve")?,
                &field(v,"target")?,field(v,"tolerance_mm")?,budget)?;
            let roots = r.crossings.iter().map(|c| json!({"parameters":c.parameters,"positionBounds":c.position_bounds,
                "contact":format!("{:?}",c.contact)})).collect::<Vec<_>>();
            Ok(json!({"crossings":roots,"identicalParameterOverlap":r.identical_parameter_overlap,
                "unresolved":r.unresolved,"complete":r.complete,"cells":r.cells}))
        }
        "curve_plane_chart_crossings" => {
            let r = crate::intersection::intersect_curve_plane_certified_report(&field(v,"curve")?,
                &field(v,"target")?,field(v,"tolerance_mm")?,budget)?;
            let roots = r.crossings.iter().map(|c| json!({"curveParameter":c.curve_parameter,
                "uvBounds":c.uv_bounds,"positionBounds":c.position_bounds,"contact":format!("{:?}",c.contact)})).collect::<Vec<_>>();
            let overlap = r.overlap.as_ref().map(|(curve,pcurve)| json!({"curve":curve,"pcurve":pcurve}));
            Ok(json!({"crossings":roots,"overlap":overlap,"unresolved":r.unresolved_curve_domains,"complete":r.complete,"cells":r.cells}))
        }
        "surface_ray_hits" => {
            let r=crate::ray_surface::ray_intersections(&field(v,"surface")?,field(v,"ray_origin")?,
                field(v,"ray_direction")?,field(v,"parameter_tolerance")?,budget)?;
            let roots=r.roots.iter().map(|h|json!({"uvBounds":h.uv,"rayParameter":h.parameter})).collect::<Vec<_>>();
            Ok(json!({"roots":roots,"unresolved":r.unresolved,"complete":r.complete,"cells":r.cells}))
        }
        _ => Err(input("Unknown intersection measurement")),
    }
}
