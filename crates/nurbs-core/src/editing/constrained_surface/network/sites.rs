//! Collocation/refinement proposals in each candidate pcurve's forward clock.
use super::*;
fn normalized_knots(c: &Curve, reverse: bool) -> Vec<f64> {
    let d = c.domain();
    c.knots
        .iter()
        .copied()
        .filter(|&x| d[0] < x && x < d[1])
        .map(|x| {
            let t = (x - d[0]) / (d[1] - d[0]);
            if reverse { 1. - t } else { t }
        })
        .collect()
}
fn sorted(s: &mut Vec<f64>) {
    s.sort_by(f64::total_cmp);
    s.dedup_by(|a, b| (*a - *b).abs() < 64. * f64::EPSILON);
}
pub(super) fn initial(g: &OwnedGuide) -> Result<Vec<f64>> {
    let mut s = vec![0., 1.];
    s.extend(normalized_knots(&g.spatial, g.reverse));
    s.extend(normalized_knots(&g.parameters, false));
    if let Some(t) = &g.tangent {
        s.extend(normalized_knots(&t.parameters, t.reverse ^ g.reverse));
    }
    sorted(&mut s);
    let mid = s
        .windows(2)
        .map(|w| w[0] / 2. + w[1] / 2.)
        .collect::<Vec<_>>();
    s.extend(mid);
    sorted(&mut s);
    Ok(s)
}
pub(super) fn parameter(c: &Curve, t: f64) -> f64 {
    let d = c.domain();
    (d[0] + (d[1] - d[0]) * t).clamp(d[0], d[1])
}
pub(super) fn constraints(
    g: &OwnedGuide,
    sites: &[f64],
    linearization: Option<&Surface>,
    out: &mut Vec<Constraint>,
) -> Result<()> {
    for &t in sites {
        let q = if g.reverse { 1. - t } else { t };
        let p = g.spatial.evaluate(parameter(&g.spatial, q))?.point;
        let uv = g.parameters.evaluate(parameter(&g.parameters, t))?.point;
        let uv = [uv[0], uv[1]];
        out.push(Constraint::Point {
            uv,
            point: [p[0], p[1], p[2]],
        });
        if let Some(s) = &g.tangent {
            let r = if s.reverse { 1. - q } else { q };
            let pc = s.parameters.evaluate(parameter(&s.parameters, r))?.point;
            let normal = s
                .surface
                .evaluate(pc[0], pc[1])?
                .unit_normal()
                .ok_or_else(|| {
                    error("Original tangent-support normal unresolved at a proposal site")
                })?;
            out.push(Constraint::TangentPlane { uv, normal });
            if let Some(linearization) = linearization.filter(|_| g.curvature) {
                curvature::equations(linearization, uv, &s.surface, [pc[0], pc[1]], out)?;
            }
        }
    }
    Ok(())
}
pub(super) fn refine(s: &mut Vec<f64>, witness: Option<f64>, max: usize) -> bool {
    let old = s.len();
    let mut proposals = Vec::new();
    if let Some(t) = witness.filter(|t| t.is_finite() && *t >= 0. && *t <= 1.) {
        proposals.push(t);
    }
    proposals.extend(s.windows(2).map(|w| w[0] / 2. + w[1] / 2.));
    for t in proposals {
        if s.len() == max {
            break;
        }
        if s.iter().all(|x| (*x - t).abs() >= 64. * f64::EPSILON) {
            s.push(t);
        }
    }
    sorted(s);
    s.len() > old
}
