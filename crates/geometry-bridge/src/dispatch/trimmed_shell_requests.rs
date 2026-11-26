use crate::*;
use brep_core::trimmed_shell::{self, Limits, Patch};

pub(super) fn assemble(v: &Value) -> Result<Value> {
    let model: brep_core::Model = field(v, "model")?;
    model.validate_boundary_diagnostic_inputs()?;
    if model.shells.len() != 1 || model.shells[0].faces.len() != model.faces.len() {
        return Err(Error::new(
            "BREP_TRIMMED_SHELL_INPUT",
            "Select a single authored shell covering all input faces",
        ));
    }
    let patches: Vec<_> = model.shells[0]
        .faces
        .iter()
        .map(|usage| {
            let face = &model.faces[usage.face];
            Patch {
                surface: face.surface.clone(),
                outer: model.loops[face.outer].coedges.clone(),
                holes: face
                    .holes
                    .iter()
                    .map(|&wire| model.loops[wire].coedges.clone())
                    .collect(),
                reversed: usage.reversed,
            }
        })
        .collect();
    let report = trimmed_shell::assemble(
        model.vertices,
        model.edges,
        patches,
        Limits {
            tolerance_mm: field(v, "tolerance_mm")?,
            tolerance_uv: field(v, "tolerance_uv")?,
            agreement_cells: field(v, "agreement_cells")?,
            trim_pairs: field(v, "trim_pairs")?,
            trim_cells: field(v, "trim_cells")?,
            trim_domain_cells: field(v, "trim_domain_cells")?,
            regularity_cells: field(v, "regularity_cells")?,
        },
    )?;
    Ok(json!({
        "admitted":report.model.is_some(),
        "model":encode(report.model)?,
        "closure":format!("{:?}",report.closure.status),
        "trimRegionsValid":report.trims.all_valid,
        "allChartsSpanwiseRegular":report.regularity.iter().all(|chart| chart.as_ref().is_some_and(|r| r.spanwise_regular)),
        "regularityCells":report.regularity_cells,
        "vertexLinksValid":report.vertex_links_valid,
        "connected":report.connected,
        "solidBodyAuthored":false,
        "globalEmbeddingCertified":false
    }))
}
