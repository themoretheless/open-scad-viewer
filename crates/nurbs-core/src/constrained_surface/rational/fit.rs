//! Bounded homogeneous equality/box QP proposals with working-set exchange.
use super::*;
use rows::Rows;
pub(super) fn encode(s: &Surface, origin: [f64; 3]) -> Vec<f64> {
    let n = s.control_points.len() * s.control_points[0].len();
    let mut x = vec![0.; 4 * n];
    for (i, p) in s.control_points.iter().flatten().enumerate() {
        let w = s.weights[i / s.weights[0].len()][i % s.weights[0].len()];
        for k in 0..3 {
            x[k * n + i] = (p[k] - origin[k]) * w;
        }
        x[3 * n + i] = w;
    }
    x
}
fn energy(
    seed: &Surface,
    origin: [f64; 3],
    options: super::super::Options,
) -> Result<(Vec<Vec<f64>>, Vec<f64>)> {
    if !options.anchor_weight.is_finite()
        || options.anchor_weight <= 0.
        || !options.fairness_weight.is_finite()
        || options.fairness_weight < 0.
    {
        return Err(error(
            "Finite positive homogeneous objective weights required",
        ));
    }
    let (nu, nv) = (seed.control_points.len(), seed.control_points[0].len());
    let n = nu * nv;
    let mut h = vec![vec![0.; n]; n];
    for i in 0..n {
        h[i][i] = options.anchor_weight;
    }
    let mut penalize = |ids: [usize; 3]| {
        for (i, a) in ids.iter().zip([1., -2., 1.]) {
            for (j, b) in ids.iter().zip([1., -2., 1.]) {
                h[*i][*j] += options.fairness_weight * a * b;
            }
        }
    };
    for i in 0..nu {
        for j in 1..nv - 1 {
            penalize([i * nv + j - 1, i * nv + j, i * nv + j + 1]);
        }
    }
    for i in 1..nu - 1 {
        for j in 0..nv {
            penalize([(i - 1) * nv + j, i * nv + j, (i + 1) * nv + j]);
        }
    }
    let l = super::super::cholesky(&h)?;
    let original = encode(seed, origin);
    let mut x0 = Vec::new();
    for k in 0..4 {
        x0.extend(super::super::triangular_solve(
            &l,
            &original[k * n..(k + 1) * n]
                .iter()
                .map(|x| x * options.anchor_weight)
                .collect::<Vec<_>>(),
        ));
    }
    Ok((l, x0))
}
#[derive(Clone, Copy)]
struct Bound {
    index: usize,
    value: f64,
    lower: bool,
}
pub(super) struct Proposal {
    pub surface: Surface,
    pub active_bounds: usize,
    pub active_iterations: usize,
    pub equation_rank: usize,
}
pub(super) fn propose(
    seed: &Surface,
    current: &Surface,
    origin: [f64; 3],
    mut rows: Rows,
    pins: &[(usize, f64)],
    options: Options,
) -> Result<Proposal> {
    let n = seed.control_points.len() * seed.control_points[0].len();
    let (l, x0) = energy(seed, origin, options.network.network.fitting)?;
    for &(i, value) in pins {
        let mut row = vec![0.; 4 * n];
        row[i] = 1.;
        rows.add(row, value)?;
    }
    let mut active = Vec::<Bound>::new();
    for iteration in 0..options.max_active_iterations {
        let base = rows.a.len();
        let mut a = rows.a.clone();
        let mut b = rows.b.clone();
        for bound in &active {
            let mut row = vec![0.; 4 * n];
            row[3 * n + bound.index] = 1.;
            a.push(row);
            b.push(bound.value);
        }
        if a.len() > 2048 {
            return Err(error("Homogeneous active equation allowance exhausted"));
        }
        let rhs = b
            .iter()
            .zip(&a)
            .map(|(b, a)| b - super::super::dot(a, &x0))
            .collect::<Vec<_>>();
        let (correction, rank, dual) = super::super::linear::homogeneous_correction(&l, &a, &rhs)?;
        let x = x0
            .iter()
            .zip(correction)
            .map(|(x, y)| x + y)
            .collect::<Vec<_>>();
        let residual = a
            .iter()
            .zip(&b)
            .map(|(a, b)| (super::super::dot(a, &x) - b).abs() / (1. + b.abs()))
            .fold(0f64, f64::max);
        if residual > 1e-8 {
            // A newly blocking bound may require exchanging an old active
            // face. Failed working sets are proposals, never infeasibility
            // certificates. The global iteration limit prevents cycling.
            if active.len() > 1 {
                active.remove(0);
                continue;
            }
            return Err(error(
                "Homogeneous working constraints are inconsistent or unresolved",
            ));
        }
        let tolerance = 1e-10 * (1. + options.weight_bounds[1]);
        let violation = (0..n)
            .filter_map(|i| {
                let w = x[3 * n + i];
                if w < options.weight_bounds[0] - tolerance {
                    Some((
                        options.weight_bounds[0] - w,
                        Bound {
                            index: i,
                            value: options.weight_bounds[0],
                            lower: true,
                        },
                    ))
                } else if w > options.weight_bounds[1] + tolerance {
                    Some((
                        w - options.weight_bounds[1],
                        Bound {
                            index: i,
                            value: options.weight_bounds[1],
                            lower: false,
                        },
                    ))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, bound)) = violation {
            if active.iter().any(|b| b.index == bound.index) {
                return Err(error("Active weight bound could not be retained"));
            }
            active.push(bound);
            continue;
        }
        // x-x0=H^-1 A^T lambda. Lower/upper active rows require
        // lambda >=0 / <=0 respectively for the box KKT signs.
        let release = active
            .iter()
            .enumerate()
            .filter_map(|(i, b)| {
                let value = dual[base + i];
                let invalid = if b.lower { -value } else { value };
                (invalid > 1e-9).then_some((i, invalid))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = release {
            active.remove(i);
            continue;
        }
        let mut surface = current.clone();
        for i in 0..n {
            let mut w = x[3 * n + i].clamp(options.weight_bounds[0], options.weight_bounds[1]);
            if let Some((_, fixed)) = pins.iter().find(|(index, _)| *index == 3 * n + i) {
                w = *fixed;
            }
            if !w.is_finite() || w < options.weight_bounds[0] || w > options.weight_bounds[1] {
                return Err(error(
                    "Actual homogeneous weights exceed their positive box",
                ));
            }
            let u = i / surface.weights[0].len();
            let v = i % surface.weights[0].len();
            surface.weights[u][v] = w;
            for k in 0..3 {
                surface.control_points[u][v][k] = origin[k] + x[k * n + i] / w;
            }
        }
        surface.validate()?;
        return Ok(Proposal {
            surface,
            active_bounds: active.len(),
            active_iterations: iteration + 1,
            equation_rank: rank,
        });
    }
    Err(error(
        "Bounded homogeneous active-set search did not converge",
    ))
}
