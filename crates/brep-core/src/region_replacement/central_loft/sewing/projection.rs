//! Global disk projection: exact Jordan boundary plus same-sign interior Jacobian.
//! The two collapsed U sides are quotient boundary points, not regular charts.
use super::*;
use nurbs_core::interval_eval::Interval as I;
#[path = "projection/boundary.rs"]
mod boundary;
pub struct ProjectionDiskReport {
    pub proven: bool,
    pub boundary_simple: bool,
    pub jacobian_positive: bool,
    pub pole_quotients: bool,
    pub cells: usize,
    pub exact_work: usize,
    pub pending: usize,
    pub reason: &'static str,
}
pub(super) struct Certificate<'a> {
    _source: &'a Surface,
}
pub(super) struct Inspection<'a> {
    pub report: ProjectionDiskReport,
    pub certificate: Option<Certificate<'a>>,
}
struct Cell {
    span: usize,
    uv: [[f64; 2]; 2],
}
pub(super) fn inspect(
    s: &Surface,
    direction: [i8; 3],
    max_cells: usize,
    max_work: usize,
) -> Result<Inspection<'_>> {
    s.validate()?;
    require(
        (200..=100000).contains(&max_cells)
            && max_work <= 1000000
            && direction.iter().all(|x| x.abs() == 1),
        "Bound two-pole disk projection work",
    )?;
    let (p, q, n, m) = (
        s.degree_u,
        s.degree_v,
        s.control_points.len(),
        s.control_points[0].len(),
    );
    require(
        (3..=16).contains(&p)
            && (1..=16).contains(&q)
            && n <= 256
            && m == q + 1
            && !s.periodic_u
            && !s.periodic_v,
        "Disk projection needs bounded original nonperiodic tensor degrees",
    )?;
    let spans = (p..n)
        .filter(|&i| s.knots_u[i] < s.knots_u[i + 1])
        .collect::<Vec<_>>();
    require(
        s.knots_u[..=p].iter().all(|&u| u == s.knots_u[p])
            && s.knots_u[n..].iter().all(|&u| u == s.knots_u[n])
            && spans
                .iter()
                .skip(1)
                .all(|&i| s.knots_u.iter().filter(|&&u| u == s.knots_u[i]).count() == p)
            && s.knots_v[..=q].iter().all(|&v| v == s.knots_v[q])
            && s.knots_v[m..].iter().all(|&v| v == s.knots_v[m]),
        "Disk projection retains literal C0 Bernstein cells",
    )?;
    let mut out = ProjectionDiskReport {
        proven: false,
        boundary_simple: false,
        jacobian_positive: false,
        pole_quotients: false,
        cells: 0,
        exact_work: 0,
        pending: 1,
        reason: "original-disk-projection-unproven",
    };
    let band = 1. / 64.;
    let mut orientation = None;
    for end in [false, true] {
        let row = if end { n - 1 } else { 0 };
        let pole = &s.control_points[row][0];
        let axes = (0..3)
            .filter(|&axis| {
                (0..3).all(|i| {
                    s.control_points[if end { n - 1 - i } else { i }]
                        .iter()
                        .all(|p| p[axis] == pole[axis])
                })
            })
            .collect::<Vec<_>>();
        let Some(&axis) = axes.first() else {
            out.reason = "disk-pole-flat-factor-unproven";
            return Ok(Inspection {
                report: out,
                certificate: None,
            });
        };
        let r = nurbs_core::surface_shape_operator::inspect_original_flat_pole(
            s,
            end,
            axis,
            band,
            ((max_cells - out.cells) / 4).max(1),
        )?;
        out.cells += r.report.cells;
        let Some(cert) = r.certificate else {
            out.reason = "disk-pole-quotient-unproven";
            return Ok(Inspection {
                report: out,
                certificate: None,
            });
        };
        let dot = cert
            .normal()
            .iter()
            .zip(direction)
            .map(|(x, d)| x * d as f64)
            .sum::<f64>();
        require(
            dot.abs() == 1.,
            "Disk pole normal is not a coordinate direction",
        )?;
        let sign = if dot > 0. { 1 } else { -1 };
        if orientation.is_some_and(|old| old != sign) {
            out.reason = "disk-poles-have-opposite-projected-orientation";
            return Ok(Inspection {
                report: out,
                certificate: None,
            });
        }
        orientation = Some(sign);
        let error = I::point(cert.normal_coefficient())
            .mul(I::point(band))?
            .mul(I::point(band))?;
        if error.hi >= 1. / 16. {
            out.reason = "disk-pole-projected-normal-unproven";
            return Ok(Inspection {
                report: out,
                certificate: None,
            });
        }
    }
    out.pole_quotients = true;
    let (simple, work, pending) = boundary::inspect(s, direction, max_work)?;
    out.exact_work = work;
    out.boundary_simple = simple;
    if !simple {
        out.pending = pending;
        out.reason = "disk-original-projected-boundary-unproven";
        return Ok(Inspection {
            report: out,
            certificate: None,
        });
    }
    let mut patches = Vec::new();
    let mut queue = Vec::new();
    for (index, &span) in spans.iter().enumerate() {
        let original = [s.knots_u[span], s.knots_u[span + 1]];
        let mut uv = [original, [s.knots_v[q], s.knots_v[m]]];
        let width = I::point(original[1]).sub(I::point(original[0]))?;
        if index == 0 {
            uv[0][0] = I::point(original[0])
                .add(width.mul(I::point(band))?)?
                .lo
                .max(original[0]);
        }
        if index + 1 == spans.len() {
            uv[0][1] = I::point(original[1])
                .sub(width.mul(I::point(band))?)?
                .hi
                .min(original[1]);
        }
        let patch = Surface {
            degree_u: p,
            degree_v: q,
            knots_u: [vec![original[0]; p + 1], vec![original[1]; p + 1]].concat(),
            knots_v: s.knots_v.clone(),
            control_points: s.control_points[span - p..=span].to_vec(),
            weights: s.weights[span - p..=span].to_vec(),
            periodic_u: false,
            periodic_v: false,
        };
        patch.validate()?;
        patches.push(patch);
        queue.push(Cell { span: index, uv });
    }
    let sign = orientation.unwrap() as f64;
    let d = direction.map(|x| sign * x as f64);
    while let Some(cell) = queue.pop() {
        if out.cells == max_cells {
            out.pending = queue.len() + 1;
            out.reason = "disk-positive-jacobian-work-exhausted";
            return Ok(Inspection {
                report: out,
                certificate: None,
            });
        }
        out.cells += 1;
        let r = nurbs_core::surface_injectivity::normal_direction_bounds_detailed(
            &patches[cell.span],
            cell.uv,
            d,
            1,
        )?;
        if r.bounds.is_some_and(|b| b[0] > 0.) {
            continue;
        }
        if r.bounds.is_some_and(|b| b[1] < 0.) {
            out.pending = queue.len() + 1;
            out.reason = "disk-projection-orientation-reversal";
            return Ok(Inspection {
                report: out,
                certificate: None,
            });
        }
        let full = &patches[cell.span];
        let widths = [
            (cell.uv[0][1] - cell.uv[0][0]) / (full.knots_u[p + 1] - full.knots_u[p]),
            (cell.uv[1][1] - cell.uv[1][0]) / (full.knots_v[q + 1] - full.knots_v[q]),
        ];
        let axis = usize::from(widths[1] > widths[0]);
        let mid = cell.uv[axis][0] * 0.5 + cell.uv[axis][1] * 0.5;
        if !(cell.uv[axis][0] < mid && mid < cell.uv[axis][1]) {
            out.pending = queue.len() + 1;
            out.reason = "disk-positive-jacobian-precision-exhausted";
            return Ok(Inspection {
                report: out,
                certificate: None,
            });
        }
        for side in 0..2 {
            let mut uv = cell.uv;
            if side == 0 {
                uv[axis][1] = mid;
            } else {
                uv[axis][0] = mid;
            }
            queue.push(Cell {
                span: cell.span,
                uv,
            });
        }
    }
    // The rectangle with its two collapsed vertical sides is a closed disk.
    // Exact C0 columns glue one-sided positive Jacobians with the same tangent.
    // The Jordan boundary has degree one; every interior local homeomorphism
    // has that same orientation. Thus its projected covering has one sheet.
    out.proven = true;
    out.jacobian_positive = true;
    out.pending = 0;
    out.reason = "original-two-pole-disk-projection";
    Ok(Inspection {
        report: out,
        certificate: Some(Certificate { _source: s }),
    })
}
