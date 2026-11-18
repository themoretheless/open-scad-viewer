//! Geometric smoothness of the interiors of every regular original material face. Ordinary edge/vertex
//! classes and physical pole neighborhoods remain separate; C0 is not upgraded.
use super::{Carrier, RegularBody};
use nurbs_core::{
    Error, Result,
    analysis::{
        exact_affine_hull::{SurfaceAffineHull, classify_surface_affine_hull},
        surface_continuity,
    },
};
pub struct SmoothFaces<'r, 's, E: Carrier> {
    regular: &'r RegularBody<'s, E>,
    faces: Vec<FaceContinuity<'s>>,
    order: usize,
}
impl<'r, 's, E: Carrier> SmoothFaces<'r, 's, E> {
    pub fn regular(&self) -> &'r RegularBody<'s, E> {
        self.regular
    }
    pub fn faces(&self) -> &[FaceContinuity<'s>] {
        &self.faces
    }
    pub fn order(&self) -> usize {
        self.order
    }
}
/// Whole-chart evidence retaining the immutable original rational carrier.
pub struct FaceContinuity<'s> {
    source: &'s crate::Surface,
    order: usize,
    geometric_joins: usize,
}
impl<'s> FaceContinuity<'s> {
    pub fn source(&self) -> &'s crate::Surface {
        self.source
    }
    pub fn order(&self) -> usize {
        self.order
    }
    pub fn geometric_joins(&self) -> usize {
        self.geometric_joins
    }
}

// Rank and one-to-one material coordinates are owned by the caller. A
// continuous embedded chart in an exact plane has open planar neighborhoods.
fn planar_joins(source: &crate::Surface, work: u64) -> Result<(Option<usize>, u64)> {
    let census = (3 * source.control_points.len() * source.control_points[0].len()
        + source.knots_u.len()
        + source.knots_v.len()) as u64;
    if work < census {
        return Ok((None, 0));
    }
    let origin = &source.control_points[0][0];
    let axis_aligned = (0..3).any(|axis| {
        source
            .control_points
            .iter()
            .flatten()
            .all(|p| p[axis] == origin[axis])
    });
    let mut used = census;
    if !axis_aligned {
        let remaining = (work - used).min(cad_predicates::MAX_WORK);
        if remaining == 0 {
            return Ok((None, used));
        }
        let hull = classify_surface_affine_hull(source, remaining)?;
        used += hull.work_used;
        if !matches!(hull.hull, SurfaceAffineHull::Plane(_)) {
            return Ok((None, used));
        }
    }
    let mut joins = 0;
    for (knots, degree, count) in [
        (
            &source.knots_u,
            source.degree_u,
            source.control_points.len(),
        ),
        (
            &source.knots_v,
            source.degree_v,
            source.control_points[0].len(),
        ),
    ] {
        for block in knots.chunk_by(|a, b| a == b) {
            if knots[degree] < block[0] && block[0] < knots[count] {
                if block.len() > degree {
                    return Ok((None, used));
                }
                joins += 1;
            }
        }
    }
    Ok((Some(joins), used))
}

fn qualify_face(
    source: &crate::Surface,
    order: usize,
    cells: usize,
    work: u64,
) -> Result<(Option<FaceContinuity<'_>>, usize, u64)> {
    let raw = surface_continuity::qualify(source, order, cells, work)?;
    if raw.certificate.is_some() && !source.periodic_u && !source.periodic_v {
        return Ok((
            Some(FaceContinuity {
                source,
                order,
                geometric_joins: 0,
            }),
            raw.cells,
            raw.exact_work,
        ));
    }
    let mut used = raw.exact_work;
    if raw.outcome == cad_predicates::BezierIdentity::Different
        && !source.periodic_u
        && !source.periodic_v
        && cells > raw.cells
    {
        let (joins, plane_work) = planar_joins(source, work - used)?;
        used += plane_work;
        if let Some(geometric_joins) = joins {
            return Ok((
                Some(FaceContinuity {
                    source,
                    order,
                    geometric_joins,
                }),
                raw.cells + 1,
                used,
            ));
        }
    }
    let (joins, extra_cells, extra_work) =
        crate::sweep_smoothness::complete::interiors::geometric_joins(
            source,
            order,
            cells - raw.cells,
            work - used,
        )?;
    Ok((
        joins.map(|geometric_joins| FaceContinuity {
            source,
            order,
            geometric_joins,
        }),
        raw.cells + extra_cells,
        used + extra_work,
    ))
}

pub struct Report<'r, 's, E: Carrier> {
    pub certificate: Option<SmoothFaces<'r, 's, E>>,
    pub cells: usize,
    pub exact_work: u64,
    pub uncertain_face: Option<usize>,
}
/// The private rank owner retains the original embedded material body. All its
/// faces, including filled caps, receive fresh whole-knot one-sided jet checks.
/// Exact geometric joins allow a change of parameter speed across a knot or
/// periodic wrap. The original chart is retained in the resulting evidence.
/// One additional cell/exact ledger covers all faces. No partial collection,
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
            "Choose original face G1/G2 and bounded work",
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
        let (proof, cells, work) = qualify_face(
            wires[0].edges()[0].surface(),
            order,
            max_cells - out.cells,
            max_work - out.exact_work,
        )?;
        out.cells += cells;
        out.exact_work += work;
        let Some(proof) = proof else {
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
