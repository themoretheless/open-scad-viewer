//! Exact image reparameterization: preserve every literal Bernstein control net.
use super::*;
pub(super) struct Reclocked {
    pub surface: Surface,
    pub curves: Vec<Curve>,
    pub pcurves: Vec<Curve>,
    pub old_breaks: Vec<f64>,
}
pub(super) fn prepare(c: &CentralLoftCandidate<'_, '_, '_>, s: &Surface) -> Result<Reclocked> {
    let p = s.degree_u;
    let n = s.control_points.len();
    let mut breaks = s.knots_u.clone();
    breaks.dedup();
    require(
        breaks.len() >= 2 && breaks.len() <= 256,
        "Bound exact central clock pieces",
    )?;
    require(
        s.knots_u[..=p].iter().all(|&u| u == breaks[0])
            && s.knots_u[n..].iter().all(|&u| u == *breaks.last().unwrap())
            && breaks[1..breaks.len() - 1]
                .iter()
                .all(|&u| s.knots_u.iter().filter(|&&x| x == u).count() == p),
        "Integer reclocking requires literal original C0 Bernstein spans",
    )?;
    let mut surface = s.clone();
    surface.knots_u = s
        .knots_u
        .iter()
        .map(|u| breaks.iter().position(|v| v == u).unwrap() as f64)
        .collect();
    let mut curves = Vec::new();
    let mut pcurves = Vec::new();
    for pc in &c.pcurves {
        let old = [
            pc.control_points[0][0],
            pc.control_points.last().unwrap()[0],
        ];
        let end = old.map(|u| {
            breaks
                .iter()
                .position(|&v| v == u)
                .ok_or_else(|| refuse("Central seam endpoint is not an original break"))
        });
        let end = [end[0].clone()?, end[1].clone()?];
        let lo = end[0].min(end[1]);
        let hi = end[0].max(end[1]);
        let v = pc.control_points[0][1];
        let row = if v == s.knots_v[s.degree_v] {
            0
        } else {
            s.degree_v
        };
        require(lo < hi, "Empty integer central seam")?;
        let first = lo * p;
        let last = hi * p;
        let mut knots = vec![lo as f64; p + 1];
        for i in lo + 1..hi {
            knots.extend(vec![i as f64; p]);
        }
        knots.extend(vec![hi as f64; p + 1]);
        let curve = Curve {
            degree: p,
            knots,
            control_points: (first..=last)
                .map(|i| surface.control_points[i][row].clone())
                .collect(),
            weights: (first..=last).map(|i| surface.weights[i][row]).collect(),
            periodic: false,
        };
        curve.validate()?;
        curves.push(curve);
        pcurves.push(Curve::from_polyline(vec![
            vec![end[0] as f64, v],
            vec![end[1] as f64, v],
        ])?);
    }
    surface.validate()?;
    require(
        surface.control_points == s.control_points
            && surface.weights == s.weights
            && surface.knots_v == s.knots_v,
        "Reclocking changed original central geometry coefficients",
    )?;
    Ok(Reclocked {
        surface,
        curves,
        pcurves,
        old_breaks: breaks,
    })
}
/// Original plane geometry is unchanged; its exact parameter is a signed world coordinate.
pub(super) fn plane(s: &Surface) -> Result<Surface> {
    let frame = planar::frame(s)?;
    let mut result = s.clone();
    for k in 0..2 {
        let (axis, o, d) = frame[k];
        let end = if k == 0 {
            s.control_points[1][0][axis]
        } else {
            s.control_points[0][1][axis]
        };
        let sign = d.signum();
        let knots = vec![sign * o, sign * o, sign * end, sign * end];
        if k == 0 {
            result.knots_u = knots;
        } else {
            result.knots_v = knots;
        }
    }
    result.validate()?;
    Ok(result)
}
