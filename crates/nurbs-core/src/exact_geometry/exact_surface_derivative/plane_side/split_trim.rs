//! Complementary trim contours from a qualified surface-plane graph.
use super::{
    PResult, Refusal, Result, StoredSurfacePlaneSectionPair, Surface,
    SurfacePlaneWorldPolylineOptions, Work,
    store_pair::{store_section, validate_pair},
};
use crate::{curve::Curve, exact_geometry::exact_projective::UvAxis};

#[derive(Clone, Debug)]
pub struct SurfacePlaneTrimSplit {
    pub section: StoredSurfacePlaneSectionPair,
    /// Counterclockwise loop on the negative side of the section trace.
    pub negative: [Curve; 2],
    /// Counterclockwise loop on the positive side of the section trace.
    pub positive: [Curve; 2],
}

#[derive(Clone, Debug)]
pub struct SurfacePlaneTrimSplitReport {
    pub outcome: std::result::Result<SurfacePlaneTrimSplit, Refusal>,
    pub work_used: u64,
}

/// Split one original UV knot cell into two complementary closed trim loops.
/// The common edge has the certified section tolerances. Side labels follow
/// the exact graph orientation. The stored UV trace defines the approximate
/// cut, with each resulting loop oriented counterclockwise.
pub fn split_surface_cell_by_plane(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldPolylineOptions,
    correspondence_tolerance: f64,
) -> Result<SurfacePlaneTrimSplitReport> {
    let domain = validate_pair(surface, normal, offset, options, correspondence_tolerance)?;
    let mut work = Work {
        used: 0,
        limit: options.max_work,
    };
    let outcome = (|| {
        let (section, increasing) = store_section(
            surface,
            normal,
            offset,
            options,
            correspondence_tolerance,
            domain,
            &mut work,
        )?;
        let solved = usize::from(options.solve_axis == UvAxis::V);
        let [lower, upper] = trim_loops(&section.pcurve, domain, solved, &mut work)?;
        let (negative, positive) = if increasing {
            (lower, upper)
        } else {
            (upper, lower)
        };
        Ok(SurfacePlaneTrimSplit {
            section,
            negative,
            positive,
        })
    })();
    Ok(SurfacePlaneTrimSplitReport {
        outcome,
        work_used: work.used,
    })
}

fn trim_loops(
    section: &Curve,
    domain: [[f64; 2]; 2],
    solved: usize,
    work: &mut Work,
) -> PResult<[[Curve; 2]; 2]> {
    let transverse = 1 - solved;
    let [lo, hi] = domain[solved];
    let points = &section.control_points;
    work.charge(points.len() as u64)?;
    if points[0][transverse] != domain[transverse][0]
        || points[points.len() - 1][transverse] != domain[transverse][1]
        || points
            .windows(2)
            .any(|p| p[0][transverse] >= p[1][transverse])
        || !points.iter().any(|p| p[solved] > lo)
        || !points.iter().any(|p| p[solved] < hi)
        || points[1..points.len() - 1]
            .iter()
            .any(|p| p[solved] <= lo || p[solved] >= hi)
    {
        return Err(Refusal::UnprovedIdentity);
    }
    Ok([
        trim_loop(section, solved, lo, solved == 1)?,
        trim_loop(section, solved, hi, solved == 0)?,
    ])
}

fn trim_loop(section: &Curve, solved: usize, side: f64, reverse: bool) -> PResult<[Curve; 2]> {
    let mut trace = section.clone();
    if reverse {
        trace.control_points.reverse();
    }
    let first = trace.control_points[0].clone();
    let last = trace.control_points[trace.control_points.len() - 1].clone();
    Ok([trace, trim_perimeter(first, last, solved, side)?])
}

pub(super) fn trim_perimeter(
    first: Vec<f64>,
    last: Vec<f64>,
    solved: usize,
    side: f64,
) -> PResult<Curve> {
    let mut after = last.clone();
    let mut before = first.clone();
    after[solved] = side;
    before[solved] = side;
    let mut perimeter = vec![last, after, before, first];
    perimeter.dedup();
    Curve::from_polyline(perimeter).map_err(|_| Refusal::InvalidOutput)
}
