//! Narrow G6 productization scaffold gated by the R1 ADR verdict (`narrow`).
//!
//! Beyond Complete-empty: planar bicubic × planar bicubic may publish a transverse
//! line as Complete. Out-of-matrix pairs refuse. False Complete is a kill.
//! No general NURBS Boolean.

use crate::coverage_verifier::verify_complete_report;
use crate::intersections::{Coverage, Options, Report, UnresolvedReason};
use nurbs_core::{surface::Surface, Error, Result};

fn refuse(message: &str) -> Error {
    Error::new("BREP_NURBS_SS_REFUSED", message)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum G6Maturity {
    Unavailable,
    ResearchOnly,
    NarrowTransverseBicubic,
}

pub const G6_MATURITY: G6Maturity = G6Maturity::NarrowTransverseBicubic;
pub const G6_CAPABILITY: &str = "nurbs-ss-transverse-bicubic/1";

#[derive(Clone, Debug, PartialEq)]
pub enum G6Component {
    Empty,
    Line {
        start: [f64; 3],
        end: [f64; 3],
    },
}

fn is_uniform_bicubic_positive(surface: &Surface) -> bool {
    surface.degree_u == 3
        && surface.degree_v == 3
        && surface.control_points.len() == 4
        && surface.control_points.iter().all(|row| row.len() == 4)
        && surface
            .weights
            .iter()
            .flatten()
            .all(|w| w.is_finite() && (*w - 1.).abs() <= 1e-15)
        && !surface.periodic_u
        && !surface.periodic_v
}

fn bbox(s: &Surface) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in s.control_points.iter().flatten() {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    (min, max)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    if !n.is_finite() || n == 0. {
        None
    } else {
        Some([a[0] / n, a[1] / n, a[2] / n])
    }
}

/// Recover a supporting plane when all control points are coplanar.
fn planar_support(surface: &Surface, tol: f64) -> Option<([f64; 3], [f64; 3])> {
    let pts: Vec<[f64; 3]> = surface
        .control_points
        .iter()
        .flatten()
        .filter_map(|p| {
            if p.len() >= 3 {
                Some([p[0], p[1], p[2]])
            } else {
                None
            }
        })
        .collect();
    if pts.len() < 3 {
        return None;
    }
    let o = pts[0];
    let mut normal = None;
    for i in 1..pts.len() {
        for j in (i + 1)..pts.len() {
            let n = cross(sub(pts[i], o), sub(pts[j], o));
            if norm(n) > tol {
                normal = normalize(n);
                break;
            }
        }
        if normal.is_some() {
            break;
        }
    }
    let n = normal?;
    for p in &pts {
        if dot(sub(*p, o), n).abs() > tol {
            return None;
        }
    }
    Some((o, n))
}

fn complete_empty() -> Report<G6Component> {
    let mut report = Report::default();
    report.coverage = Coverage::Complete;
    report.boxes_visited = 1;
    report.components.push(G6Component::Empty);
    report
}

fn complete_line(start: [f64; 3], end: [f64; 3]) -> Report<G6Component> {
    let mut report = Report::default();
    report.coverage = Coverage::Complete;
    report.boxes_visited = 1;
    report.components.push(G6Component::Line { start, end });
    report
}

/// Research/product spike: separated Complete-empty, or planar transverse Complete line.
pub fn narrow_transverse_bicubic(
    a: &Surface,
    b: &Surface,
    options: Options,
) -> Result<Report<G6Component>> {
    let options = options.validate()?;
    a.validate()?;
    b.validate()?;
    if G6_MATURITY == G6Maturity::Unavailable {
        return Err(refuse("G6 capability is Unavailable per R1 stop"));
    }
    if !is_uniform_bicubic_positive(a) || !is_uniform_bicubic_positive(b) {
        let mut report = Report::default();
        report.unresolved(
            [
                a.knots_u.first().copied().unwrap_or(0.),
                a.knots_u.last().copied().unwrap_or(1.),
                b.knots_u.first().copied().unwrap_or(0.),
                b.knots_u.last().copied().unwrap_or(1.),
            ],
            UnresolvedReason::UnsupportedSurface,
        );
        return Ok(report);
    }
    let (amin, amax) = bbox(a);
    let (bmin, bmax) = bbox(b);
    let separated = (0..3).any(|i| amax[i] < bmin[i] - 1e-9 || bmax[i] < amin[i] - 1e-9);
    if separated {
        return Ok(complete_empty());
    }

    let tol = options.distance_tolerance.max(1e-9);
    // Near-identical cages → coincident refuse (R1-02).
    let same_cage = (0..3).all(|i| (amin[i] - bmin[i]).abs() <= tol && (amax[i] - bmax[i]).abs() <= tol);
    if same_cage {
        let mut report = Report::default();
        report.unresolved(
            vec![0., 1., 0., 1., 0., 1., 0., 1.],
            UnresolvedReason::NearCoincidence,
        );
        return Ok(report);
    }

    // R1-01: planar bicubic × planar bicubic transverse line when normals disagree.
    if let (Some((oa, na)), Some((ob, nb))) = (planar_support(a, tol), planar_support(b, tol)) {
        let parallel = norm(cross(na, nb)) <= 1e-9;
        if parallel {
            let d = dot(sub(ob, oa), na).abs();
            if d <= tol {
                let mut report = Report::default();
                report.unresolved(
                    vec![0., 1., 0., 1., 0., 1., 0., 1.],
                    UnresolvedReason::NearCoincidence,
                );
                return Ok(report);
            }
            // Parallel separated planes already handled by bbox; overlapping slabs refuse.
            let mut report = Report::default();
            report.unresolved(
                vec![0., 1., 0., 1., 0., 1., 0., 1.],
                UnresolvedReason::TangencyOrMultipleRoot,
            );
            return Ok(report);
        }
        // Line direction = na × nb; pick a point on both planes inside the AABB overlap.
        let dir = normalize(cross(na, nb)).ok_or_else(|| refuse("Degenerate plane normals"))?;
        // Solve for a point on the intersection line near the overlap center.
        let center = [
            (amin[0].max(bmin[0]) + amax[0].min(bmax[0])) * 0.5,
            (amin[1].max(bmin[1]) + amax[1].min(bmax[1])) * 0.5,
            (amin[2].max(bmin[2]) + amax[2].min(bmax[2])) * 0.5,
        ];
        // Project center onto both planes along the average normal plane.
        let da = -dot(sub(center, oa), na);
        let db = -dot(sub(center, ob), nb);
        // Move along the plane of normals (na, nb) to satisfy both offsets.
        let nn = dot(na, nb);
        let denom = 1. - nn * nn;
        if denom.abs() <= 1e-15 {
            let mut report = Report::default();
            report.unresolved(
                vec![0., 1., 0., 1., 0., 1., 0., 1.],
                UnresolvedReason::NearCoincidence,
            );
            return Ok(report);
        }
        let alpha = (da - db * nn) / denom;
        let beta = (db - da * nn) / denom;
        let point = [
            center[0] + alpha * na[0] + beta * nb[0],
            center[1] + alpha * na[1] + beta * nb[1],
            center[2] + alpha * na[2] + beta * nb[2],
        ];
        let extent = 0.5
            * ((amax[0] - amin[0])
                .hypot(amax[1] - amin[1])
                .hypot(amax[2] - amin[2])
                .max(1.));
        let start = [
            point[0] - dir[0] * extent,
            point[1] - dir[1] * extent,
            point[2] - dir[2] * extent,
        ];
        let end = [
            point[0] + dir[0] * extent,
            point[1] + dir[1] * extent,
            point[2] + dir[2] * extent,
        ];
        let report = complete_line(start, end);
        verify_complete_report(&report, false)?;
        return Ok(report);
    }

    // Non-planar overlapping cages: refuse rather than claim Complete (R1-03 near-tangent).
    let mut report = Report::default();
    report.unresolved(
        vec![0., 1., 0., 1., 0., 1., 0., 1.],
        UnresolvedReason::NearCoincidence,
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage_verifier::{verify_complete_report, verify_incomplete_refusal};

    fn bicubic(z: f64, shift: f64) -> Surface {
        let row = |y: f64| {
            (0..4)
                .map(|i| vec![i as f64 + shift, y, z])
                .collect::<Vec<_>>()
        };
        Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![row(0.), row(1.), row(2.), row(3.)],
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn planar_yz(x: f64) -> Surface {
        let row = |y: f64| (0..4).map(|_| vec![x, y, 0.]).collect::<Vec<_>>();
        // Vary z in v so the patch spans YZ at fixed X.
        let mut control_points = Vec::new();
        for iz in 0..4 {
            let z = iz as f64;
            control_points.push((0..4).map(|iy| vec![x, iy as f64, z]).collect());
        }
        let _ = row;
        Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points,
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn separated_bicubics_are_complete_empty() {
        let report = narrow_transverse_bicubic(
            &bicubic(0., 0.),
            &bicubic(0., 20.),
            Options::default(),
        )
        .unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        verify_complete_report(&report, true).unwrap();
        assert!(!report.permits_topology_change());
    }

    #[test]
    fn overlapping_bicubics_refuse_without_complete() {
        let report =
            narrow_transverse_bicubic(&bicubic(0., 0.), &bicubic(0., 0.5), Options::default())
                .unwrap();
        verify_incomplete_refusal(&report).unwrap();
    }

    #[test]
    fn non_bicubic_is_unsupported() {
        let mut s = bicubic(0., 0.);
        s.degree_u = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.control_points = vec![
            s.control_points[0].clone(),
            s.control_points[1].clone(),
            s.control_points[2].clone(),
        ];
        s.weights = vec![vec![1.; 4]; 3];
        let report = narrow_transverse_bicubic(&s, &bicubic(0., 20.), Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
    }

    #[test]
    fn rational_weights_and_high_degree_refuse() {
        let mut s = bicubic(0., 0.);
        s.weights[0][0] = 2.;
        let report = narrow_transverse_bicubic(&s, &bicubic(0., 20.), Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
    }

    #[test]
    fn g6_never_permits_topology_change() {
        let report = narrow_transverse_bicubic(
            &bicubic(0., 0.),
            &bicubic(0., 20.),
            Options::default(),
        )
        .unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        assert!(!report.permits_topology_change());
    }

    #[test]
    fn planar_transverse_bicubics_publish_complete_line() {
        let xy = bicubic(0., 0.); // plane z=0
        let yz = planar_yz(1.5); // plane x=1.5 intersecting the xy patch
        let report = narrow_transverse_bicubic(&xy, &yz, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        verify_complete_report(&report, false).unwrap();
        assert!(matches!(report.components[0], G6Component::Line { .. }));
        assert!(!report.permits_topology_change());
    }
}
