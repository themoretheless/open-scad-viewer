use super::*;
use crate::editing::exact_surface_derivative as gap;

pub(super) fn inspect(v: &Value, op: &str) -> Result<Value> {
    let surface = field(v, "surface")?;
    let budget = field(v, "max_work")?;
    match op {
        "surface_umbilic_cells" => {
            let r = gap::audit_umbilics(&surface, budget)?;
            let cells = r
                .cells
                .ok_or_else(|| input(format!("Curvature gap audit refused: {:?}", r.refusal)))?;
            let rows: Vec<_> = cells
                .iter()
                .map(|c| json!({"domain":c.domain,"status":format!("{:?}",c.status)}))
                .collect();
            Ok(json!({"cells":rows,"workUsed":r.work_used}))
        }
        "surface_umbilic_regions" => {
            let r = gap::refine_umbilics(&surface, field(v, "max_depth")?, budget)?;
            let regions = r
                .regions
                .ok_or_else(|| input(format!("Curvature gap audit refused: {:?}", r.refusal)))?;
            let rows:Vec<_>=regions.iter().map(|c|json!({"originalDomain":c.original_domain,"unitDomain":c.unit_domain,"status":format!("{:?}",c.status)})).collect();
            Ok(json!({"regions":rows,"workUsed":r.work_used}))
        }
        "surface_gap_comparison" => {
            let r = gap::compare_curvature_gap(&surface, field(v, "gap_level_per_mm2")?, budget)?;
            let cells = r
                .cells
                .ok_or_else(|| input(format!("Curvature gap audit refused: {:?}", r.refusal)))?;
            let rows: Vec<_> = cells
                .iter()
                .map(|c| json!({"domain":c.domain,"comparison":format!("{:?}",c.comparison)}))
                .collect();
            Ok(json!({"cells":rows,"workUsed":r.work_used}))
        }
        "surface_gap_ranges" => {
            let r = gap::curvature_gap_ranges(&surface, budget)?;
            let cells = r
                .cells
                .ok_or_else(|| input(format!("Curvature gap audit refused: {:?}", r.refusal)))?;
            let rows: Vec<_> = cells
                .iter()
                .map(|c| json!({"domain":c.domain,"boundsPerMm2":c.bounds}))
                .collect();
            Ok(json!({"cells":rows,"workUsed":r.work_used}))
        }
        "surface_gap_regions" => {
            let r = gap::refine_curvature_gap(
                &surface,
                field(v, "gap_level_per_mm2")?,
                field(v, "max_depth")?,
                budget,
            )?;
            let regions = r
                .regions
                .ok_or_else(|| input(format!("Curvature gap audit refused: {:?}", r.refusal)))?;
            let rows:Vec<_>=regions.iter().map(|c|json!({"originalDomain":c.original_domain,"unitDomain":c.unit_domain,"comparison":format!("{:?}",c.comparison)})).collect();
            Ok(json!({"regions":rows,"workUsed":r.work_used}))
        }
        _ => Err(input("Unknown curvature gap audit")),
    }
}
