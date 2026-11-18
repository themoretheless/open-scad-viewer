//! STEP delivery of NURBS sheets with trims authored across periodic seams.
use brep_core::feature_ops::periodic_trim_sheet::{
    PeriodicTrimSheetReport, build_exact_periodic_trim_sheet,
};
use nurbs_core::{Result, surface::Surface};

#[derive(Clone, Debug)]
pub struct PeriodicTrimStepReport {
    pub sheet: PeriodicTrimSheetReport,
    pub text: Option<String>,
}

/// Author exact periodic trim sheets and export the open-shell topology,
/// NURBS support and UV/world edge pairs through the STEP AP242 route.
pub fn export_exact_periodic_trim_sheet_step(
    source: &Surface,
    contours: &[&[[f64; 2]]],
    tolerance_mm: f64,
    max_work: u64,
) -> Result<PeriodicTrimStepReport> {
    let sheet = build_exact_periodic_trim_sheet(source, contours, tolerance_mm, max_work)?;
    let text = sheet
        .model
        .as_ref()
        .map(|model| crate::export_step_v9(model).map(|(text, _, _)| text))
        .transpose()?;
    Ok(PeriodicTrimStepReport { sheet, text })
}
