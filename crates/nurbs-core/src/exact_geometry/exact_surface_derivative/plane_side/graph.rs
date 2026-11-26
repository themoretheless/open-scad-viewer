//! Graph certificates for plane intersections on rational tensor knot cells.
use super::{PResult, PlaneNet, Q, Refusal, Result, Surface, Work, validate, visit_nets};
use crate::exact_geometry::exact_projective::UvAxis;
use num_traits::{Signed, Zero};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaneGraphPosition {
    Interior,
    /// The graph may meet an edge in the solved direction.
    Closed,
    LowerEdge,
    UpperEdge,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfacePlaneGraph {
    /// Solve for this axis as a function of the other parameter.
    pub solve_axis: UvAxis,
    pub position: PlaneGraphPosition,
    /// Bounds on the derivative of the homogeneous plane numerator with
    /// respect to the normalized solved coordinate of the original cell.
    pub derivative: [Q; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfacePlaneGraphStatus {
    Separated,
    Coplanar,
    /// Each certifiable solved axis, in U/V order.
    Graphs(Box<[Option<SurfacePlaneGraph>; 2]>),
    Unresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneGraphCell {
    pub domain: [[f64; 2]; 2],
    pub status: SurfacePlaneGraphStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneGraphAudit {
    pub cells: std::result::Result<Vec<SurfacePlaneGraphCell>, Refusal>,
    pub work_used: u64,
}

/// Prove a unique plane root for every value of a cell's transverse parameter.
/// A strict derivative sign certifies uniqueness. Opposite weak signs on
/// entire bounding edges certify existence, including roots on those edges.
pub fn audit_surface_plane_graphs(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    max_work: u64,
) -> Result<SurfacePlaneGraphAudit> {
    validate(surface, normal, offset, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut cells = Vec::new();
    let result = visit_nets(surface, normal, offset, &mut work, |domain, net, work| {
        let status = certify(&net, work)?;
        cells.push(SurfacePlaneGraphCell { domain, status });
        Ok(())
    });
    Ok(SurfacePlaneGraphAudit {
        cells: result.map(|()| cells),
        work_used: work.used,
    })
}

fn certify(net: &PlaneNet, work: &mut Work) -> PResult<SurfacePlaneGraphStatus> {
    work.charge(net.numerator.len() as u64)?;
    if net.numerator.iter().all(Q::is_zero) {
        return Ok(SurfacePlaneGraphStatus::Coplanar);
    }
    if net.numerator.iter().all(Q::is_positive) || net.numerator.iter().all(Q::is_negative) {
        return Ok(SurfacePlaneGraphStatus::Separated);
    }
    let graphs = [graph(net, UvAxis::U, work)?, graph(net, UvAxis::V, work)?];
    Ok(if graphs.iter().any(Option::is_some) {
        SurfacePlaneGraphStatus::Graphs(Box::new(graphs))
    } else {
        SurfacePlaneGraphStatus::Unresolved
    })
}

pub(super) fn graph(
    net: &PlaneNet,
    axis: UvAxis,
    work: &mut Work,
) -> PResult<Option<SurfacePlaneGraph>> {
    let [nu, nv] = net.size;
    let (along, across, stride) = match axis {
        UvAxis::U => (nu, nv, nv),
        UvAxis::V => (nv, nu, 1),
    };
    let row = |j| match axis {
        UvAxis::U => j,
        UvAxis::V => j * nv,
    };
    let endpoint =
        |along_index| (0..across).map(move |j| &net.numerator[row(j) + along_index * stride]);
    let increasing =
        endpoint(0).all(|q| !q.is_positive()) && endpoint(along - 1).all(|q| !q.is_negative());
    let decreasing =
        endpoint(0).all(|q| !q.is_negative()) && endpoint(along - 1).all(|q| !q.is_positive());
    if !increasing && !decreasing {
        return Ok(None);
    }
    let mut bounds: Option<[Q; 2]> = None;
    for j in 0..across {
        for i in 0..along - 1 {
            let k = row(j) + i * stride;
            let difference =
                work.op(&net.numerator[k + stride], &net.numerator[k], |a, b| a - b)?;
            bounds = Some(match bounds {
                Some([lo, hi]) => [lo.min(difference.clone()), hi.max(difference)],
                None => [difference.clone(), difference],
            });
        }
    }
    let [lo, hi] = bounds.ok_or(Refusal::InvalidOutput)?;
    if !(increasing && lo.is_positive() || decreasing && hi.is_negative()) {
        return Ok(None);
    }
    let position = if endpoint(0).all(Q::is_zero) {
        PlaneGraphPosition::LowerEdge
    } else if endpoint(along - 1).all(Q::is_zero) {
        PlaneGraphPosition::UpperEdge
    } else if endpoint(0).all(|q| !q.is_zero()) && endpoint(along - 1).all(|q| !q.is_zero()) {
        PlaneGraphPosition::Interior
    } else {
        PlaneGraphPosition::Closed
    };
    let degree = Q::from_integer((along - 1).into());
    let derivative = [
        work.op(&lo, &degree, |a, b| a * b)?,
        work.op(&hi, &degree, |a, b| a * b)?,
    ];
    Ok(Some(SurfacePlaneGraph {
        solve_axis: axis,
        position,
        derivative,
    }))
}
