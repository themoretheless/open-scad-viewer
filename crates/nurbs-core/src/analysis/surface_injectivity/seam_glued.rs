//! Common contraction on a continuously glued ORIGINAL periodic rectangle.
use super::*;
#[derive(Clone, Debug)]
pub struct GluedRectangleReport {
    pub proven: bool,
    pub projection: Option<[[f64; 3]; 2]>,
    pub contraction_upper: Option<f64>,
    pub spans: usize,
    pub parameter_scale: [f64; 2],
    pub reason: &'static str,
}
fn output(
    report: Report,
    projection: Option<[[f64; 3]; 2]>,
    parameter_scale: [f64; 2],
) -> GluedRectangleReport {
    GluedRectangleReport {
        proven: report.proven,
        projection,
        contraction_upper: report.contraction_upper,
        spans: report.spans,
        parameter_scale,
        reason: report.reason,
    }
}
/// Each axis is one original interval, or two ordered intervals
/// [near-high,source-high], [source-low,near-low]. The latter concatenate via
/// the exact conceptual period translation, never a rounded shifted endpoint.
/// With two wrapped axes every product section contributes to the same hull.
/// Actual source seam identity proves C0 gluing; translations preserve the
/// derivative columns. One common contraction proves the complete rectangle.
pub fn certify_seam_glued_rectangle(
    s: &Surface,
    axes: [Vec<[f64; 2]>; 2],
    max_spans: usize,
    max_depth: usize,
) -> Result<GluedRectangleReport> {
    s.validate()?;
    check(
        (1..=100000).contains(&max_spans) && max_depth <= 6,
        "Invalid original glued rectangle proof allowance",
    )?;
    let natural = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut report = Report {
        proven: false,
        projection: None,
        linear_projection: None,
        projective_projection: None,
        polar_projection: None,
        contraction_upper: None,
        spans: 0,
        reason: "projection-not-proven",
    };
    for axis in 0..2 {
        check(
            (1..=2).contains(&axes[axis].len()),
            "Glued axis needs one or two complete original intervals",
        )?;
        for d in &axes[axis] {
            check(
                d.iter().all(|v| v.is_finite())
                    && natural[axis][0] <= d[0]
                    && d[0] < d[1]
                    && d[1] <= natural[axis][1],
                "Glued interval must be noncollapsed and inside the original source",
            )?;
        }
        if axes[axis].len() == 2 {
            let first = axes[axis][0];
            let second = axes[axis][1];
            check(
                first[1] == natural[axis][1]
                    && second[0] == natural[axis][0]
                    && second[1] < first[0],
                "Glued intervals must meet actual opposite shores and cover less than one period",
            )?;
            if crate::periodic_trim::verify_surface_seam_identity(s, axis)?.is_none() {
                report.reason = "periodic-seam-identity-unproven";
                return Ok(output(report, None, [1., 1.]));
            }
        }
    }
    let us =
        crate::certificates::audit::nonempty_spans(&s.knots_u, s.degree_u, s.control_points.len());
    let vs = crate::certificates::audit::nonempty_spans(
        &s.knots_v,
        s.degree_v,
        s.control_points[0].len(),
    );
    let mut cover = Vec::new();
    for u_part in &axes[0] {
        for v_part in &axes[1] {
            for &u in &us {
                for &v in &vs {
                    let section = [
                        [u_part[0].max(s.knots_u[u]), u_part[1].min(s.knots_u[u + 1])],
                        [v_part[0].max(s.knots_v[v]), v_part[1].min(s.knots_v[v + 1])],
                    ];
                    if section.iter().all(|d| d[0] < d[1]) {
                        cover.push(([u, v], section));
                    }
                }
            }
        }
    }
    check(
        !cover.is_empty(),
        "Original glued rectangle has no complete source cover",
    )?;
    // Positive constants propose a weighted parameter norm. Outward products
    // prove the scaled Jacobian; no rounded shifted source endpoint is created.
    let scale = std::array::from_fn(|axis| {
        let width = axes[axis].iter().map(|d| d[1] - d[0]).sum::<f64>();
        if width.is_finite() && width > 0. {
            width
        } else {
            1.
        }
    });
    let (report, projection, scale) = certify_original_cover_with_projection(
        s,
        cover,
        report,
        max_spans,
        max_depth,
        "seam-glued-original-common-contraction",
        true,
        scale,
    )?;
    Ok(output(report, projection, scale))
}
