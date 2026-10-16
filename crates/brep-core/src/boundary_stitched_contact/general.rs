//! Two original charts in one common affine parameter coordinate system.
//! Original knots, world poles and weights are preserved, including non-dyadic
//! station knots. Only transposition swaps axes; parameter rates are enclosed.
use super::*;
use nurbs_core::interval_eval::Interval as I;
struct Chart {
    surface: Surface,
    rates: [[f64; 2]; 2],
}
fn sample(s: &Surface) -> Option<([f64; 3], [f64; 3])> {
    let inside = |k: &[f64], p: usize, n: usize| {
        let i = (p..n).find(|&i| k[i] < k[i + 1])?;
        let x = k[i] * 0.629 + k[i + 1] * 0.371;
        (k[i] < x && x < k[i + 1]).then_some(x)
    };
    s.evaluate(
        inside(&s.knots_u, s.degree_u, s.control_points.len())?,
        inside(&s.knots_v, s.degree_v, s.control_points[0].len())?,
    )
    .ok()?
    .first_derivatives()
}
fn clamped(s: &Surface) -> bool {
    let knots = [&s.knots_u, &s.knots_v];
    let degrees = [s.degree_u, s.degree_v];
    let sizes = [s.control_points.len(), s.control_points[0].len()];
    !s.periodic_u
        && !s.periodic_v
        && (0..2).all(|k| {
            let (t, p, n) = (knots[k], degrees[k], sizes[k]);
            t.len() == n + p + 1
                && t[..=p].iter().all(|&v| v == t[p])
                && t[n..].iter().all(|&v| v == t[n])
                && t[p] < t[n]
        })
        && s.degree_u <= 8
        && s.degree_v <= 8
        && sizes[0] <= 128
        && sizes[1] <= 33
}
fn free_basis(a: &Surface, b: &Surface, mirror: bool) -> bool {
    if a.degree_v != b.degree_v || a.control_points[0].len() != b.control_points[0].len() {
        return false;
    }
    // All clamped Bezier bases have the same normalized free traversal.
    if a.control_points[0].len() == a.degree_v + 1 {
        return true;
    }
    if !mirror {
        return a.knots_v == b.knots_v;
    }
    let end = a.knots_v[a.degree_v] + a.knots_v[a.control_points[0].len()];
    let lo = a.knots_v[a.degree_v];
    let hi = a.knots_v[a.control_points[0].len()];
    let residual = |x: f64, y: f64| {
        let sum = x + y;
        let v = sum - x;
        (sum, (x - (sum - v)) + (y - v))
    };
    if residual(lo, hi).1 != 0.
        || b.knots_v[b.degree_v] != lo
        || b.knots_v[b.control_points[0].len()] != hi
    {
        return false;
    }
    a.knots_v
        .iter()
        .zip(b.knots_v.iter().rev())
        .all(|(&x, &y)| residual(x, y) == (end, 0.))
}
fn chart(s: Surface, end: bool, left: bool, mirror: bool) -> Option<Chart> {
    let (du, dv) = sample(&s)?;
    let norm = |v: [f64; 3]| v[0].hypot(v[1]).hypot(v[2]);
    let width = I::point(s.knots_v[s.control_points[0].len()])
        .sub(I::point(s.knots_v[s.degree_v]))
        .ok()?;
    if width.lo <= 0. {
        return None;
    }
    let proposal = norm(dv) * (width.lo * 0.5 + width.hi * 0.5) / norm(du);
    if !proposal.is_finite() || proposal <= 0. {
        return None;
    }
    let metric = 2_f64.powi((proposal.log2().round() as i32).clamp(-20, 20));
    let sign = if left == end { 1. } else { -1. };
    let free = if mirror {
        [-width.hi, -width.lo]
    } else {
        [width.lo, width.hi]
    };
    Some(Chart {
        surface: s,
        rates: [[sign * metric; 2], free],
    })
}
fn prepare(a: &Surface, b: &Surface, axes: [usize; 2], ends: [bool; 2]) -> Option<(Chart, Chart)> {
    let a = if axes[0] == 1 {
        nurbs_core::surface_edit::transpose(a).ok()?
    } else {
        a.clone()
    };
    let b = if axes[1] == 1 {
        nurbs_core::surface_edit::transpose(b).ok()?
    } else {
        b.clone()
    };
    if !clamped(&a) || !clamped(&b) {
        return None;
    }
    let left = strip(&a, 0, ends[0]);
    let right = strip(&b, 0, ends[1]);
    if left.len() != right.len() {
        return None;
    }
    let direct = left.iter().zip(&right).all(|(x, y)| x.0 == y.0);
    let mirror = !direct;
    if mirror && !left.iter().zip(right.iter().rev()).all(|(x, y)| x.0 == y.0) {
        return None;
    }
    if !free_basis(&a, &b, mirror) {
        return None;
    }
    let right = if mirror {
        right.into_iter().rev().collect::<Vec<_>>()
    } else {
        right
    };
    let polynomial =
        left.iter().all(|p| p.1 == left[0].1) && right.iter().all(|p| p.1 == right[0].1);
    let scale = left[0].1 / right[0].1;
    if !polynomial
        && (!scale.is_finite()
            || scale <= 0.
            || left
                .iter()
                .zip(&right)
                .any(|(x, y)| exact_product(scale, y.1) != Some(x.1)))
    {
        return None;
    }
    // Proportional shared weights prove identical boundary parameterization.
    // Do not rescale any original weight or transform/round any source knot.
    Some((
        chart(a, ends[0], true, false)?,
        chart(b, ends[1], false, mirror)?,
    ))
}
fn projection(a: &Chart, b: &Chart) -> Option<[[i8; 3]; 2]> {
    let rates = |c: &Chart| c.rates.map(|v| v[0] * 0.5 + v[1] * 0.5);
    let (au, av) = sample(&a.surface)?;
    let (bu, bv) = sample(&b.surface)?;
    let ar = rates(a);
    let br = rates(b);
    let du: [f64; 3] = std::array::from_fn(|k| (au[k] * ar[0]) * 0.5 + (bu[k] * br[0]) * 0.5);
    let dv: [f64; 3] = std::array::from_fn(|k| (av[k] * ar[1]) * 0.5 + (bv[k] * br[1]) * 0.5);
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| a[k] * b[k]).sum::<f64>();
    let (uu, uv, vv) = (dot(du, du), dot(du, dv), dot(dv, dv));
    let dual: [[f64; 3]; 2] = [
        std::array::from_fn(|k| vv * du[k] - uv * dv[k]),
        std::array::from_fn(|k| uu * dv[k] - uv * du[k]),
    ];
    let mut p = [[0i8; 3]; 2];
    for row in 0..2 {
        let scale = dual[row].iter().map(|x| x.abs()).fold(0_f64, f64::max);
        if !scale.is_finite() || scale <= 0. {
            return None;
        }
        p[row] = dual[row].map(|x| (63. * x / scale).round() as i8);
    }
    (0..3)
        .any(|k| {
            p[0][k] as i16 * p[1][(k + 1) % 3] as i16 != p[1][k] as i16 * p[0][(k + 1) % 3] as i16
        })
        .then_some(p)
}
/// The affine source maps define halfrectangles [-La,0] x [0,1] and
/// [0,Lb] x [0,1], with possibly non-binary real La/Lb. Their exact common
/// rational boundary makes the union continuous and convex. The SAME fixed
/// projection/metric must have positive symmetric Jacobians on BOTH pieces.
/// Integrating along any segment proves joint injectivity even across C0 seams
/// and different transverse degrees. P-matrix-only results are insufficient.
pub(super) fn certify(
    a: &Surface,
    b: &Surface,
    axes: [usize; 2],
    ends: [bool; 2],
    max_cells: usize,
) -> Result<Option<surface_linear_monotonicity::Report>> {
    let Some((a, b)) = prepare(a, b, axes, ends) else {
        return Ok(None);
    };
    let Some(p) = projection(&a, &b) else {
        return Ok(None);
    };
    let first = surface_linear_monotonicity::inspect_strong_reparameterized(
        &a.surface,
        p,
        [1., 1.],
        a.rates,
        max_cells,
    )?;
    if !first.certified {
        return Ok(Some(first));
    }
    let mut second = surface_linear_monotonicity::inspect_strong_reparameterized(
        &b.surface,
        p,
        [1., 1.],
        b.rates,
        max_cells - first.cells,
    )?;
    second.cells += first.cells;
    Ok(Some(second))
}
