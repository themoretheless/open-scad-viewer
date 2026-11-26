//! Subdivision of homogeneous plane residual nets on exact dyadic UV cells.
use super::{
    PResult, PlaneNet, Refusal, Result, Surface, SurfacePlaneCell, SurfacePlaneSide, Work, check,
    validate, visit_nets,
};
use crate::exact_geometry::exact_surface_derivative::umbilic_subdivision::split;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneRegion {
    /// Absolute source knot interval in each axis.
    pub original_domain: [[f64; 2]; 2],
    /// Analysis of the region. Its domain is in the original cell's normalized UV.
    pub cell: SurfacePlaneCell,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefinedSurfacePlaneAudit {
    pub regions: std::result::Result<Vec<SurfacePlaneRegion>, Refusal>,
    pub work_used: u64,
}

/// Refine possible plane contacts by splitting the exact homogeneous
/// residual and weight together. The leaves cover every original cell.
/// Strictly separated and coplanar leaves stop early. At the depth limit,
/// any remaining weak, straddling or unresolved leaf retains its bounds.
pub fn refine_surface_plane_side(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    max_depth: usize,
    max_work: u64,
) -> Result<RefinedSurfacePlaneAudit> {
    validate(surface, normal, offset, max_work)?;
    check(
        max_depth <= 32,
        "Plane subdivision depth must be at most 32",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut regions = Vec::new();
    let result = visit_nets(surface, normal, offset, &mut work, |domain, net, work| {
        refine(domain, net, max_depth, &mut regions, work)
    });
    Ok(RefinedSurfacePlaneAudit {
        regions: result.map(|()| regions),
        work_used: work.used,
    })
}

fn refine(
    original_domain: [[f64; 2]; 2],
    net: PlaneNet,
    max_depth: usize,
    regions: &mut Vec<SurfacePlaneRegion>,
    work: &mut Work,
) -> PResult<()> {
    let mut pending = vec![([[0., 1.]; 2], net, 0)];
    while let Some((unit_domain, net, depth)) = pending.pop() {
        let cell = net.audit(unit_domain, work)?;
        if depth == max_depth
            || matches!(
                cell.side,
                SurfacePlaneSide::Positive
                    | SurfacePlaneSide::Negative
                    | SurfacePlaneSide::Coplanar
            )
        {
            regions.push(SurfacePlaneRegion {
                original_domain,
                cell,
            });
            continue;
        }
        let axis = depth % 2;
        let [low_n, high_n] = split(&net.numerator, net.size, axis, work)?;
        let [low_d, high_d] = split(&net.denominator, net.size, axis, work)?;
        let [lo, hi] = unit_domain[axis];
        let middle = (lo + hi) * 0.5;
        let mut low_domain = unit_domain;
        let mut high_domain = unit_domain;
        low_domain[axis][1] = middle;
        high_domain[axis][0] = middle;
        pending.push((
            high_domain,
            PlaneNet {
                size: net.size,
                numerator: high_n,
                denominator: high_d,
            },
            depth + 1,
        ));
        pending.push((
            low_domain,
            PlaneNet {
                size: net.size,
                numerator: low_n,
                denominator: low_d,
            },
            depth + 1,
        ));
    }
    Ok(())
}
