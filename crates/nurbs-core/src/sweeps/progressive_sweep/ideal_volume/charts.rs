//! Outward original carrier jets and one fixed monotonicity projection.
use super::*;
pub(super) fn decode(v: Box3) -> Result<V3> {
    Ok([
        I::new(v[0][0], v[0][1])?,
        I::new(v[1][0], v[1][1])?,
        I::new(v[2][0], v[2][1])?,
    ])
}
pub(super) fn encode(v: V3) -> Box3 {
    v.map(|i| [i.lo, i.hi])
}
fn c0(degree: usize, knots: &[f64], end: usize) -> bool {
    knots
        .iter()
        .filter(|&&u| knots[degree] < u && u < knots[end])
        .all(|u| knots.iter().filter(|v| *v == u).count() <= degree)
}
pub(super) fn curve_c0(c: &Curve) -> bool {
    !c.periodic && c0(c.degree, &c.knots, c.control_points.len())
}
pub(super) fn surface_c0(s: &Surface) -> bool {
    !s.periodic_u
        && !s.periodic_v
        && c0(s.degree_u, &s.knots_u, s.control_points.len())
        && c0(s.degree_v, &s.knots_v, s.control_points[0].len())
}
/// Original rational surface value and first jets in NORMALIZED UV coordinates.
/// Every intersected ORIGINAL knot cell is enclosed and charged, including
/// one-sided derivatives at knots. No extracted floating net becomes a source.
pub(super) fn cap_jets(
    s: &Surface,
    uv: [[f64; 2]; 2],
    spent: &mut usize,
    max: usize,
) -> Result<Option<[V3; 3]>> {
    let d = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let widths = [
        I::point(d[0][1]).sub(I::point(d[0][0]))?,
        I::point(d[1][1]).sub(I::point(d[1][0]))?,
    ];
    let mut query = [[0.; 2]; 2];
    for k in 0..2 {
        let q = I::point(d[k][0])
            .add(widths[k].mul(I::new(uv[k][0], uv[k][1])?)?)?
            .intersect(d[k][0], d[k][1])?;
        query[k] = [q.lo, q.hi];
    }
    let mut total: Option<[V3; 3]> = None;
    for u in s.degree_u..s.control_points.len() {
        let du = [s.knots_u[u], s.knots_u[u + 1]];
        if du[0] >= du[1] || du[1] < query[0][0] || du[0] > query[0][1] {
            continue;
        }
        for v in s.degree_v..s.control_points[0].len() {
            let dv = [s.knots_v[v], s.knots_v[v + 1]];
            if dv[0] >= dv[1] || dv[1] < query[1][0] || dv[0] > query[1][1] {
                continue;
            }
            if *spent == max {
                return Ok(None);
            }
            *spent += 1;
            let region = [
                [query[0][0].max(du[0]), query[0][1].min(du[1])],
                [query[1][0].max(dv[0]), query[1][1].min(dv[1])],
            ];
            let j = crate::surface_measure::jets::calculate_partial_stable(
                s,
                [u, v],
                region,
                [None; 2],
            )?;
            // The stable jet owner subtracts this exact stored origin. Its
            // derivatives are with respect to the local ORIGINAL knot cell.
            let origin = &s.control_points[u - s.degree_u][v - s.degree_v];
            let part = [
                iv::add(
                    j[0][0],
                    [
                        I::point(origin[0]),
                        I::point(origin[1]),
                        I::point(origin[2]),
                    ],
                )?,
                iv::scale(
                    j[1][0],
                    widths[0].div(I::point(du[1]).sub(I::point(du[0]))?)?,
                )?,
                iv::scale(
                    j[0][1],
                    widths[1].div(I::point(dv[1]).sub(I::point(dv[0]))?)?,
                )?,
            ];
            total = Some(if let Some(mut old) = total {
                for a in 0..3 {
                    for b in 0..3 {
                        old[a][b] = I::new(
                            old[a][b].lo.min(part[a][b].lo),
                            old[a][b].hi.max(part[a][b].hi),
                        )?;
                    }
                }
                old
            } else {
                part
            });
        }
    }
    Ok(total)
}
pub(super) fn local(v: V3, basis: [V3; 3]) -> Result<V3> {
    Ok([
        iv::dot_tight(v, basis[0])?,
        iv::dot_tight(v, basis[1])?,
        iv::dot_tight(v, basis[2])?,
    ])
}
/// Strict diagonal dominance is a sufficient SPD enclosure, not a numerical
/// eigenvalue test. The lower bound uses outward arithmetic throughout.
pub(super) fn margin(columns: [V3; 3], projection: [[f64; 3]; 3]) -> Result<f64> {
    let mut a = [[I::point(0.); 3]; 3];
    for i in 0..3 {
        for (j, col) in columns.iter().enumerate() {
            a[i][j] = iv::dot_tight(projection[i].map(I::point), *col)?;
        }
    }
    let mut lower = f64::INFINITY;
    for i in 0..3 {
        let mut row = a[i][i];
        for j in 0..3 {
            if i != j {
                let s = a[i][j].add(a[j][i])?.div(I::point(2.))?;
                row = row.sub(I::point(s.lo.abs().max(s.hi.abs())))?;
            }
        }
        lower = lower.min(row.lo);
    }
    Ok(lower)
}
pub(super) fn original_jets(
    s: &Sweep<'_>,
    t: [f64; 2],
    max: usize,
    offset: Option<&Curve>,
    transport: Option<&OriginalRmfTransportReport>,
) -> Result<Option<frame::Report>> {
    if let Some(transport) = transport {
        if transport.closing_angle.is_some() {
            return super::super::rmf_transport::closed_frame_jets(
                transport, s.path, s.twist, t, max,
            )
            .map(Some);
        }
        return super::super::rmf_transport::frame_jets(transport, s.path, s.twist, t, max)
            .map(Some);
    }
    if let Some((a, n)) = s.frame_laws {
        return frame::certify_twisted(a, n, s.twist, t, max).map(Some);
    }
    if let Some(g) = s.orientation_guide {
        return if let Some(offset) = offset {
            frame::certify_path_guide_from_offset(s.path, offset, s.twist, t, max).map(Some)
        } else {
            frame::certify_path_guide(s.path, g, s.twist, t, max).map(Some)
        };
    }
    Ok(Some(match s.options.orientation {
        Orientation::Fixed => frame::certify_fixed_path(s.path, s.options.normal, s.twist, t, max)?,
        Orientation::FixedNormal => {
            frame::certify_fixed_normal_path(s.path, s.options.normal, s.twist, t, max)?
        }
        Orientation::Frenet => frame::certify_frenet_path(s.path, s.twist, t, max)?,
        Orientation::RotationMinimizing if super::authored_error::original_line(s.path) => {
            frame::certify_fixed_path(s.path, s.options.normal, s.twist, t, max)?
        }
        Orientation::RotationMinimizing
            if super::authored_error::original_planar_rmf(s.path, s.options.normal) =>
        {
            frame::certify_fixed_normal_path(s.path, s.options.normal, s.twist, t, max)?
        }
        _ => return Ok(None),
    }))
}

// Error-free TwoDiff: a zero residual proves the stored subtraction exact.
// Coordinate ceilings keep every intermediate finite. An inexact difference
// falls back to independent original jets, never to a rounded proxy curve.
pub(super) fn exact_offset(
    path: &Curve,
    guide: &Curve,
    spent: &mut usize,
    max: usize,
) -> Result<Option<Curve>> {
    if path.degree != guide.degree
        || path.knots != guide.knots
        || path.weights != guide.weights
        || path.periodic != guide.periodic
    {
        return Ok(None);
    }
    let charge = 3 * path.control_points.len();
    if charge > max - *spent {
        return Ok(None);
    }
    *spent += charge;
    let mut out = path.clone();
    for ((q, p), g) in out
        .control_points
        .iter_mut()
        .zip(&path.control_points)
        .zip(&guide.control_points)
    {
        for k in 0..3 {
            let x = g[k] - p[k];
            let bv = g[k] - x;
            let av = x + bv;
            let br = bv - p[k];
            let ar = g[k] - av;
            if !x.is_finite() || x.abs() > 1e9 || ar + br != 0. {
                return Ok(None);
            }
            q[k] = x;
        }
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_difference_keeps_exact_coefficients_or_refuses_the_shortcut() {
        let p = constant_vector_law([1e9, 0., 0.]).unwrap();
        let g = constant_vector_law([1e-9, 0., 0.]).unwrap();
        let mut work = 0;
        assert!(exact_offset(&p, &g, &mut work, 100).unwrap().is_none());
        assert_eq!(work, 6);
        let g = constant_vector_law([1e9, 1., 0.]).unwrap();
        let mut work = 0;
        let exact = exact_offset(&p, &g, &mut work, 100).unwrap().unwrap();
        assert!(exact.control_points.iter().all(|q| q == &[0., 1., 0.]));
        let mut work = 0;
        assert!(exact_offset(&p, &g, &mut work, 5).unwrap().is_none());
        assert_eq!(work, 0);
    }
}

/// Children cover the entire parent including the shared binary64 boundary.
pub(super) fn split_axis(b: Box3, axis: usize) -> Option<[Box3; 2]> {
    let mid = crate::distance_bounds::midpoint(b[axis][0], b[axis][1]);
    if !(b[axis][0] < mid && mid < b[axis][1]) {
        return None;
    }
    let mut left = b;
    let mut right = b;
    left[axis][1] = mid;
    right[axis][0] = mid;
    Some([left, right])
}
