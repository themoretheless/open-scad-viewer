//! Original curve/surface roots qualified against a real authored face region.
//! Outside trim boxes can exclude otherwise unresolved surface-product cells.
use crate::{Model, TopoId, face_domain::FaceDomain};
use nurbs_core::{
    Error, Result,
    intersection::certified_curve_surface::{self, Root, Unresolved},
    trim_domain::{Classification, Location},
};
#[derive(Debug)]
pub struct FaceRoot {
    pub root: Root,
    pub location: Location,
    pub classification: Option<Classification>,
    /// Increasing source-curve traversal against the oriented face use.
    pub face_crossing_sign: i8,
}
#[derive(Debug)]
pub struct FaceUnknown {
    pub cell: Unresolved,
    pub location: Location,
    pub classification: Option<Classification>,
}
#[derive(Debug)]
pub struct Report {
    pub source_edge_identity: TopoId,
    /// False for a standalone curve-definition query, which grants no ownership.
    pub source_is_owned: bool,
    pub support_face_identity: TopoId,
    pub support_face_reversed: bool,
    pub roots: Vec<FaceRoot>,
    pub unresolved: Vec<FaceUnknown>,
    /// Indices into roots, retaining the original parameter ordering.
    pub eligible: Vec<usize>,
    pub native_complete: bool,
    /// Every possible intersection with the actual trimmed face is classified.
    /// This is not a Boolean material-side or solid-validity certificate.
    pub complete: bool,
    pub cells: usize,
    pub trim_cells: usize,
    pub trim_pairs: usize,
    pub root_cells: usize,
    pub boundary_cells: usize,
    pub membership_cells: usize,
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_FACE_CROSSING_INPUT", message)
}
fn unproven(message: &str) -> Error {
    Error::new("BREP_FACE_CROSSING_UNPROVEN", message)
}
fn membership(
    domain: &FaceDomain,
    uv: [[f64; 2]; 2],
    cells: &mut usize,
    max_cells: usize,
) -> Result<Option<Classification>> {
    if *cells == max_cells {
        return Ok(None);
    }
    let proof = domain.classify(uv, (max_cells - *cells).min(100000))?;
    *cells += proof.cells;
    Ok(Some(proof))
}
/// Inspect immutable source/cutter geometry. The selected support face must have
/// one authored shell use and a proven simple outer contour with disjoint holes.
/// Each root is classified using its complete outward UV enclosure. A midpoint,
/// tolerance band or sampled point never authorizes membership. Unresolved root
/// cells are harmless only when their complete UV boxes are proved Outside.
/// Root ordering is not merged by world position, and all exclusions are retained.
pub fn inspect(
    source: &Model,
    edge_id: usize,
    support: &Model,
    face_id: usize,
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Report> {
    source.validate()?;
    if !source.persistent_naming_complete() {
        return Err(invalid("Source identities are incomplete"));
    }
    let edge = source
        .edges
        .get(edge_id)
        .ok_or_else(|| invalid("Source edge is out of range"))?;
    inspect_definition(
        &edge.curve,
        source.1.edges[edge_id],
        true,
        support,
        face_id,
        tolerance_uv,
        max_cells,
    )
}
/// Classify a standalone original curve against an actual authored trimmed face.
/// Its definition key identifies query geometry only; no existing edge ownership
/// or shared root identity is inferred. All root and trim proofs are recomputed.
pub fn inspect_curve(
    curve: &nurbs_core::curve::Curve,
    support: &Model,
    face_id: usize,
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Report> {
    curve.validate()?;
    let signature = crate::rational_identity::curve_signature(curve);
    let identity = TopoId::derive(
        crate::TopoKind::Edge,
        "query",
        "unowned-curve",
        "definition",
        signature.as_bytes(),
    );
    inspect_definition(
        curve,
        identity,
        false,
        support,
        face_id,
        tolerance_uv,
        max_cells,
    )
}
fn inspect_definition(
    curve: &nurbs_core::curve::Curve,
    source_edge_identity: TopoId,
    source_is_owned: bool,
    support: &Model,
    face_id: usize,
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Report> {
    support.validate()?;
    if !support.persistent_naming_complete()
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=1000000).contains(&max_cells)
    {
        return Err(invalid(
            "Face crossing needs complete support identities, positive UV tolerance and bounded work",
        ));
    }
    let face = support
        .faces
        .get(face_id)
        .ok_or_else(|| invalid("Support face is out of range"))?;
    let owners: Vec<_> = support
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .filter(|u| u.face == face_id)
        .collect();
    if owners.len() != 1 {
        return Err(invalid(
            "Support face needs one unambiguous authored shell use",
        ));
    }
    let loops: Vec<Vec<_>> = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|w| {
            support.loops[w]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect();
    if max_cells < 4 {
        return Err(unproven(
            "Insufficient shared work for trim validity and root membership",
        ));
    }
    let stage = (max_cells / 4).min(100000);
    let trim = nurbs_core::trim_region_audit::inspect(&loops, tolerance_uv, 10000, stage, stage)?;
    if trim.valid != Some(true) {
        return Err(unproven(
            "Support trim region validity is unproven or invalid",
        ));
    }
    let trim_cells = trim.cells + trim.domain_cells;
    if trim_cells >= max_cells {
        return Err(unproven("Trim audit exhausted the shared work budget"));
    }
    let mut boundary_cells = 0;
    for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        for (coedge, c) in support.loops[wire].coedges.iter().enumerate() {
            let spent = trim_cells + boundary_cells;
            if spent == max_cells {
                return Err(unproven("Support boundary audit exhausted shared work"));
            }
            let proof = nurbs_core::curve_surface_agreement::verify(
                &support.edges[c.edge].curve,
                &c.pcurve,
                &face.surface,
                c.reversed,
                support.tolerance_mm,
                (max_cells - spent).min(100000),
            )?;
            boundary_cells += proof.cells;
            if proof.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
                return Err(unproven(&format!(
                    "Support boundary lift is unproven: face={face_id} wire={wire} coedge={coedge}"
                )));
            }
        }
    }
    if trim_cells + boundary_cells == max_cells {
        return Err(unproven("Support boundary audit left no root work"));
    }
    let native = certified_curve_surface::intersections(
        curve,
        &face.surface,
        (max_cells - trim_cells - boundary_cells).min(100000),
    )?;
    let mut out = Report {
        source_edge_identity,
        source_is_owned,
        support_face_identity: support.1.faces[face_id],
        support_face_reversed: owners[0].reversed,
        roots: vec![],
        unresolved: vec![],
        eligible: vec![],
        native_complete: native.complete,
        complete: true,
        cells: trim_cells + boundary_cells + native.cells,
        trim_cells,
        trim_pairs: trim.pairs,
        root_cells: native.cells,
        boundary_cells,
        membership_cells: 0,
    };
    let domain = FaceDomain::new(support, face_id, tolerance_uv)?;
    for root in native.roots {
        let classification = membership(
            &domain,
            [root.parameter_box[1], root.parameter_box[2]],
            &mut out.cells,
            max_cells,
        )?;
        let location = classification
            .as_ref()
            .map_or(Location::Unresolved, |r| r.location);
        out.complete &= location != Location::Unresolved;
        if location == Location::Inside {
            out.eligible.push(out.roots.len());
        }
        let face_crossing_sign =
            root.crossing_sign * if out.support_face_reversed { -1 } else { 1 };
        out.roots.push(FaceRoot {
            root,
            location,
            classification,
            face_crossing_sign,
        });
    }
    for cell in native.unresolved {
        let classification = membership(
            &domain,
            [cell.parameter_box[1], cell.parameter_box[2]],
            &mut out.cells,
            max_cells,
        )?;
        let location = classification
            .as_ref()
            .map_or(Location::Unresolved, |r| r.location);
        out.complete &= location == Location::Outside;
        out.unresolved.push(FaceUnknown {
            cell,
            location,
            classification,
        });
    }
    out.membership_cells = out.cells - out.trim_cells - out.root_cells - out.boundary_cells;
    Ok(out)
}
