//! surface self chord algorithm: qualify_rectangle.
use super::*;
/// Excludes all aligned self chords whose endpoints lie in this original
/// parameter rectangle. A local certificate does not cover the full chart.
pub fn qualify_rectangle<'a>(
    s: &'a Surface,
    domain: [[f64; 2]; 2],
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<Report<'a>> {
    check(
        max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Self chord exclusion requires squared sine in [0,1)",
    )?;
    s.validate()?;
    check(
        domain.iter().all(|a| a[0] < a[1]),
        "Self chord rectangle requires positive widths",
    )?;
    normal_alignment::validate_rectangle(s, domain, max_spans)?;
    let mut out = Report {
        certificate: None,
        mean_matrix: None,
        spans: 0,
        complete: false,
        cosine_squared_upper: None,
        reason: "self-chord-unproven",
    };
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    // Integration across a knot needs a continuous chart. Repeated internal
    // knots beyond degree permit jumps; never infer continuity from samples.
    for axis in 0..2 {
        if degrees[axis] == 0 {
            out.reason = "self-chord-continuity-unproven";
            return Ok(out);
        }
        let mut i = degrees[axis] + 1;
        while i < counts[axis] {
            let knot = knots[axis][i];
            let mut end = i + 1;
            while end < knots[axis].len() && knots[axis][end] == knot {
                end += 1
            }
            if knot > domain[axis][0] && knot < domain[axis][1] && end - i > degrees[axis] {
                out.reason = "self-chord-continuity-unproven";
                return Ok(out);
            }
            i = end;
        }
    }
    let mut hull = [[[f64::INFINITY, f64::NEG_INFINITY]; 2]; 3];
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            if knots[0][u] == knots[0][u + 1] || knots[1][v] == knots[1][v + 1] {
                continue;
            }
            let section = [
                [
                    knots[0][u].max(domain[0][0]),
                    knots[0][u + 1].min(domain[0][1]),
                ],
                [
                    knots[1][v].max(domain[1][0]),
                    knots[1][v + 1].min(domain[1][1]),
                ],
            ];
            if section.iter().any(|d| d[0] > d[1]) {
                continue;
            }
            if out.spans == max_spans {
                out.reason = "self-chord-span-limit";
                return Ok(out);
            }
            let j = normal_alignment::jacobian_on(s, [u, v], section)?;
            out.spans += 1;
            for k in 0..3 {
                for a in 0..2 {
                    hull[k][a][0] = hull[k][a][0].min(j[k][a].lo);
                    hull[k][a][1] = hull[k][a][1].max(j[k][a].hi);
                }
            }
        }
    }
    out.complete = true;
    let mut j = [[I::point(0.); 2]; 3];
    for k in 0..3 {
        for a in 0..2 {
            j[k][a] = I::new(hull[k][a][0], hull[k][a][1])?;
        }
    }
    let best = balanced_cosine_upper(j)?;
    out.mean_matrix = Some(MeanMatrix { surface:s, domain, jacobian:j });
    out.cosine_squared_upper = Some(best);
    let threshold = I::point(1.).sub(I::point(max_sine_squared))?;
    if best < threshold.lo {
        out.certificate = Some(Certificate {
            surface: s,
            domain,
            max_sine_squared,
            cosine_squared_upper: best,
        });
        out.reason = "self-chord-normal-excluded";
    }
    Ok(out)
}
