//! Ck interiors of every regular original material face. Ordinary edge/vertex
//! classes and physical pole neighborhoods remain separate; C0 is not upgraded.
use super::{Carrier, RegularBody};
use nurbs_core::{Error, Result, analysis::surface_continuity};
pub struct SmoothFaces<'r, 's, E: Carrier> {
    regular: &'r RegularBody<'s, E>,
    faces: Vec<surface_continuity::Certificate<'s>>,
    order: usize,
}
impl<'r, 's, E: Carrier> SmoothFaces<'r, 's, E> {
    pub fn regular(&self) -> &'r RegularBody<'s, E> {
        self.regular
    }
    pub fn faces(&self) -> &[surface_continuity::Certificate<'s>] {
        &self.faces
    }
    pub fn order(&self) -> usize {
        self.order
    }
}
pub struct Report<'r, 's, E: Carrier> {
    pub certificate: Option<SmoothFaces<'r, 's, E>>,
    pub cells: usize,
    pub exact_work: u64,
    pub uncertain_face: Option<usize>,
}
/// The private rank owner retains the original embedded material body. All its
/// faces, including filled caps, receive fresh whole-knot one-sided jet checks
/// with ONE additional aggregate cell/exact ledger. No partial collection,
/// caller continuity flag or sampled derivative can construct this owner.
pub fn qualify<'r, 's, E: Carrier>(
    regular: &'r RegularBody<'s, E>,
    order: usize,
    max_cells: usize,
    max_work: u64,
) -> Result<Report<'r, 's, E>> {
    if !matches!(order, 1 | 2) || max_cells > 1000000 || max_work > 100000000 {
        return Err(Error::new(
            "BREP_SOURCE_FACE_CONTINUITY",
            "Choose original face C1/C2 and bounded work",
        ));
    }
    let body: &'s crate::source_volume::Body<E> = regular.body();
    let mut out = Report {
        certificate: None,
        cells: 0,
        exact_work: 0,
        uncertain_face: None,
    };
    let mut faces = Vec::new();
    for (index, wires) in body.geometry().shell().faces().iter().enumerate() {
        out.uncertain_face = Some(index);
        let r = surface_continuity::qualify(
            wires[0].edges()[0].surface(),
            order,
            max_cells - out.cells,
            max_work - out.exact_work,
        )?;
        out.cells += r.cells;
        out.exact_work += r.exact_work;
        let Some(proof) = r.certificate else {
            return Ok(out);
        };
        faces.push(proof);
    }
    if !faces.is_empty() {
        out.certificate = Some(SmoothFaces {
            regular,
            faces,
            order,
        });
        out.uncertain_face = None;
    }
    Ok(out)
}

pub mod stratified;
pub use stratified::{StratifiedReport, StratifiedSmoothFaces, qualify as qualify_stratified};
