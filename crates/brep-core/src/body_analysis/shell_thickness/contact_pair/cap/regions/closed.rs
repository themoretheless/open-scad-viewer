//! Closed actual-carrier UV cuts; retained source and fitted lifts replay in full.
use super::*;
impl Regions {
    /// Retain both sides of a simple closed CCW UV contour. Every endpoint is
    /// new and owned by this cut; no boundary vertex or material side is guessed.
    pub fn split_closed_uv(mut self, face: usize, uv: &[Curve], limits: Limits) -> Result<Self> {
        require(self.cuts.len() < 64, "At most 64 retained cap face cuts")?;
        let (model, record) = apply(&self.source, &self.model, face, uv, limits)?;
        self.model = model;
        self.cuts.push(record);
        Ok(self)
    }
}

pub(super) fn apply(
    source: &Sheet,
    model: &Model,
    face: usize,
    uv: &[Curve],
    limits: Limits,
) -> Result<(Model, Record)> {
    validate(limits)?;
    require(
        (2..=64).contains(&uv.len()),
        "Choose 2..64 closed UV cut segments",
    )?;
    for c in uv {
        c.validate()?;
        require(
            c.control_points[0].len() == 2 && clamped(c) && c.weights.iter().all(|w| *w > 0.),
            "Closed cap cut needs positive clamped 2D curves",
        )?;
    }
    for (i, c) in uv.iter().enumerate() {
        require(
            c.control_points.last() == Some(&uv[(i + 1) % uv.len()].control_points[0]),
            "Closed UV cut needs literal common endpoints including its seam",
        )?;
    }
    let carrier = &model
        .faces
        .get(face)
        .ok_or_else(|| crate::Error::new("BREP_OFFSET_CAP_REFUSED", "Unknown closed-cut cap face"))?
        .surface;
    require(
        carrier == source.cap().surface(),
        "Closed cut carrier differs from sealed actual surface",
    )?;
    // Values propose common vertices. The full fitted curves, including these
    // literal endpoints, need whole actual-carrier proofs before publication.
    let vertices = uv
        .iter()
        .map(|c| {
            let p = &c.control_points[0];
            carrier.evaluate(p[0], p[1]).map(|e| e.point)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut path = vec![];
    let mut fitting = vec![];
    for (i, c) in uv.iter().enumerate() {
        let fit = lift::fit(
            c,
            carrier,
            0.,
            model.tolerance_mm,
            Some([vertices[i], vertices[(i + 1) % uv.len()]]),
            limits.fitting,
        )?;
        let curve = fit
            .curve
            .as_ref()
            .ok_or_else(|| {
                crate::Error::new(
                    "BREP_OFFSET_CAP_REFUSED",
                    "Closed actual-cap lift remains unproved",
                )
            })?
            .clone();
        path.push(face_cut::Segment {
            curve,
            pcurve: c.clone(),
        });
        fitting.push(fit);
    }
    let result = face_cut::closed_with_budget(
        model,
        face,
        &path,
        source.cap().tolerance_uv(),
        limits.cut_cells,
        limits.exact_work,
    )?;
    // The record uses two new seam vertices only as retained metadata. Closed
    // replay uses the complete saved contour rather than open endpoint binding.
    let endpoints = result.model.edges[result.edges[0]].vertices;
    let (model, mut record) = finish(
        source,
        face,
        endpoints,
        uv.to_vec(),
        None,
        fitting,
        limits,
        result,
    )?;
    record.closed = true;
    Ok((model, record))
}
