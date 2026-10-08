//! Local x/y rational jets through total order three, on original knot cells.
use crate::{Result, distance_bounds::Interval as I, surface::Surface};
type Net = Vec<Vec<[I; 4]>>;
pub(crate) type Jets = [[[I; 3]; 4]; 4];
fn difference(net: Net, axis: usize) -> Result<Net> {
    let degree = if axis == 0 {
        net.len() - 1
    } else {
        net[0].len() - 1
    };
    if degree == 0 {
        return Ok(vec![
            vec![
                [I::point(0.); 4];
                if axis == 0 { net[0].len() } else { 1 }
            ];
            if axis == 0 { 1 } else { net.len() }
        ]);
    }
    let nu = net.len() - usize::from(axis == 0);
    let nv = net[0].len() - usize::from(axis == 1);
    let mut out = vec![vec![[I::point(0.); 4]; nv]; nu];
    for u in 0..nu {
        for v in 0..nv {
            for k in 0..4 {
                out[u][v][k] = net[u + usize::from(axis == 0)][v + usize::from(axis == 1)][k]
                    .sub(net[u][v][k])?
                    .mul(I::point(degree as f64))?;
            }
        }
    }
    Ok(out)
}
fn choose(n: usize, k: usize) -> f64 {
    if k == 0 || k == n { 1. } else { n as f64 }
}
pub(crate) fn calculate(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    corner: Option<[usize; 2]>,
) -> Result<Jets> {
    calculate_partial(s, span, domain, corner.map_or([None; 2], |c| c.map(Some)))
}
/// Restrict selected coordinates to an endpoint; enclose the others fully.
pub(crate) fn calculate_partial(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    corner: [Option<usize>; 2],
) -> Result<Jets> {
    let mut net = crate::curve_surface_composition::surface_net_on(s, span, domain)?;
    let origin = &s.control_points[span[0] - s.degree_u][span[1] - s.degree_v];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let mut h = [[[I::point(0.); 4]; 4]; 4];
    for order in 0..=3 {
        for a in 0..=order {
            let b = order - a;
            let mut d = net.clone();
            for _ in 0..a {
                d = difference(d, 0)?;
            }
            for _ in 0..b {
                d = difference(d, 1)?;
            }
            for k in 0..4 {
                let rows = d.len();
                let points: Vec<_> = d
                    .iter()
                    .enumerate()
                    .flat_map(|(u, row)| {
                        row.iter().enumerate().filter_map(move |(v, p)| {
                            (corner[0].is_none_or(|c| u == c * (rows - 1))
                                && corner[1].is_none_or(|c| v == c * (row.len() - 1)))
                            .then_some(p[k])
                        })
                    })
                    .collect();
                h[a][b][k] = I::new(
                    points.iter().map(|p| p.lo).fold(f64::INFINITY, f64::min),
                    points
                        .iter()
                        .map(|p| p.hi)
                        .fold(f64::NEG_INFINITY, f64::max),
                )?;
            }
        }
    }
    quotient(h)
}
fn quotient(h: [[[I; 4]; 4]; 4]) -> Result<Jets> {
    let mut jets = [[[I::point(0.); 3]; 4]; 4];
    for order in 0..=3 {
        for a in 0..=order {
            let b = order - a;
            for k in 0..3 {
                let mut numerator = h[a][b][k];
                for i in 0..=a {
                    for j in 0..=b {
                        if i + j == 0 {
                            continue;
                        }
                        numerator = numerator.sub(
                            h[i][j][3]
                                .mul(jets[a - i][b - j][k])?
                                .mul(I::point(choose(a, i) * choose(b, j)))?,
                        )?;
                    }
                }
                jets[a][b][k] = numerator.div(h[0][0][3])?;
            }
        }
    }
    Ok(jets)
}

fn evaluate_bernstein(mut points: Vec<[I; 4]>, t: I) -> Result<[I; 4]> {
    let beta = I::point(1.).sub(t)?.intersect(0., 1.)?;
    for count in (1..points.len()).rev() {
        for i in 0..count {
            for k in 0..4 {
                points[i][k] = points[i][k].mul(beta)?.add(points[i + 1][k].mul(t)?)?;
            }
        }
    }
    Ok(points[0])
}
/// Derivatives in ORIGINAL span coordinates, evaluated over a restricted
/// rectangle. Differentiate before restriction so tiny query intervals do not
/// subtract nearly equal restricted coefficients to reconstruct a derivative.
pub(crate) fn calculate_partial_stable(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    corner: [Option<usize>; 2],
) -> Result<Jets> {
    let full = [
        [s.knots_u[span[0]], s.knots_u[span[0] + 1]],
        [s.knots_v[span[1]], s.knots_v[span[1] + 1]],
    ];
    let mut parameter = [I::point(0.); 2];
    for k in 0..2 {
        let query = if let Some(c) = corner[k] {
            I::point(domain[k][c])
        } else {
            I::new(domain[k][0], domain[k][1])?
        };
        parameter[k] = query
            .sub(I::point(full[k][0]))?
            .div(I::point(full[k][1]).sub(I::point(full[k][0]))?)?
            .intersect(0., 1.)?;
    }
    let mut net = crate::curve_surface_composition::surface_net(s, span)?;
    let origin = &s.control_points[span[0] - s.degree_u][span[1] - s.degree_v];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let mut h = [[[I::point(0.); 4]; 4]; 4];
    for order in 0..=3 {
        for a in 0..=order {
            let b = order - a;
            let mut d = net.clone();
            for _ in 0..a {
                d = difference(d, 0)?;
            }
            for _ in 0..b {
                d = difference(d, 1)?;
            }
            let mut rows = Vec::new();
            for row in &d {
                rows.push(evaluate_bernstein(row.clone(), parameter[1])?);
            }
            let value = evaluate_bernstein(rows, parameter[0])?;
            for k in 0..4 {
                // Every derivative polynomial retains the Bernstein convex
                // hull property. In particular, positive weight lower bounds
                // survive wide interval parameters without dependency collapse.
                let lo = d
                    .iter()
                    .flatten()
                    .map(|p| p[k].lo)
                    .fold(f64::INFINITY, f64::min);
                let hi = d
                    .iter()
                    .flatten()
                    .map(|p| p[k].hi)
                    .fold(f64::NEG_INFINITY, f64::max);
                h[a][b][k] = value[k].intersect(lo, hi)?;
            }
        }
    }
    quotient(h)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contains(i: I, value: f64) {
        assert!(i.lo <= value && value <= i.hi, "{value} outside {i:?}");
    }
    #[test]
    fn stable_jets_on_ulp_wide_queries_match_original_rational_derivatives() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 4., 0.]],
                vec![vec![3., 0., 0.], vec![3., 4., 0.]],
            ],
            weights: vec![vec![1., 3.], vec![2., 6.]],
            periodic_u: false,
            periodic_v: false,
        };
        let u = 0.5_f64;
        let v = 0.25_f64;
        let d = [[u.next_down(), u.next_up()], [v.next_down(), v.next_up()]];
        let j = calculate_partial_stable(&s, [1, 1], d, [None; 2]).unwrap();
        contains(j[1][0][0], 6. / (1. + u).powi(2));
        contains(j[2][0][0], -12. / (1. + u).powi(3));
        contains(j[3][0][0], 36. / (1. + u).powi(4));
        contains(j[0][1][1], 12. / (1. + 2. * v).powi(2));
        contains(j[0][2][1], -48. / (1. + 2. * v).powi(3));
        contains(j[0][3][1], 288. / (1. + 2. * v).powi(4));
        assert!(j[1][0][0].hi - j[1][0][0].lo < 1e-8);
        assert!(j[0][1][1].hi - j[0][1][1].lo < 1e-8);
        contains(j[1][1][0], 0.);
        contains(j[1][1][1], 0.);
    }
    #[test]
    fn mixed_polynomial_jets_and_corners_match_independent_equations() {
        let s = crate::polynomial::graph(
            [0., 1., 0., 1.],
            &[vec![0., 0., 0.], vec![0., 0., 1.], vec![0., 1., 0.]],
        )
        .unwrap();
        let domain = [[0.25, 0.75], [0.125, 0.875]];
        let all = calculate(&s, [2, 2], domain, None).unwrap();
        for (corner, u, v) in [
            (Some([0, 0]), 0.25, 0.125),
            (Some([0, 1]), 0.25, 0.875),
            (Some([1, 0]), 0.75, 0.125),
            (Some([1, 1]), 0.75, 0.875),
        ] {
            let c = calculate(&s, [2, 2], domain, corner).unwrap();
            for jets in [&all, &c] {
                contains(jets[0][0][2], u * u * v + u * v * v);
                contains(jets[1][0][2], (2. * u * v + v * v) * 0.5);
                contains(jets[0][1][2], (u * u + 2. * u * v) * 0.75);
                contains(jets[2][0][2], 2. * v * 0.5_f64.powi(2));
                contains(jets[1][1][2], (2. * u + 2. * v) * 0.5 * 0.75);
                contains(jets[2][1][2], 2. * 0.5_f64.powi(2) * 0.75);
                contains(jets[1][2][2], 2. * 0.5 * 0.75_f64.powi(2));
                contains(jets[3][0][2], 0.);
                contains(jets[0][3][2], 0.);
            }
        }
    }
    #[test]
    fn rational_third_jets_match_separable_weighted_chart() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 4., 0.]],
                vec![vec![3., 0., 0.], vec![3., 4., 0.]],
            ],
            weights: vec![vec![1., 3.], vec![2., 6.]],
            periodic_u: false,
            periodic_v: false,
        };
        let domain = [[0.25, 0.75], [0.125, 0.875]];
        let all = calculate(&s, [1, 1], domain, None).unwrap();
        for (corner, u, v) in [(Some([0, 0]), 0.25, 0.125), (Some([1, 1]), 0.75, 0.875)] {
            let c = calculate(&s, [1, 1], domain, corner).unwrap();
            for j in [&all, &c] {
                contains(j[1][0][0], 6. * 0.5 / (1_f64 + u).powi(2));
                contains(j[2][0][0], -12. * 0.5_f64.powi(2) / (1_f64 + u).powi(3));
                contains(j[3][0][0], 36. * 0.5_f64.powi(3) / (1_f64 + u).powi(4));
                contains(j[0][1][1], 12. * 0.75 / (1_f64 + 2. * v).powi(2));
                contains(
                    j[0][2][1],
                    -48. * 0.75_f64.powi(2) / (1_f64 + 2. * v).powi(3),
                );
                contains(
                    j[0][3][1],
                    288. * 0.75_f64.powi(3) / (1_f64 + 2. * v).powi(4),
                );
                contains(j[1][1][0], 0.);
                contains(j[1][2][1], 0.);
            }
        }
    }
}
