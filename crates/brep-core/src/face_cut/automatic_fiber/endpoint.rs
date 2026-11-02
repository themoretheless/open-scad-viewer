//! Bounded numeric UV proposals only. The caller must reprove the complete
//! edited trace against the unchanged original root operator and both supports.
use nurbs_core::surface::Surface;

pub(super) fn propose(
    surface: &Surface,
    target: [f64; 3],
    seed: [f64; 2],
    domain: [[f64; 2]; 2],
    evaluations: &mut usize,
    max_evaluations: usize,
) -> [f64; 2] {
    let mut uv = seed;
    let mut best = seed;
    let mut error = f64::INFINITY;
    for _ in 0..8 {
        if *evaluations >= max_evaluations {
            break;
        }
        *evaluations += 1;
        let Ok(e) = surface.evaluate(uv[0], uv[1]) else {
            break;
        };
        let residual: [f64; 3] = std::array::from_fn(|k| target[k] - e.point[k]);
        let squared = dot(residual, residual);
        if !squared.is_finite() || squared >= error {
            break;
        }
        best = uv;
        error = squared;
        let Some((u, v)) = e.first_derivatives() else {
            break;
        };
        let (uu, uv_dot, vv) = (dot(u, u), dot(u, v), dot(v, v));
        let determinant = uu * vv - uv_dot * uv_dot;
        if !determinant.is_finite() || determinant <= 0. {
            break;
        }
        let (ru, rv) = (dot(residual, u), dot(residual, v));
        let step = [
            (vv * ru - uv_dot * rv) / determinant,
            (uu * rv - uv_dot * ru) / determinant,
        ];
        if step.iter().any(|s| !s.is_finite()) {
            break;
        }
        let next = std::array::from_fn(|k| (uv[k] + step[k]).clamp(domain[k][0], domain[k][1]));
        if next == uv {
            break;
        }
        uv = next;
    }
    best
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
