//! surface injectivity algorithm: certify_contraction.
use super::*;
/// Failure of this sufficient condition is unresolved, never evidence of an
/// intersection. This certifies the whole natural chart, hence any trimmed
/// subset, but does not compare it with other faces or certify a whole solid.
pub fn certify_contraction(s: &Surface, max_spans: usize) -> Result<Report> {
    s.validate()?;
    check(
        max_spans > 0 && max_spans <= 100_000,
        "Injectivity span budget must be in 1..100000",
    )?;
    let mut report = Report {
        proven: false,
        projection: None,
        linear_projection: None,
        projective_projection: None,
        polar_projection: None,
        polar_p_matrix: None,
        contraction_upper: None,
        spans: 0,
        reason: "projection-not-proven",
    };
    if s.periodic_u || s.periodic_v {
        report.reason = "periodic-domain";
        return Ok(report);
    }
    // A clamped boundary with identical Euclidean controls maps a whole
    // parameter edge to one point, even with different positive weights.
    // Strict rectangular-chart injectivity cannot certify that pole. A future
    // proof on the quotient domain must handle it explicitly.
    let collapsed_u = [0, s.control_points.len()-1].into_iter().any(|i| {
        let knots = if i == 0 { &s.knots_u[..=s.degree_u] }
            else { &s.knots_u[s.control_points.len()..] };
        knots.iter().all(|k| *k == knots[0]) &&
            s.control_points[i].iter().all(|p| *p == s.control_points[i][0])
    });
    let collapsed_v = [0, s.control_points[0].len()-1].into_iter().any(|j| {
        let knots = if j == 0 { &s.knots_v[..=s.degree_v] }
            else { &s.knots_v[s.control_points[0].len()..] };
        knots.iter().all(|k| *k == knots[0]) &&
            s.control_points.iter().all(|row| row[j] == s.control_points[0][j])
    });
    if collapsed_u || collapsed_v {
        report.reason = "collapsed-boundary-requires-quotient-proof";
        return Ok(report);
    }
    let mut j = [[I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 2]; 3];
    let us = crate::certificates::audit::nonempty_spans(&s.knots_u, s.degree_u, s.control_points.len());
    let vs = crate::certificates::audit::nonempty_spans(&s.knots_v, s.degree_v, s.control_points[0].len());
    for &u in &us {
        for &v in &vs {
            if report.spans == max_spans {
                report.reason = "work-limit";
                return Ok(report);
            }
            let next = jacobian(s, [u, v])?;
            report.spans += 1;
            for k in 0..3 {
                for axis in 0..2 {
                    j[k][axis] = hull([j[k][axis], next[k][axis]].into_iter());
                }
            }
        }
    }
    for axes in [[0, 1], [0, 2], [1, 2]] {
        if let Some(q) = contraction([j[axes[0]], j[axes[1]]])? {
            if q < 1. {
                report.proven = true;
                report.projection = Some(axes);
                report.contraction_upper = Some(q);
                report.reason = "global-projection-contraction";
                return Ok(report);
            }
        }
    }
    Ok(report)
}
