//! Exact affine-plane bounds over the original tensor knot cells.
use super::{
    PResult, Poly, Q, Refusal, Result, Surface, Work, Zero, check, extraction, polynomial, storage,
};
use num_traits::Signed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfacePlaneSide {
    Coplanar,
    Positive,
    Negative,
    Nonnegative,
    Nonpositive,
    /// Exact samples certify a point on each strict side.
    Straddles,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneCell {
    pub domain: [[f64; 2]; 2],
    /// Exact bounds on the unnormalized plane residual.
    pub residual: [Q; 2],
    /// Residuals at the original cell's four one-sided corners.
    pub corners: [Q; 4],
    pub side: SurfacePlaneSide,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneAudit {
    pub cells: std::result::Result<Vec<SurfacePlaneCell>, Refusal>,
    pub work_used: u64,
}

/// Bound the entire rational surface against dot(normal, XYZ) = offset.
/// On each original knot cell the homogeneous residual and weight share a
/// Bernstein basis. Positive weights make their quotient a convex combination
/// of the coefficient ratios. Exact corner signs certify straddling.
pub fn audit_surface_plane_side(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    max_work: u64,
) -> Result<SurfacePlaneAudit> {
    validate(surface, normal, offset, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let cells = audit(surface, normal, offset, &mut work);
    Ok(SurfacePlaneAudit {
        cells,
        work_used: work.used,
    })
}

fn validate(surface: &Surface, normal: [f64; 3], offset: f64, max_work: u64) -> Result<()> {
    surface.validate()?;
    check(
        normal.iter().all(|x| x.is_finite())
            && normal.iter().any(|&x| x != 0.)
            && offset.is_finite()
            && (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Surface plane audit needs a finite nonzero plane and bounded arithmetic work",
    )
}

struct PlaneNet {
    size: [usize; 2],
    numerator: Vec<Q>,
    denominator: Vec<Q>,
}

fn audit(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    work: &mut Work,
) -> PResult<Vec<SurfacePlaneCell>> {
    let mut cells = Vec::new();
    visit_nets(surface, normal, offset, work, |domain, net, work| {
        cells.push(net.audit(domain, work)?);
        Ok(())
    })?;
    Ok(cells)
}

fn visit_nets(
    surface: &Surface,
    [x, y, z]: [f64; 3],
    offset: f64,
    work: &mut Work,
    mut visit: impl FnMut([[f64; 2]; 2], PlaneNet, &mut Work) -> PResult<()>,
) -> PResult<()> {
    let plane = [
        work.lift(x)?,
        work.lift(y)?,
        work.lift(z)?,
        -work.lift(offset)?,
    ];
    for u in surface.degree_u..surface.control_points.len() {
        let u_domain = [surface.knots_u[u], surface.knots_u[u + 1]];
        if u_domain[0] == u_domain[1] {
            continue;
        }
        for v in surface.degree_v..surface.control_points[0].len() {
            let v_domain = [surface.knots_v[v], surface.knots_v[v + 1]];
            if v_domain[0] == v_domain[1] {
                continue;
            }
            let domain = [u_domain, v_domain];
            visit(
                domain,
                extract_net(surface, [u, v], domain, &plane, work)?,
                work,
            )?;
        }
    }
    Ok(())
}

fn extract_net(
    surface: &Surface,
    [u, v]: [usize; 2],
    domain: [[f64; 2]; 2],
    plane: &[Q; 4],
    work: &mut Work,
) -> PResult<PlaneNet> {
    let homogeneous = homogeneous_cell(surface, [u, v], domain, work)?;
    plane_net(surface, &homogeneous, plane, work)
}

fn homogeneous_cell(
    surface: &Surface,
    [u, v]: [usize; 2],
    domain: [[f64; 2]; 2],
    work: &mut Work,
) -> PResult<Vec<Poly>> {
    let span_width = |[lo, hi]: [f64; 2], work: &mut Work| -> PResult<Q> {
        let lo = work.lift(lo)?;
        let hi = work.lift(hi)?;
        Ok(work.op(&hi, &lo, |a, b| a - b)?)
    };
    let widths = [span_width(domain[0], work)?, span_width(domain[1], work)?];
    extraction::cell(surface, u, v, domain, &widths, work)
}

fn plane_net(
    surface: &Surface,
    homogeneous: &[Poly],
    plane: &[Q; 4],
    work: &mut Work,
) -> PResult<PlaneNet> {
    let mut residual_poly = Poly::new();
    for (channel, coefficient) in homogeneous.iter().zip(plane) {
        if coefficient.is_zero() {
            continue;
        }
        for (&key, term) in channel {
            let value = work.op(term, coefficient, |a, b| a * b)?;
            polynomial::accumulate(&mut residual_poly, key, &value, work)?;
        }
    }
    let pu = surface.degree_u;
    let pv = surface.degree_v;
    let numerator = storage::bernstein(&residual_poly, pu, pv, work)?;
    let denominator = storage::bernstein(&homogeneous[3], pu, pv, work)?;
    Ok(PlaneNet {
        size: [pu + 1, pv + 1],
        numerator,
        denominator,
    })
}

impl PlaneNet {
    fn audit(&self, domain: [[f64; 2]; 2], work: &mut Work) -> PResult<SurfacePlaneCell> {
        let mut ratios = Vec::with_capacity(self.numerator.len());
        for (n, d) in self.numerator.iter().zip(&self.denominator) {
            if !d.is_positive() {
                return Err(Refusal::UnprovedDenominator);
            }
            ratios.push(work.op(n, d, |a, b| a / b)?);
        }
        let residual = [
            ratios.iter().min().ok_or(Refusal::InvalidOutput)?.clone(),
            ratios.iter().max().ok_or(Refusal::InvalidOutput)?.clone(),
        ];
        let [nu, nv] = self.size;
        let corners = [0, nv - 1, (nu - 1) * nv, ratios.len() - 1].map(|i| ratios[i].clone());
        let side = classify(&residual, &corners);
        Ok(SurfacePlaneCell {
            domain,
            residual,
            corners,
            side,
        })
    }
}

mod refinement;
pub use refinement::{RefinedSurfacePlaneAudit, SurfacePlaneRegion, refine_surface_plane_side};

fn classify([lo, hi]: &[Q; 2], corners: &[Q; 4]) -> SurfacePlaneSide {
    if lo.is_positive() {
        SurfacePlaneSide::Positive
    } else if hi.is_negative() {
        SurfacePlaneSide::Negative
    } else if lo.is_zero() && hi.is_zero() {
        SurfacePlaneSide::Coplanar
    } else if lo.is_zero() {
        SurfacePlaneSide::Nonnegative
    } else if hi.is_zero() {
        SurfacePlaneSide::Nonpositive
    } else if corners.iter().any(Signed::is_negative) && corners.iter().any(Signed::is_positive) {
        SurfacePlaneSide::Straddles
    } else {
        SurfacePlaneSide::Unresolved
    }
}

mod graph;
pub use graph::{
    PlaneGraphPosition, SurfacePlaneGraph, SurfacePlaneGraphAudit, SurfacePlaneGraphCell,
    SurfacePlaneGraphStatus, audit_surface_plane_graphs,
};

mod samples;
pub use samples::{
    SurfacePlaneGraphSample, SurfacePlaneGraphSampleOptions, SurfacePlaneGraphSampleReport,
    SurfacePlaneGraphSamples, sample_surface_plane_graph,
};

mod polyline;
pub use polyline::{
    SurfacePlanePolyline, SurfacePlanePolylineOptions, SurfacePlanePolylineReport,
    approximate_surface_plane_graph,
};

mod world_polyline;
pub use world_polyline::{
    SurfacePlaneWorldPolyline, SurfacePlaneWorldPolylineOptions, SurfacePlaneWorldPolylineReport,
    approximate_surface_plane_graph_xyz,
};

mod store_curve;
pub use store_curve::{
    StoredSurfacePlaneCurve, StoredSurfacePlaneCurveReport, store_surface_plane_section_curve,
};

mod store_pair;
pub use store_pair::{
    StoredSurfacePlaneSectionPair, StoredSurfacePlaneSectionPairReport,
    store_surface_plane_section_pair,
};

mod split_trim;
pub use split_trim::{
    SurfacePlaneTrimSplit, SurfacePlaneTrimSplitReport, split_surface_cell_by_plane,
};

mod stitch;
pub use stitch::{
    SurfacePlaneGraphJunction, SurfacePlaneGraphStrip, SurfacePlaneGraphStripCell,
    SurfacePlaneGraphStripOptions, SurfacePlaneGraphStripReport, stitch_surface_plane_graph_strip,
};

mod world_strip;
pub use world_strip::{
    SurfacePlaneWorldStrip, SurfacePlaneWorldStripOptions, SurfacePlaneWorldStripReport,
    approximate_surface_plane_strip_xyz,
};

mod periodic_strip;
pub use periodic_strip::close_periodic_surface_plane_strip_xyz;

mod store_periodic;
pub use store_periodic::{
    StoredPeriodicSurfacePlaneSection, StoredPeriodicSurfacePlaneSectionReport,
    store_periodic_surface_plane_section,
};

mod split_periodic;
pub use split_periodic::{
    PeriodicSurfacePlaneTrimSplit, PeriodicSurfacePlaneTrimSplitReport,
    split_periodic_surface_strip_by_plane,
};

mod cap;
pub use cap::{
    BoundedPlanarCapSupport, BoundedPlanarCapSupportReport, PlanarSectionCap,
    PlanarSectionCapArrangement, PlanarSectionCapArrangementReport, PlanarSectionCapRegion,
    PlanarSectionCapReport, PlanarSectionCapWithHoles, PlanarSectionCapWithHolesReport,
    arrange_surface_plane_section_caps, cap_surface_plane_section,
    cap_surface_plane_section_with_holes, lift_planar_cap_support_bounded,
};
