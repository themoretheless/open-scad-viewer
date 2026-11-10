//! Diagnostic whole-panel fields, independently of any seam construction flag.
use super::{ContactQualificationLimits, panel, refused};
use crate::face_curvature;
use nurbs_core::{Result, surface::Surface};
pub struct RunoutPanelField {
    pub patch: usize,
    pub injectivity: nurbs_core::surface_linear_monotonicity::Report,
    pub curvature: Option<face_curvature::Report>,
}
pub struct RunoutPanelFields {
    pub panels: Vec<RunoutPanelField>,
    pub complete: bool,
    pub within_tolerance: bool,
    pub cells: usize,
    pub domain_cells: usize,
    pub shape_cells: usize,
}
/// Fresh checks of actual surfaces. Empty/exhausted coverage never grants
/// admission; a witnessed curvature excess is a complete negative decision.
/// Whole-chart injectivity concerns each panel, not intersections between them.
pub fn inspect_runout_panels(
    surfaces: &[Surface],
    tolerance_mm: f64,
    tolerance_uv: f64,
    limits: ContactQualificationLimits,
) -> Result<RunoutPanelFields> {
    if surfaces.len() > 256
        || limits.cells > 100_000
        || limits.domain_cells > 1_000_000
        || limits.shape_cells > 1_000_000
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(refused(
            "Runout fields need bounded work and positive tolerances",
        ));
    }
    let mut out = RunoutPanelFields {
        panels: vec![],
        complete: false,
        within_tolerance: false,
        cells: 0,
        domain_cells: 0,
        shape_cells: 0,
    };
    for (patch, s) in surfaces.iter().enumerate() {
        s.validate()?;
        if s.control_points[0][0].len() != 3
            || s.control_points
                .len()
                .saturating_mul(s.control_points[0].len())
                > 4096
        {
            return Err(refused("Runout field input must be a bounded 3D surface"));
        }
        if out.cells >= limits.cells
            || out.domain_cells >= limits.domain_cells
            || out.shape_cells >= limits.shape_cells
        {
            break;
        }
        let injectivity = nurbs_core::surface_linear_monotonicity::inspect_candidate(
            s,
            limits.cells - out.cells,
        )?;
        out.cells += injectivity.cells;
        let curvature = if out.cells < limits.cells {
            let model = panel(s, tolerance_mm)?;
            let field = face_curvature::inspect(
                &model,
                0,
                10.,
                tolerance_uv,
                face_curvature::Limits {
                    cells: (limits.cells - out.cells).min(50_000),
                    domain_cells: limits.domain_cells - out.domain_cells,
                    shape_cells: limits.shape_cells - out.shape_cells,
                },
                &[],
            )?;
            out.cells += field.cells;
            out.domain_cells += field.domain_cells;
            out.shape_cells += field.shape_cells;
            Some(field)
        } else {
            None
        };
        out.panels.push(RunoutPanelField {
            patch,
            injectivity,
            curvature,
        });
    }
    out.complete = !surfaces.is_empty()
        && out.panels.len() == surfaces.len()
        && out.panels.iter().all(|p| {
            p.injectivity.certified
                && p.curvature
                    .as_ref()
                    .is_some_and(|c| c.decision != face_curvature::Decision::Unresolved)
        });
    out.within_tolerance = out.complete
        && out.panels.iter().all(|p| {
            p.curvature.as_ref().is_some_and(|c| {
                c.decision == face_curvature::Decision::Within
                    && c.absolute_curvature_upper
                        .is_some_and(|x| x.is_finite() && x <= 10.)
            })
        });
    Ok(out)
}
