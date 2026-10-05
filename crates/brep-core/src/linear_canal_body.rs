//! Fresh source-shell, embedding and volume gates for authored capped canals.
//! The resulting native body is not a conventional Model or STEP admission.
use crate::{linear_canal, source_shell_geometry, source_volume};

pub struct Limits {
    pub tolerance_mm: f64,
    pub tolerance_uv: f64,
    /// Region work is bounded independently per original face.
    pub regions: crate::trimmed_face_recipe::Limits,
    pub incidence_exact_work: u64,
    pub embedding: source_shell_geometry::Limits,
    pub volume: source_volume::Limits,
}
pub struct Report {
    /// Successful source payloads are moved into the next private gate.
    pub incidence: crate::source_shell_incidence::Report,
    pub embedding: Option<source_shell_geometry::Report>,
    pub volume: Option<source_volume::Report>,
}
impl Report {
    pub fn body(&self) -> Option<&source_volume::Body> {
        self.volume.as_ref()?.body.as_ref()
    }
    pub fn reason(&self) -> &'static str {
        self.volume
            .as_ref()
            .map(|r| r.reason)
            .or_else(|| self.embedding.as_ref().map(|r| r.reason))
            .unwrap_or(self.incidence.reason)
    }
}
/// Rebuild all gates from immutable authored spans. Diagnostics or caller
/// certificate flags never skip incidence, injectivity, contacts or volume.
pub fn qualify(spans: &[linear_canal::Span], limits: Limits) -> crate::Result<Report> {
    let mut incidence = linear_canal::to_capped_source_shell(
        spans,
        limits.tolerance_mm,
        limits.tolerance_uv,
        limits.regions,
        limits.incidence_exact_work,
    )?;
    let shell = incidence.shell.take();
    let mut out = Report {
        incidence,
        embedding: None,
        volume: None,
    };
    let Some(shell) = shell else {
        return Ok(out);
    };
    let mut embedding = source_shell_geometry::qualify(shell, limits.embedding)
        .map_err(|e| crate::Error::new(e.code, e.message))?;
    let geometry = embedding.geometry.take();
    out.embedding = Some(embedding);
    if let Some(geometry) = geometry {
        out.volume = Some(
            source_volume::qualify(geometry, limits.volume)
                .map_err(|e| crate::Error::new(e.code, e.message))?,
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits(volume_cells: usize) -> Limits {
        Limits {
            tolerance_mm: 1e-7,
            tolerance_uv: 1e-8,
            regions: crate::trimmed_face_recipe::Limits {
                pairs: 10000,
                region_cells: 10000,
                domain_cells: 10000,
                agreement_cells: 10000,
            },
            incidence_exact_work: 100_000_000,
            embedding: source_shell_geometry::Limits {
                tolerance_uv: 1e-8,
                corners: 10000,
                spans: 20000,
                linear_cells: 10000,
                pairs: crate::face_contacts::Limits {
                    pairs: 10000,
                    cells: 10000,
                    domain_cells: 100000,
                    cells_per_pair: 32,
                    domain_cells_per_pair: 512,
                },
                exact_work: 10000000,
                driver_cells: 10000,
            },
            volume: source_volume::Limits {
                axis: 2,
                origin: 5.,
                absolute_error: 0.25,
                tolerance_uv: 1e-8,
                cells: volume_cells,
                spans: 100000,
                domain_cells: 100000,
            },
        }
    }
    #[test]
    fn capped_body_recomputes_all_gates_and_encloses_independent_cap_frustum_volume() {
        let spans = linear_canal::construct(
            [[10., -7., 5.], [13., -3., 17.]],
            [0.5, 1.25],
            [1., 0., 0.],
            std::f64::consts::TAU,
        )
        .unwrap();
        let report = qualify(&spans, limits(50000)).unwrap();
        let body = report.body().expect(report.reason());
        let r = [0.5_f64, 1.25];
        let length = 13_f64;
        let slope = (r[1] - r[0]) / length;
        let k2 = 1. - slope * slope;
        let h = [r[0] * (1. - slope), r[1] * (1. + slope)];
        // Independent axial frustum plus two spherical segment integrals.
        let expected = std::f64::consts::PI
            * (length * k2 * k2 * (r[0] * r[0] + r[0] * r[1] + r[1] * r[1]) / 3.
                + h[0] * h[0] * (r[0] - h[0] / 3.)
                + h[1] * h[1] * (r[1] - h[1] / 3.));
        let bounds = body.volume();
        assert!(
            bounds[0] <= expected && expected <= bounds[1],
            "{expected} outside {bounds:?}"
        );
        assert!(bounds[0] > 0. && bounds[1] - bounds[0] <= 0.25);
        assert!(body.reverse_orientation());
        assert_eq!(body.geometry().contacts().pairs.len(), 120);
        assert!(body.geometry().contacts().all_pairs_qualified);
        eprintln!(
            "capped body: {} volume={bounds:?} oracle={expected} cells={} spans={}",
            report.reason(),
            report.volume.as_ref().unwrap().cells,
            report.volume.as_ref().unwrap().spans
        );
        let model = crate::source_body_model::convert(body, 1e-7).unwrap();
        assert_eq!(model.bodies.len(), 1);
        assert!(model.shells[0].closed);
        assert_eq!(model.faces.len(), body.geometry().shell().faces().len());
        assert_eq!(
            model,
            crate::source_body_model::convert(body, 1e-7).unwrap()
        );
        let encoded = value_codec::to_value(&model).unwrap();
        let restored: crate::Model = value_codec::from_value(encoded).unwrap();
        assert_eq!(restored, model);
        let (step, _, _) = crate::export_step_v6(&model).unwrap();
        let (imported, _, _) = crate::import_step_v6(&step).unwrap();
        crate::source_body_model::assert_step_identity(&model, &imported);
        if let Some(path) = std::env::var_os("CAD_SOURCE_MODEL_STEP_OUTPUT") {
            std::fs::write(path, step).unwrap();
        }
        let exhausted = qualify(&spans, limits(1)).unwrap();
        assert!(exhausted.body().is_none());
        assert_eq!(exhausted.reason(), "source-volume-initial-work-limit");
        assert!(exhausted.embedding.is_some());
    }
}
