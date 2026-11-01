//! Whole original face regularity bound to the admitted material body.
//! Embedding alone does not imply differential rank. This additional owner
//! recomputes rank on complete source charts, hence also on their trim regions.
//! It makes no G1/G2 claim across knots, seams, vertices or collapsed poles.
use super::Body;
use crate::source_seam_tangency::Carrier;
use nurbs_core::{Error, Result, surface_regularity};
pub mod stratified;

pub struct RegularBody<'a, E: Carrier> {
    body: &'a Body<E>,
    faces: Vec<surface_regularity::Report>,
}
impl<'a, E: Carrier> RegularBody<'a, E> {
    pub fn body(&self) -> &'a Body<E> {
        self.body
    }
    pub fn faces(&self) -> &[surface_regularity::Report] {
        &self.faces
    }
}
pub struct Report<'a, E: Carrier> {
    pub certificate: Option<RegularBody<'a, E>>,
    pub cells: usize,
    pub completed_faces: usize,
    pub uncertain_face: Option<usize>,
    pub reason: &'static str,
}

/// All faces, including filled caps, consume ONE shared allowance. A previous
/// injectivity report, retained mesh or caller flag never supplies rank evidence.
/// Declared polar singularities require a separate quotient/neighborhood owner;
/// they are deliberately not relabeled as regular original parameter charts.
pub fn qualify<E: Carrier>(body: &Body<E>, max_cells: usize) -> Result<Report<'_, E>> {
    if max_cells > 100000 {
        return Err(Error::new(
            "BREP_SOURCE_REGULARITY",
            "Choose at most 100000 regularity cells",
        ));
    }
    let shell = body.geometry().shell();
    // Validate every immutable source before even a zero-work refusal.
    for wires in shell.faces() {
        wires[0].edges()[0].surface().validate()?;
    }
    let mut out = Report {
        certificate: None,
        cells: 0,
        completed_faces: 0,
        uncertain_face: None,
        reason: "source-face-regularity-unproven",
    };
    let mut faces = Vec::new();
    for (index, wires) in shell.faces().iter().enumerate() {
        out.uncertain_face = Some(index);
        if shell.poles().iter().any(|(a, _)| a.face == index) {
            out.reason = "source-polar-neighborhood-required";
            return Ok(out);
        }
        let remaining = max_cells - out.cells;
        if remaining == 0 {
            out.reason = "source-regularity-work-limit";
            return Ok(out);
        }
        let proof = match surface_regularity::inspect(wires[0].edges()[0].surface(), remaining) {
            Ok(proof) => proof,
            Err(_) => {
                // An unresolved enclosure or initial grid consumes the remaining
                // allowance; never restart an unaccounted budget on another face.
                out.cells = max_cells;
                return Ok(out);
            }
        };
        out.cells += proof.cells;
        if !proof.spanwise_regular {
            return Ok(out);
        }
        faces.push(proof);
        out.completed_faces += 1;
    }
    if !faces.is_empty() {
        out.certificate = Some(RegularBody { body, faces });
        out.uncertain_face = None;
        out.reason = "source-body-piecewise-regular-qualified";
    }
    Ok(out)
}
