use super::*;

pub(super) fn inspect(v: &Value, op: &str) -> Result<Value> {
    let levels: Vec<f64> = field(v, "plane_offsets")?;
    let report = crate::parallel_sections::intersect(
        &field(v, "surface")?, field(v, "plane_normal")?, &levels,
        field(v, "tolerance_mm")?, field(v, "max_work")?,
    )?;
    let curve = if op == "parallel_section_piece" {
        let section = report.sections.get(field::<usize>(v, "section_index")?)
            .ok_or_else(|| input("Unknown parallel section index"))?;
        let piece = field::<usize>(v,"piece")?;
        Some(section.curves.get(piece).ok_or_else(|| input(format!(
            "No curve at piece {piece} in section with status {:?}: {:?}",section.status,section.stop
        )))?)
    } else { None };
    let sections = report.sections.iter().map(|s| json!({
        "level":s.level,"status":format!("{:?}",s.status),"stop":format!("{:?}",s.stop),
        "pieces":s.curves.len(),"errorUpperMm":s.error_upper,"solvedAxis":s.solved_axis,
        "unresolvedDomain":s.unresolved_domain,"work":s.work,
    })).collect::<Vec<_>>();
    Ok(json!({"sections":sections,"complete":report.complete,"work":report.work,
        "preparationWork":report.preparation_work,"curve":curve}))
}
