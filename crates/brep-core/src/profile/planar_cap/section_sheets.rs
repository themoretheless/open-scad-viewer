//! Author open B-rep sheets from certified NURBS section arrangements.
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex};
use num_rational::BigRational as Q;
use num_traits::Zero;
use nurbs_core::{
    Error, Result,
    analysis::exact_affine_hull::{PlanarLiftRefusal, lift_planar_surface},
    curve::Curve,
    curve_surface_agreement::{self, Status},
    editing::exact_surface_derivative::{
        arrange_surface_plane_section_caps, lift_planar_cap_support_bounded,
    },
};

#[derive(Clone, Copy, Debug)]
pub struct SectionSheetOptions {
    pub tolerance: f64,
    pub max_exact_work: u64,
    pub max_verification_cells: usize,
}

#[derive(Clone, Debug)]
pub struct SectionSheetReport {
    pub model: Model,
    /// Input ring indices for each face, outer followed by holes.
    pub face_rings: Vec<Vec<usize>>,
    pub exact_work: u64,
    pub verification_cells: usize,
}

/// Build B-rep sheets for all material regions of the section. The spatial
/// edges keep their original control nets and clocks. Each coedge records
/// its actual traversal direction and the certified trim in the cap chart.
/// Cap supports must have an exact binary64 representation.
pub fn build_section_sheets(
    rings: &[&[Curve]],
    normal: [f64; 3],
    offset: f64,
    options: SectionSheetOptions,
) -> Result<SectionSheetReport> {
    build(rings, normal, offset, options, Support::Tight).map(|report| report.sheet)
}

#[derive(Clone, Debug)]
pub struct BoundedSectionSheetReport {
    pub sheet: SectionSheetReport,
    /// Continuous support errors in the same order as the authored faces.
    pub support_error_upper: Vec<Q>,
}

/// Author section sheets with an explicit approximation limit for the
/// stored planar support. Each support remains affine and every retained
/// spatial edge is reverified against its trim on the stored surface.
pub fn build_section_sheets_with_support_tolerance(
    rings: &[&[Curve]],
    normal: [f64; 3],
    offset: f64,
    options: SectionSheetOptions,
    support_tolerance: f64,
) -> Result<BoundedSectionSheetReport> {
    build(
        rings,
        normal,
        offset,
        options,
        Support::Rounded(support_tolerance),
    )
}

/// Author section sheets with exactly stored planar supports. If the tight
/// Cartesian rectangle cannot be lifted exactly, expand it on a rational
/// lattice anchored at a rim pole. The trim coordinates and edge clocks stay
/// in the original Cartesian chart. The rim must be exactly on the plane.
pub fn build_section_sheets_with_expanded_exact_support(
    rings: &[&[Curve]],
    normal: [f64; 3],
    offset: f64,
    options: SectionSheetOptions,
) -> Result<SectionSheetReport> {
    build(rings, normal, offset, options, Support::ExpandedExact).map(|report| report.sheet)
}

#[derive(Clone, Copy)]
pub(super) enum Support {
    Tight,
    ExpandedExact,
    Rounded(f64),
}

mod expanded_support;

pub(super) fn build(
    rings: &[&[Curve]],
    normal: [f64; 3],
    offset: f64,
    options: SectionSheetOptions,
    support: Support,
) -> Result<BoundedSectionSheetReport> {
    if let Support::Rounded(support_tolerance) = support {
        if !support_tolerance.is_finite() || support_tolerance < 0. {
            return Err(Error::new(
                "BREP_INVALID_PLANAR_CAP",
                "Support tolerance must be finite and nonnegative",
            ));
        }
    }
    if !(1e-10..=1e-2).contains(&options.tolerance)
        || !(1..=100_000).contains(&options.max_verification_cells)
    {
        return Err(Error::new(
            "BREP_INVALID_PLANAR_CAP",
            "Section sheets need a valid model tolerance and agreement budget",
        ));
    }
    let arranged = arrange_surface_plane_section_caps(
        rings,
        normal,
        offset,
        options.tolerance,
        options.max_exact_work,
    )?;
    let arrangement = arranged
        .outcome
        .map_err(|reason| unresolved(format!("Section arrangement: {reason:?}")))?;
    let mut exact_work = arranged.work_used;
    let mut verification_cells = 0;
    let mut model = Model::empty(options.tolerance)?;
    let mut face_rings = Vec::new();
    let mut support_error_upper = Vec::new();
    for region in arrangement.regions {
        if exact_work == options.max_exact_work {
            return Err(unresolved("Section plane lifting budget exhausted"));
        }
        let budget = options.max_exact_work - exact_work;
        let projection = &region.cap.face.projection;
        let (surface, error) = if let Support::Rounded(tolerance) = support {
            let report = lift_planar_cap_support_bounded(projection, tolerance, budget)?;
            exact_work += report.work_used;
            let support = report
                .outcome
                .map_err(|reason| unresolved(format!("Section plane lift: {reason:?}")))?;
            (support.surface, support.error_upper)
        } else {
            if matches!(support, Support::ExpandedExact) && !region.cap.boundary_gap_upper.is_zero()
            {
                return Err(unresolved(
                    "Expanded exact support needs an exactly planar rim",
                ));
            }
            let mut lift = lift_planar_surface(projection, budget)?;
            exact_work += lift.work_used;
            if matches!(support, Support::ExpandedExact)
                && lift.refusal == Some(PlanarLiftRefusal::NotRepresentable)
            {
                let anchor = &region.cap.face.loops[0][0].control_points[0];
                let anchor = [anchor[0], anchor[1]];
                // Prefer smaller extensions while retaining the prior candidates
                // for charts where fine endpoint coordinates cannot be stored.
                for refinement in [Some(3), Some(1), Some(0), None] {
                    const EXPANSION_WORK: u64 = 48;
                    let remaining = options.max_exact_work - exact_work;
                    if remaining <= EXPANSION_WORK {
                        return Err(unresolved("Section plane expansion budget exhausted"));
                    }
                    exact_work += EXPANSION_WORK;
                    let expanded = match expanded_support::expand(projection, anchor, refinement) {
                        Ok(expanded) => expanded,
                        Err(_) if refinement.is_some() => continue,
                        Err(error) => return Err(error),
                    };
                    lift = lift_planar_surface(&expanded, remaining - EXPANSION_WORK)?;
                    exact_work += lift.work_used;
                    if lift.surface.is_some()
                        || lift.refusal == Some(PlanarLiftRefusal::ResourceLimit)
                    {
                        break;
                    }
                }
            }
            let surface = lift
                .surface
                .ok_or_else(|| unresolved(format!("Section plane lift: {:?}", lift.refusal)))?;
            (surface, Q::zero())
        };
        support_error_upper.push(error);
        let mut loop_ids = Vec::with_capacity(region.rings.len());
        for (i, pcurves) in region.cap.face.loops.into_iter().enumerate() {
            let wire = rings[region.rings[i]];
            let reversed = region.cap.reversed[i];
            let count = wire.len();
            let base = model.vertices.len();
            let mut coedges = Vec::with_capacity(count);
            for (j, pcurve) in pcurves.into_iter().enumerate() {
                let source = if reversed { count - 1 - j } else { j };
                let curve = &wire[source];
                if verification_cells == options.max_verification_cells {
                    return Err(unresolved("Section pcurve agreement budget exhausted"));
                }
                let report = curve_surface_agreement::verify(
                    curve,
                    &pcurve,
                    &surface,
                    reversed,
                    options.tolerance,
                    options.max_verification_cells - verification_cells,
                )?;
                verification_cells += report.cells;
                if report.status != Status::WithinTolerance {
                    return Err(unresolved("Section pcurve agreement is unproved"));
                }
                let point = if reversed {
                    &curve.control_points[curve.control_points.len() - 1]
                } else {
                    &curve.control_points[0]
                };
                model.vertices.push(Vertex {
                    point: [point[0], point[1], point[2]],
                });
                let mut vertices = [base + j, base + (j + 1) % count];
                if reversed {
                    vertices.swap(0, 1);
                }
                let edge = model.edges.len();
                model.edges.push(Edge {
                    vertices,
                    curve: curve.clone(),
                    degenerate: false,
                });
                coedges.push(Coedge::new(edge, reversed, pcurve));
            }
            loop_ids.push(model.loops.len());
            model.loops.push(Loop { coedges });
        }
        let face = model.faces.len();
        model.faces.push(Face {
            surface,
            outer: loop_ids[0],
            holes: loop_ids[1..].to_vec(),
        });
        model.shells.push(Shell {
            faces: vec![FaceUse {
                face,
                reversed: false,
            }],
            closed: false,
        });
        face_rings.push(region.rings);
    }
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(BoundedSectionSheetReport {
        sheet: SectionSheetReport {
            model,
            face_rings,
            exact_work,
            verification_cells,
        },
        support_error_upper,
    })
}

fn unresolved(message: impl Into<String>) -> Error {
    Error::new("BREP_CAP_UNRESOLVED", message)
}
