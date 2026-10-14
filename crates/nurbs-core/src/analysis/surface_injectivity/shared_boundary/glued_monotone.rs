//! One fixed linear projection on two complete parameter half-rectangles.
//! The source Bezier poles and weights remain unchanged. Midpoint derivatives
//! propose a projection and transverse parameter lengths only; existing interval
//! symmetric-Jacobian certificates establish the actual whole-domain result.
use super::{Result, Surface};

fn virtual_face(source: &Surface, end: bool, left: bool) -> Result<Option<Surface>> {
    let u = [
        source.knots_u[source.degree_u],
        source.knots_u[source.degree_u + 1],
    ];
    let v = [
        source.knots_v[source.degree_v],
        source.knots_v[source.degree_v + 1],
    ];
    let sample = source.evaluate(u[0] * 0.5 + u[1] * 0.5, v[0] * 0.5 + v[1] * 0.5)?;
    let Some((du, dv)) = sample.first_derivatives() else {
        return Ok(None);
    };
    let length = (du[0].hypot(du[1]).hypot(du[2]) * (u[1] - u[0]))
        .max(dv[0].hypot(dv[1]).hypot(dv[2]) * (v[1] - v[0]));
    if !length.is_finite() || length <= 0. {
        return Ok(None);
    }
    let mut out = source.clone();
    if left != end {
        out.control_points.reverse();
        out.weights.reverse();
    }
    let domain = if left { [-length, 0.] } else { [0., length] };
    out.knots_u = [
        vec![domain[0]; source.degree_u + 1],
        vec![domain[1]; source.degree_u + 1],
    ]
    .concat();
    out.validate()?;
    Ok(Some(out))
}
fn proposal(a: &Surface, b: &Surface) -> Result<Option<[[i8; 3]; 2]>> {
    let mut columns = [[0.; 3]; 2];
    for s in [a, b] {
        let sample = s.evaluate(
            (s.knots_u[s.degree_u] + s.knots_u[s.degree_u + 1]) * 0.5,
            (s.knots_v[s.degree_v] + s.knots_v[s.degree_v + 1]) * 0.5,
        )?;
        let Some((du, dv)) = sample.first_derivatives() else {
            return Ok(None);
        };
        for k in 0..3 {
            columns[0][k] += du[k] * 0.5;
            columns[1][k] += dv[k] * 0.5;
        }
    }
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| a[k] * b[k]).sum::<f64>();
    let g = [
        [dot(columns[0], columns[0]), dot(columns[0], columns[1])],
        [dot(columns[1], columns[0]), dot(columns[1], columns[1])],
    ];
    let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
    if !det.is_finite() || det <= 0. {
        return Ok(None);
    }
    let inverse = [
        [g[1][1] / det, -g[0][1] / det],
        [-g[1][0] / det, g[0][0] / det],
    ];
    let y = std::array::from_fn::<_, 2, _>(|i| {
        std::array::from_fn::<_, 3, _>(|k| {
            inverse[i][0] * columns[0][k] + inverse[i][1] * columns[1][k]
        })
    });
    if y.iter().flatten().any(|v| !v.is_finite()) {
        return Ok(None);
    }
    let scale = y.iter().flatten().map(|v| v.abs()).fold(0., f64::max);
    if scale <= 0. {
        return Ok(None);
    }
    let p = y.map(|r| r.map(|v| (v / scale * 63.).round() as i8));
    if !(0..3).any(|k| {
        p[0][k] as i16 * p[1][(k + 1) % 3] as i16 != p[1][k] as i16 * p[0][(k + 1) % 3] as i16
    }) {
        return Ok(None);
    }
    Ok(Some(p))
}
pub(super) fn certify(
    a: &Surface,
    b: &Surface,
    ends: [bool; 2],
    max_cells: usize,
) -> Result<(bool, usize)> {
    let (Some(a), Some(b)) = (
        virtual_face(a, ends[0], true)?,
        virtual_face(b, ends[1], false)?,
    ) else {
        return Ok((false, 0));
    };
    let Some(p) = proposal(&a, &b)? else {
        return Ok((false, 0));
    };
    // Both maps use the same V interval and exact common boundary. Their
    // rectangles glue to [-La,Lb] x [v0,v1], which is convex. Positive symmetric
    // projected Jacobians on every cell imply strict monotonicity along every
    // segment in that union, including segments crossing the shared boundary.
    let first = crate::surface_linear_monotonicity::inspect(&a, p, max_cells)?;
    if !first.certified {
        return Ok((false, first.cells));
    }
    let second = crate::surface_linear_monotonicity::inspect(&b, p, max_cells - first.cells)?;
    Ok((second.certified, first.cells + second.cells))
}
