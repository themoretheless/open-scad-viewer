use super::*;
use value_codec::{Value, json};

pub fn periodic_sheet_delivery(request: &Value, export_step: bool) -> Result<Value> {
    let surface = crate::field(request, "surface")?;
    let contours: Vec<Vec<[f64; 2]>> = crate::field(request, "contours")?;
    let contours = contours.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let tolerance = crate::field(request, "tolerance_mm")?;
    let max_work = crate::field(request, "max_work")?;
    if !(1..=1_000_000).contains(&max_work) {
        return Err(input("Invalid periodic sheet work limit"));
    }
    let (sheet, mesh, text) = if export_step {
        let r = cad_step::periodic_trim_step::export_exact_periodic_trim_sheet_step(
            &surface, &contours, tolerance, max_work,
        )?;
        (r.sheet, None, r.text)
    } else {
        let r = tessellate_exact_periodic_trim_sheet(
            &surface,
            &contours,
            tolerance,
            max_work,
            crate::field(request, "segments")?,
        )?;
        (r.sheet, r.tessellation, None)
    };
    Ok(
        json!({"model":sheet.model,"workUsed":sheet.work_used,"refusal":sheet.refusal.map(|r|format!("{r:?}")),
        "mesh":mesh,"step":text}),
    )
}
