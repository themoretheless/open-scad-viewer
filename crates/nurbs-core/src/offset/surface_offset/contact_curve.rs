//! Whole-world error against one original normal-offset contact branch.
use crate::{
    Result, check, curve::Curve, curve_surface_agreement, distance_bounds::Interval as I,
    offset_contact_pcurve as uv, offset_contact_tangent as tangent, surface::Surface,
};
mod fitting;
pub use crate::offset_contact_pcurve::FitLimits;
pub use fitting::{FitAttempt, FitReport, FitStop, fit};
pub mod chain;
pub mod lift;

#[derive(Clone, Copy)]
pub struct Limits {
    pub cells: usize,
    pub spans: usize,
    pub section_refinements: usize,
    pub band_refinements: usize,
}
pub struct Cell {
    pub interval: [f64; 2],
    pub anchor_error_mm: Option<[f64; 2]>,
    pub error_upper_mm: Option<f64>,
    pub admitted: bool,
}
pub struct Report {
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub band_queries: usize,
    pub section_queries: usize,
    pub world_correspondence_proven: bool,
    pub error_upper_mm: Option<f64>,
    pub reason: &'static str,
}
fn lift(c: &Curve) -> Result<Surface> {
    c.validate()?;
    check(
        !c.periodic && c.control_points[0].len() == 3,
        "World contact curves need three coordinates and a literal nonperiodic clock",
    )?;
    Ok(Surface {
        degree_u: c.degree,
        degree_v: 1,
        knots_u: c.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: c
            .control_points
            .iter()
            .map(|p| vec![p.clone(); 2])
            .collect(),
        weights: c.weights.iter().map(|w| vec![*w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    })
}
/// Enclose candidate(t)-original_contact_root(t) over the COMPLETE requested
/// driving interval, in world millimetres. Fresh implicit world tangents and
/// certified ORIGINAL-tube section anchors bound the residual; sampled points,
/// UV proximity and matching endpoint positions cannot issue this authority.
/// Optional tube contractions preserve the original branch and are counted.
///
/// This proves world coincidence/error only. UV-sheet identity, material trim
/// membership, source/curve regularity, embedding and BRep sewing are separate.
#[allow(clippy::too_many_arguments)]
/// The contact parameter domain: fixed axis, its drive interval, the free interval on the first
/// face and the domain on the second face.
#[derive(Clone, Copy, Debug)]
pub struct ContactDomain {
    pub axis: usize,
    pub drive: [f64; 2],
    pub free: [f64; 2],
    pub second: [[f64; 2]; 2],
}
pub fn certify(
    candidate: &Curve,
    faces: crate::offset::contact_pcurve::OffsetFaces<'_>,
    domain: ContactDomain,
    tolerance_mm: f64,
    limits: Limits,
) -> Result<Report> {
    let crate::offset::contact_pcurve::OffsetFaces { surfaces, distances } = faces;
    let ContactDomain { axis, drive, free, second } = domain;
    check(
        axis < 2
            && drive.iter().all(|x| x.is_finite())
            && drive[0] < drive[1]
            && (drive[1] - drive[0]).is_finite()
            && tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && (1..=100000).contains(&limits.cells)
            && (1..=100000).contains(&limits.spans)
            && limits.section_refinements <= 16
            && limits.band_refinements <= 16,
        "Choose finite world tolerance and bounded contact-curve proof work",
    )?;
    let lifted = lift(candidate)?;
    check(
        candidate.domain() == drive,
        "World curve must retain the original driving clock",
    )?;
    let smooth = tangent::differentiable(&lifted, [drive, [0., 0.]], 1);
    let mut out = Report {
        cells: vec![],
        visited: 0,
        band_queries: 0,
        section_queries: 0,
        world_correspondence_proven: false,
        error_upper_mm: None,
        reason: "world-contact-work-limit",
    };
    let mut pending = vec![drive];
    let mut mismatch = false;
    while let Some(range) = pending.pop() {
        out.visited += 1;
        let target = tangent::certify_refined(
            crate::offset::contact_pcurve::OffsetFaces {
                surfaces,
                distances,
            },
            axis,
            range,
            free,
            second,
            limits.spans,
            limits.band_refinements,
        )?;
        out.band_queries += target.band_queries;
        let mut cell = Cell {
            interval: range,
            anchor_error_mm: None,
            error_upper_mm: None,
            admitted: false,
        };
        if smooth {
            if let Some(world_derivative) = target.report.tangent {
                let curve = super::jacobian_bounds(&lifted, [range, [0., 0.]], 0., limits.spans)?;
                if let Some(derivative) = curve.derivatives {
                    let mid = range[0] * 0.5 + range[1] * 0.5;
                    let (anchor, queries) = uv::section(
                        crate::offset::contact_pcurve::OffsetFaces {
                            surfaces,
                            distances,
                        },
                        axis,
                        mid,
                        free,
                        second,
                        limits.spans,
                        limits.section_refinements,
                    )?;
                    out.section_queries += queries;
                    if let Some(anchor) = anchor {
                        let point =
                            curve_surface_agreement::curve_bounds(candidate, I::point(mid))?;
                        let mut error = [I::point(0.); 3];
                        for k in 0..3 {
                            error[k] =
                                point[k].sub(I::new(anchor.point[k][0], anchor.point[k][1])?)?;
                        }
                        let at_mid = tangent::speed(error)?;
                        cell.anchor_error_mm = Some([at_mid.lo, at_mid.hi]);
                        mismatch |= at_mid.lo > tolerance_mm;
                        let delta = I::new(range[0], range[1])?.sub(I::point(mid))?;
                        for k in 0..3 {
                            let residual = I::new(derivative[0][k][0], derivative[0][k][1])?
                                .sub(I::new(world_derivative[k][0], world_derivative[k][1])?)?;
                            error[k] = error[k].add(residual.mul(delta)?)?;
                        }
                        let upper = tangent::speed(error)?.hi;
                        cell.error_upper_mm = Some(upper);
                        cell.admitted = upper <= tolerance_mm;
                    }
                }
            }
        }
        let mid = range[0] * 0.5 + range[1] * 0.5;
        if !cell.admitted
            && !cell.anchor_error_mm.is_some_and(|x| x[0] > tolerance_mm)
            && smooth
            && mid > range[0]
            && mid < range[1]
            && out.visited + pending.len() + 2 <= limits.cells
        {
            pending.push([mid, range[1]]);
            pending.push([range[0], mid]);
        } else {
            out.cells.push(cell);
        }
    }
    out.world_correspondence_proven = out.cells.iter().all(|c| c.admitted);
    if out.world_correspondence_proven {
        out.error_upper_mm = Some(
            out.cells
                .iter()
                .filter_map(|c| c.error_upper_mm)
                .fold(0., f64::max),
        );
        out.reason = "whole-original-world-contact-proven";
    } else if mismatch {
        out.reason = "world-contact-mismatch";
    } else if !smooth {
        out.reason = "world-curve-continuity-unproven";
    }
    Ok(out)
}
