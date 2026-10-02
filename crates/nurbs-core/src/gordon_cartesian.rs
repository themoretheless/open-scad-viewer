//! Cartesian Gordon assembly before global patch stitching and retention audit.
use super::{
    denominators::{self, Cell},
    normalized, stations,
};
use crate::{Result, check, curve::Curve, surface::Surface};

fn axis(curves: &[Curve], parameters: &[f64]) -> Result<Vec<Cell>> {
    let mut inputs = curves.to_vec();
    for station in 0..parameters.len() {
        let sites: Vec<_> = (0..parameters.len())
            .map(|i| [if i == station { 1. } else { 0. }, 0., 0.])
            .collect();
        inputs.push(crate::natural_spline::interpolate(&sites, parameters)?);
    }
    denominators::prepare(&inputs)
}
fn elevate(mut controls: Vec<f64>, degree: usize) -> Vec<f64> {
    while controls.len() <= degree {
        let n = controls.len();
        let mut next = vec![controls[0]; n + 1];
        next[n] = controls[n - 1];
        for i in 1..n {
            let a = i as f64 / n as f64;
            next[i] = a * controls[i - 1] + (1. - a) * controls[i];
        }
        controls = next;
    }
    controls
}
/// Produces Bezier cells of sum L_j(v)C_j(u) + sum M_i(u)G_i(v)
/// minus the tensor interpolation of the Cartesian crossing points.
/// Shared denominators are products of positive univariate denominators.
pub(super) fn cells(
    u_curves: &[Curve],
    v_curves: &[Curve],
    parameters_u: &[f64],
    parameters_v: &[f64],
) -> Result<Vec<Surface>> {
    check(
        (2..=86).contains(&u_curves.len()) && (2..=86).contains(&v_curves.len()),
        "Cartesian Gordon needs 2..86 curves in each family",
    )?;
    let u = u_curves
        .iter()
        .map(normalized)
        .collect::<Result<Vec<_>>>()?;
    let v = v_curves
        .iter()
        .map(normalized)
        .collect::<Result<Vec<_>>>()?;
    let pu = stations(parameters_u, v.len())?;
    let pv = stations(parameters_v, u.len())?;
    let mut crossings = vec![vec![[0.; 3]; u.len()]; v.len()];
    for i in 0..v.len() {
        for j in 0..u.len() {
            let a = u[j].evaluate(pu[i])?.point;
            let b = v[i].evaluate(pv[j])?.point;
            check(
                a == b,
                "Cartesian Gordon crossings must agree geometrically",
            )?;
            crossings[i][j] = [a[0], a[1], a[2]];
        }
    }
    let ucells = axis(&u, &pu)?;
    let vcells = axis(&v, &pv)?;
    let mut result = Vec::new();
    for uc in &ucells {
        for vc in &vcells {
            let du = uc.numerators.iter().map(|n| n.len() - 1).max().unwrap();
            let dv = vc.numerators.iter().map(|n| n.len() - 1).max().unwrap();
            let qu = elevate(uc.denominator.clone(), du);
            let qv = elevate(vc.denominator.clone(), dv);
            let un: Vec<Vec<Vec<f64>>> = uc
                .numerators
                .iter()
                .map(|n| {
                    (0..3)
                        .map(|k| elevate(n.iter().map(|p| p[k]).collect(), du))
                        .collect()
                })
                .collect();
            let vn: Vec<Vec<Vec<f64>>> = vc
                .numerators
                .iter()
                .map(|n| {
                    (0..3)
                        .map(|k| elevate(n.iter().map(|p| p[k]).collect(), dv))
                        .collect()
                })
                .collect();
            let mut control_points = vec![vec![vec![0.; 3]; dv + 1]; du + 1];
            let mut weights = vec![vec![0.; dv + 1]; du + 1];
            for a in 0..=du {
                for b in 0..=dv {
                    let w = qu[a] * qv[b];
                    weights[a][b] = w;
                    for k in 0..3 {
                        let sections: f64 = (0..u.len())
                            .map(|j| un[j][k][a] * vn[v.len() + j][0][b])
                            .sum();
                        let guides: f64 = (0..v.len())
                            .map(|i| un[u.len() + i][0][a] * vn[i][k][b])
                            .sum();
                        let correction: f64 = (0..v.len())
                            .map(|i| {
                                (0..u.len())
                                    .map(|j| {
                                        crossings[i][j][k]
                                            * un[u.len() + i][0][a]
                                            * vn[v.len() + j][0][b]
                                    })
                                    .sum::<f64>()
                            })
                            .sum();
                        control_points[a][b][k] = (sections + guides - correction) / w;
                    }
                }
            }
            let knots = |degree: usize, domain: [f64; 2]| {
                let mut k = vec![domain[0]; degree + 1];
                k.extend(vec![domain[1]; degree + 1]);
                k
            };
            let surface = Surface {
                degree_u: du,
                degree_v: dv,
                knots_u: knots(du, uc.domain),
                knots_v: knots(dv, vc.domain),
                control_points,
                weights,
                periodic_u: false,
                periodic_v: false,
            };
            surface.validate()?;
            result.push(surface);
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cartesian_cells_retain_boundaries_with_incompatible_weights() {
        let line = |a: [f64; 3], b: [f64; 3], weights: Vec<f64>| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights,
            periodic: false,
        };
        let u = [
            line([0., 0., 0.], [1., 0., 0.], vec![1., 2.]),
            line([0., 1., 0.], [1., 1., 1.], vec![3., 1.]),
        ];
        let v = [
            line([0., 0., 0.], [0., 1., 0.], vec![2., 1.]),
            line([1., 0., 0.], [1., 1., 1.], vec![1., 4.]),
        ];
        assert!(super::super::patch(&u, &v, &[0., 1.], &[0., 1.]).is_err());
        let patches = cells(&u, &v, &[0., 1.], &[0., 1.]).unwrap();
        assert_eq!(patches.len(), 1);
        let s = &patches[0];
        for sample in 0..=100 {
            let t = sample as f64 / 100.;
            for (i, c) in u.iter().enumerate() {
                let actual = s.evaluate(t, i as f64).unwrap().point;
                let expected = c.evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((actual[k] - expected[k]).abs() < 1e-12);
                }
            }
            for (i, c) in v.iter().enumerate() {
                let actual = s.evaluate(i as f64, t).unwrap().point;
                let expected = c.evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((actual[k] - expected[k]).abs() < 1e-12);
                }
            }
        }
    }
    #[test]
    fn cartesian_cells_retain_three_by_three_rational_network() {
        let make = |station: f64, weight: f64, transpose: bool| {
            let mut curve = Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![
                    vec![0., station, 0.],
                    vec![0.5, station, 0.5 * station],
                    vec![1., station, station],
                ],
                weights: vec![1., weight, 1.],
                periodic: false,
            };
            if transpose {
                for p in &mut curve.control_points {
                    p.swap(0, 1);
                }
            }
            curve
        };
        let stations = [0., 0.5, 1.];
        let u: Vec<_> = stations
            .iter()
            .enumerate()
            .map(|(i, &s)| make(s, [0.5, 2., 4.][i], false))
            .collect();
        let v: Vec<_> = stations
            .iter()
            .enumerate()
            .map(|(i, &s)| make(s, [4., 0.5, 2.][i], true))
            .collect();
        let patches = cells(&u, &v, &stations, &stations).unwrap();
        assert_eq!(patches.len(), 4);
        for patch in &patches {
            assert!(patch.weights.iter().flatten().all(|w| *w > 0.));
            let [ua, ub] = [
                patch.knots_u[patch.degree_u],
                patch.knots_u[patch.control_points.len()],
            ];
            let [va, vb] = [
                patch.knots_v[patch.degree_v],
                patch.knots_v[patch.control_points[0].len()],
            ];
            for sample in 0..=100 {
                let t = sample as f64 / 100.;
                let x = ua + (ub - ua) * t;
                let y = va + (vb - va) * t;
                for (j, c) in u.iter().enumerate() {
                    if stations[j] >= va && stations[j] <= vb {
                        let actual = patch.evaluate(x, stations[j]).unwrap().point;
                        let expected = c.evaluate(x).unwrap().point;
                        for k in 0..3 {
                            assert!((actual[k] - expected[k]).abs() < 1e-11);
                        }
                    }
                }
                for (i, c) in v.iter().enumerate() {
                    if stations[i] >= ua && stations[i] <= ub {
                        let actual = patch.evaluate(stations[i], y).unwrap().point;
                        let expected = c.evaluate(y).unwrap().point;
                        for k in 0..3 {
                            assert!((actual[k] - expected[k]).abs() < 1e-11);
                        }
                    }
                }
            }
        }
    }
}
