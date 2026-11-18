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

#[cfg(test)]
mod gradient_tests {
    use super::*;

    /// Homogeneous fairing objective per coordinate block: anchor
    /// `w/2 |x - x_seed|^2` plus `f/2` times squared index-space second
    /// differences along both net directions.
    fn objective(x: &[f64], seed: &[f64], nu: usize, nv: usize, w: f64, f: f64) -> f64 {
        let anchor = x.iter().zip(seed).map(|(a, b)| (a - b) * (a - b)).sum::<f64>();
        let mut fair = 0.;
        for i in 0..nu {
            for j in 1..nv - 1 {
                let d = x[i * nv + j - 1] - 2. * x[i * nv + j] + x[i * nv + j + 1];
                fair += d * d;
            }
        }
        for i in 1..nu - 1 {
            for j in 0..nv {
                let d = x[(i - 1) * nv + j] - 2. * x[i * nv + j] + x[(i + 1) * nv + j];
                fair += d * d;
            }
        }
        0.5 * w * anchor + 0.5 * f * fair
    }

    /// The non-rational constrained solve assembles the same objective inline;
    /// without constraints its result must be the objective's stationary point.
    #[test]
    fn unconstrained_polynomial_solve_is_stationary_for_the_fairing_objective() {
        let mut state = 0x0847_5eed_u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 11) as f64 / (1u64 << 52) as f64 - 1.
        };
        for degree in [2, 3] {
            let net = (0..=degree)
                .map(|_| (0..=degree).map(|_| vec![next(), next(), next()]).collect())
                .collect();
            let seed = crate::patches::bezier(net, None).unwrap();
            let mut options = super::super::super::Options::default();
            options.anchor_weight = 0.5 + next().abs();
            options.fairness_weight = 0.1 + next().abs();
            let (w, f) = (options.anchor_weight, options.fairness_weight);
            let solved = super::super::super::solve(&seed, &[], options).unwrap().surface;
            let (nu, nv) = (seed.control_points.len(), seed.control_points[0].len());
            for axis in 0..3 {
                let block: Vec<f64> = seed.control_points.iter().flatten().map(|p| p[axis]).collect();
                let at_rest: Vec<f64> = solved.control_points.iter().flatten().map(|p| p[axis]).collect();
                let h = 1e-5;
                for i in 0..nu * nv {
                    let mut plus = at_rest.clone();
                    plus[i] += h;
                    let mut minus = at_rest.clone();
                    minus[i] -= h;
                    let d = (objective(&plus, &block, nu, nv, w, f)
                        - objective(&minus, &block, nu, nv, w, f))
                        / (2. * h);
                    assert!(d.abs() < 1e-7 * (1. + w), "degree {degree} axis {axis} control {i}: {d}");
                }
            }
        }
    }

    #[test]
    fn homogeneous_fairing_gradient_matches_central_differences() {
        let mut state = 0x5eed_0847_u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 11) as f64 / (1u64 << 52) as f64 - 1.
        };
        for trial in 0..4 {
            let degree = 2 + trial % 2;
            let net = (0..=degree)
                .map(|_| (0..=degree).map(|_| vec![next(), next(), next()]).collect())
                .collect();
            let mut seed = crate::patches::bezier(net, None).unwrap();
            seed.weights[1][1] = 1. + 0.5 * next().abs();
            let mut options = super::super::super::Options::default();
            options.anchor_weight = 0.5 + next().abs();
            options.fairness_weight = 0.1 + next().abs();
            let origin = [0.1, -0.2, 0.3];
            let (l, x0) = energy(&seed, origin, options).unwrap();
            let encoded = encode(&seed, origin);
            let (nu, nv) = (seed.control_points.len(), seed.control_points[0].len());
            let n = nu * nv;
            let (w, f) = (options.anchor_weight, options.fairness_weight);
            for k in 0..4 {
                let block = &encoded[k * n..(k + 1) * n];
                let x: Vec<f64> = block.iter().map(|v| v + 0.3 * next()).collect();
                // Analytic gradient L Lᵀ x - w x_seed of the factored Hessian.
                let lt_x: Vec<f64> = (0..n)
                    .map(|i| (i..n).map(|j| l[j][i] * x[j]).sum())
                    .collect();
                let gradient: Vec<f64> = (0..n)
                    .map(|i| (0..=i).map(|j| l[i][j] * lt_x[j]).sum::<f64>() - w * block[i])
                    .collect();
                let base = objective(&x, block, nu, nv, w, f);
                let h = 1e-6;
                for i in 0..n {
                    let mut plus = x.clone();
                    plus[i] += h;
                    let mut minus = x.clone();
                    minus[i] -= h;
                    let numeric_derivative = (objective(&plus, block, nu, nv, w, f)
                        - objective(&minus, block, nu, nv, w, f))
                        / (2. * h);
                    let scale = gradient[i]
                        .abs()
                        .max(numeric_derivative.abs())
                        .max(base)
                        .max(1.);
                    assert!(
                        (gradient[i] - numeric_derivative).abs() / scale < 1e-6,
                        "trial {trial} block {k} control {i}: analytic={} numeric={numeric_derivative}",
                        gradient[i]
                    );
                }
                // The unconstrained proposal is the stationary point.
                let at_rest = &x0[k * n..(k + 1) * n];
                let h = 1e-5;
                for i in 0..n {
                    let mut plus = at_rest.to_vec();
                    plus[i] += h;
                    let mut minus = at_rest.to_vec();
                    minus[i] -= h;
                    let d = (objective(&plus, block, nu, nv, w, f)
                        - objective(&minus, block, nu, nv, w, f))
                        / (2. * h);
                    assert!(d.abs() < 1e-7 * (1. + w), "block {k} control {i}: {d}");
                }
            }
        }
    }
}
