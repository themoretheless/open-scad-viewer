//! Finite Newton and rational collocation proposals; full original-lift gates.
use super::super::refused;
use super::{dot, sub};
use crate::{Model, Result};
use nurbs_core::{
    curve::Curve, curve_pullback, curve_surface_agreement as agreement, surface::Surface,
};
fn choose(n: usize, k: usize) -> f64 {
    let mut x = 1.;
    for i in 0..k.min(n - k) {
        x = x * (n - i) as f64 / (i + 1) as f64;
    }
    x
}
fn bernstein(n: usize, i: usize, t: f64) -> f64 {
    choose(n, i) * t.powi(i as i32) * (1. - t).powi((n - i) as i32)
}
fn product(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut c = vec![0.; a.len() + b.len() - 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            c[i + j] += x * y * choose(a.len() - 1, i) * choose(b.len() - 1, j)
                / choose(c.len() - 1, i + j);
        }
    }
    c
}
/// Tiny bounded dense solves propose coefficients; no rank/sample certificate.
fn solve(mut a: Vec<Vec<f64>>, mut rhs: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>> {
    let n = a.len();
    for k in 0..n {
        let row = (k..n)
            .max_by(|&i, &j| a[i][k].abs().total_cmp(&a[j][k].abs()))
            .unwrap();
        if a[row][k].abs() < 1e-13 || !a[row][k].is_finite() {
            return Err(refused("Contact proposal matrix is singular"));
        }
        a.swap(k, row);
        rhs.swap(k, row);
        let pivot = a[k][k];
        for j in k..n {
            a[k][j] /= pivot;
        }
        for r in &mut rhs[k] {
            *r /= pivot;
        }
        for i in 0..n {
            if i != k {
                let f = a[i][k];
                for j in k..n {
                    a[i][j] -= f * a[k][j];
                }
                for j in 0..rhs[i].len() {
                    rhs[i][j] -= f * rhs[k][j];
                }
            }
        }
    }
    if rhs.iter().flatten().any(|v| !v.is_finite()) {
        return Err(refused("Contact proposal overflow"));
    }
    Ok(rhs)
}
fn seed(source: &Model, face: usize, vertex: usize) -> Result<[f64; 2]> {
    let f = &source.faces[face];
    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        for c in &source.loops[wire].coedges {
            let e = &source.edges[c.edge];
            for end in 0..2 {
                if e.vertices[end ^ usize::from(c.reversed)] == vertex {
                    let q = c.pcurve.evaluate(c.pcurve.domain()[end])?.point;
                    return Ok([q[0], q[1]]);
                }
            }
        }
    }
    Err(refused("Original end face does not own the sharp vertex"))
}
fn intersect(
    s: &Surface,
    ray: [f64; 3],
    axis: [f64; 3],
    mut uv: [f64; 2],
    mut z: f64,
    tol: f64,
) -> Result<([f64; 2], f64)> {
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    for _ in 0..32 {
        let e = s.evaluate(uv[0], uv[1])?;
        let residual = std::array::from_fn(|k| e.point[k] - ray[k] - axis[k] * z);
        if dot(residual, residual).sqrt() <= tol {
            return Ok((uv, z));
        }
        let (u, v) = e
            .first_derivatives()
            .ok_or_else(|| refused("Original cap derivatives unresolved"))?;
        let delta = solve(
            (0..3).map(|k| vec![u[k], v[k], -axis[k]]).collect(),
            residual.iter().map(|x| vec![-x]).collect(),
        )?;
        let mut accepted = false;
        for half in 0..12 {
            let step = 0.5f64.powi(half);
            let q = std::array::from_fn(|k| {
                (uv[k] + step * delta[k][0]).clamp(domain[k][0], domain[k][1])
            });
            let h = z + step * delta[2][0];
            let p = s.evaluate(q[0], q[1])?.point;
            let r = std::array::from_fn(|k| p[k] - ray[k] - axis[k] * h);
            if dot(r, r) < dot(residual, residual) {
                uv = q;
                z = h;
                accepted = true;
                break;
            }
        }
        if !accepted {
            break;
        }
    }
    Err(refused(
        "Bounded original cap/ray intersection did not converge",
    ))
}
fn charge(cells: &mut usize, add: usize, max: usize) -> Result<()> {
    *cells += add;
    if *cells > max {
        return Err(refused("Automatic end-contact proof allowance exhausted"));
    }
    Ok(())
}
pub(super) fn construct(
    source: &Model,
    face: usize,
    vertex: usize,
    skin: &Surface,
    axis: [f64; 3],
    cells: &mut usize,
    max: usize,
) -> Result<(Curve, Curve)> {
    let initial = seed(source, face, vertex)?;
    let cap = &source.faces[face].surface;
    let span = dot(
        sub(skin.evaluate(0., 1.)?.point, skin.evaluate(0., 0.)?.point),
        axis,
    );
    if span <= 0. {
        return Err(refused("Automatic axial carrier is reversed"));
    }
    for power in 1..=3 {
        let mut denominator = vec![1.];
        for _ in 0..power {
            denominator = product(&denominator, &[1., skin.weights[1][0], 1.]);
        }
        let degree = denominator.len();
        let mut weights = Vec::new();
        let mut u_controls = Vec::new();
        for i in 0..=degree {
            let left = if i > 0 {
                i as f64 / degree as f64 * denominator[i - 1]
            } else {
                0.
            };
            let right = if i < degree {
                (1. - i as f64 / degree as f64) * denominator[i]
            } else {
                0.
            };
            weights.push(left + right);
            u_controls.push(left / (left + right));
        }
        let mut matrix = Vec::new();
        let mut rhs = Vec::new();
        let mut uv = initial;
        for i in 0..=degree {
            // Chebyshev-Lobatto sites are interpolation proposals, not proof.
            let t = (1. - (std::f64::consts::PI * i as f64 / degree as f64).cos()) / 2.;
            let ray = skin.evaluate(t, 0.)?.point;
            let z0 = dot(sub(source.vertices[vertex].point, ray), axis);
            let (q, z) = intersect(cap, ray, axis, uv, z0, source.tolerance_mm / 256.)?;
            uv = q;
            let v = z / span;
            if !(0. ..=1.).contains(&v) {
                return Err(refused("Original cap contact exceeds automatic carrier"));
            }
            let p = skin.evaluate(t, v)?.point;
            let den = (0..=degree)
                .map(|j| bernstein(degree, j, t) * weights[j])
                .sum::<f64>();
            matrix.push(
                (0..=degree)
                    .map(|j| bernstein(degree, j, t) * weights[j] / den)
                    .collect(),
            );
            rhs.push(vec![v, p[0], p[1], p[2]]);
        }
        let fitted = solve(matrix, rhs)?;
        let knots = [vec![0.; degree + 1], vec![1.; degree + 1]].concat();
        let pc = Curve {
            degree,
            knots: knots.clone(),
            weights: weights.clone(),
            periodic: false,
            control_points: fitted
                .iter()
                .enumerate()
                .map(|(i, p)| vec![u_controls[i], p[0]])
                .collect(),
        };
        if pc.control_points.iter().any(|p| p[1] < 0. || p[1] > 1.) {
            continue;
        }
        let world = Curve {
            degree,
            knots,
            weights,
            periodic: false,
            control_points: fitted.iter().map(|p| p[1..].to_vec()).collect(),
        };
        pc.validate()?;
        world.validate()?;
        if *cells >= max {
            return Err(refused("Automatic end-contact work exhausted"));
        }
        let lift = agreement::verify(
            &world,
            &pc,
            skin,
            false,
            source.tolerance_mm / 16.,
            max - *cells,
        )?;
        charge(cells, lift.cells, max)?;
        if lift.status != agreement::Status::WithinTolerance {
            continue;
        }
        if *cells >= max {
            return Err(refused("Automatic original end-face work exhausted"));
        }
        let original = curve_pullback::project(
            &world,
            cap,
            curve_pullback::Limits {
                tolerance: source.tolerance_mm / 16.,
                max_segments: 8,
                max_iterations: 32,
                agreement_cells: max - *cells,
            },
        )?;
        charge(cells, original.agreement_cells, max)?;
        if original.within_tolerance {
            return Ok((pc, world));
        }
    }
    Err(refused(
        "No rational end-contact candidate passed the whole original chart proof",
    ))
}
