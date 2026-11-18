//! Original parametric Ck jets on a body with owned physical C1/C2 pole disks.
//! Singular source parameters remain singular; physical rank is retained by
//! the opaque stratified regularity owner, not inferred from these jet checks.
use super::super::{
    Carrier,
    stratified::{PoleEvidence, StratifiedBody},
};
use crate::source_seam_tangency::C2PoleAtlas;
use nurbs_core::{Error, Result, analysis::surface_continuity};
pub struct StratifiedSmoothFaces<
    'r,
    'p,
    'b,
    's,
    E: Carrier,
    P: PoleEvidence<'b, 's, E> = C2PoleAtlas<'b, 's, E>,
> {
    regular: &'r StratifiedBody<'p, 'b, 's, E, P>,
    faces: Vec<surface_continuity::Certificate<'s>>,
    order: usize,
}
impl<'r, 'p, 'b, 's, E: Carrier, P: PoleEvidence<'b, 's, E>>
    StratifiedSmoothFaces<'r, 'p, 'b, 's, E, P>
{
    pub fn regular(&self) -> &'r StratifiedBody<'p, 'b, 's, E, P> {
        self.regular
    }
    pub fn body(&self) -> &'s crate::source_volume::Body<E> {
        self.regular.poles().boundary().body()
    }
    pub fn faces(&self) -> &[surface_continuity::Certificate<'s>] {
        &self.faces
    }
    pub fn order(&self) -> usize {
        self.order
    }
}
pub struct StratifiedReport<
    'r,
    'p,
    'b,
    's,
    E: Carrier,
    P: PoleEvidence<'b, 's, E> = C2PoleAtlas<'b, 's, E>,
> {
    pub certificate: Option<StratifiedSmoothFaces<'r, 'p, 'b, 's, E, P>>,
    pub cells: usize,
    pub exact_work: u64,
    pub uncertain_face: Option<usize>,
}
/// Fresh whole-knot checks for EVERY original source face under one cell/exact
/// ledger. The same original body retains its embedded material geometry,
/// complete ordinary/polar rank coverage and physical pole-neighborhood proof.
pub fn qualify<'r, 'p, 'b, 's, E: Carrier, P: PoleEvidence<'b, 's, E>>(
    regular: &'r StratifiedBody<'p, 'b, 's, E, P>,
    order: usize,
    max_cells: usize,
    max_work: u64,
) -> Result<StratifiedReport<'r, 'p, 'b, 's, E, P>> {
    if !matches!(order, 1 | 2) || max_cells > 1000000 || max_work > 100000000 {
        return Err(Error::new(
            "BREP_SOURCE_FACE_CONTINUITY",
            "Choose original stratified C1/C2 and bounded work",
        ));
    }
    let body: &'s crate::source_volume::Body<E> = regular.poles().boundary().body();
    let mut out = StratifiedReport {
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
        out.certificate = Some(StratifiedSmoothFaces {
            regular,
            faces,
            order,
        });
        out.uncertain_face = None;
    }
    Ok(out)
}
