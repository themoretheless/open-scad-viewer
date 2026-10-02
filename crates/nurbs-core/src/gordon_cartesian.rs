//! Cartesian Gordon assembly before global patch stitching and retention audit.
use super::{
    denominators::{self, Cell},
    normalized, stations,
};
use crate::{Result, check, curve::Curve, surface::Surface};

fn axis(curves: &[Curve], parameters: &[f64], clamped: bool) -> Result<Vec<Cell>> {
    let mut inputs = curves.to_vec();
    for station in 0..parameters.len() {
        let sites: Vec<_> = (0..parameters.len())
            .map(|i| [if i == station { 1. } else { 0. }, 0., 0.])
            .collect();
        inputs.push(if clamped {
            crate::natural_spline::clamped(&sites, parameters, [0.; 3], [0.; 3])?
        } else {
            crate::natural_spline::interpolate(&sites, parameters)?
        });
    }
    if clamped {
        let zeros = vec![[0.; 3]; parameters.len()];
        inputs.push(crate::natural_spline::clamped(
            &zeros,
            parameters,
            [1., 0., 0.],
            [0.; 3],
        )?);
        inputs.push(crate::natural_spline::clamped(
            &zeros,
            parameters,
            [0.; 3],
            [1., 0., 0.],
        )?);
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
#[cfg(test)]
pub(super) fn cells(
    u_curves: &[Curve],
    v_curves: &[Curve],
    parameters_u: &[f64],
    parameters_v: &[f64],
    crossing_budget: f64,
) -> Result<Vec<Surface>> {
    cells_mode(
        u_curves,
        v_curves,
        parameters_u,
        parameters_v,
        crossing_budget,
        None,
    )
}
fn cells_mode(
    u_curves: &[Curve],
    v_curves: &[Curve],
    parameters_u: &[f64],
    parameters_v: &[f64],
    crossing_budget: f64,
    tangents: Option<&[Curve; 2]>,
) -> Result<Vec<Surface>> {
    check(
        (2..=86).contains(&u_curves.len()) && (2..=86).contains(&v_curves.len()),
        "Cartesian Gordon needs 2..86 curves in each family",
    )?;
    let mut u = u_curves
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
                (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt() <= crossing_budget,
                "Cartesian Gordon crossing discrepancy exceeds its explicit budget",
            )?;
            crossings[i][j] = [a[0], a[1], a[2]];
        }
    }
    let section_count = u.len();
    let mut guide_derivatives = vec![[[0.; 3]; 2]; v.len()];
    if let Some(targets) = tangents {
        for target in targets {
            target.validate()?;
            check(
                !target.periodic
                    && target.domain() == [0., 1.]
                    && target.control_points[0].len() == 3,
                "Cartesian tangent fields must be open normalized 3D curves",
            )?;
        }
        for (i, guide) in v.iter().enumerate() {
            for (end, parameter) in [0., 1.].into_iter().enumerate() {
                let actual = guide
                    .evaluate(parameter)?
                    .d1
                    .ok_or_else(|| crate::numeric_err("Missing guide derivative"))?;
                let expected = targets[end].evaluate(pu[i])?.point;
                check(
                    (0..3)
                        .map(|k| (actual[k] - expected[k]).powi(2))
                        .sum::<f64>()
                        .sqrt()
                        <= crossing_budget,
                    "Guide endpoint derivatives conflict with Cartesian tangent fields",
                )?;
                guide_derivatives[i][end] = [actual[0], actual[1], actual[2]];
            }
        }
        u.extend(targets.iter().cloned());
    }
    let ucells = axis(&u, &pu, false)?;
    let vcells = axis(&v, &pv, tangents.is_some())?;
    let degree = |cells: &[Cell]| {
        cells
            .iter()
            .flat_map(|c| c.numerators.iter())
            .map(|n| n.len() - 1)
            .max()
            .unwrap()
    };
    if ucells.len() * degree(&ucells) + 1 > 256 || vcells.len() * degree(&vcells) + 1 > 256 {
        return Err(crate::resource(
            "Cartesian Gordon exceeds 256 controls per axis",
        ));
    }
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
                        let sections: f64 = (0..section_count)
                            .map(|j| un[j][k][a] * vn[v.len() + j][0][b])
                            .sum();
                        let guides: f64 = (0..v.len())
                            .map(|i| un[u.len() + i][0][a] * vn[i][k][b])
                            .sum();
                        let correction: f64 = (0..v.len())
                            .map(|i| {
                                (0..section_count)
                                    .map(|j| {
                                        crossings[i][j][k]
                                            * un[u.len() + i][0][a]
                                            * vn[v.len() + j][0][b]
                                    })
                                    .sum::<f64>()
                            })
                            .sum();
                        let tangent_correction = if tangents.is_some() {
                            (0..2)
                                .map(|end| {
                                    let interpolated: f64 = (0..v.len())
                                        .map(|i| {
                                            un[u.len() + i][0][a] * guide_derivatives[i][end][k]
                                        })
                                        .sum();
                                    (un[section_count + end][k][a] - interpolated)
                                        * vn[v.len() + section_count + end][0][b]
                                })
                                .sum::<f64>()
                        } else {
                            0.
                        };
                        control_points[a][b][k] =
                            (sections + guides - correction + tangent_correction) / w;
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

fn stitch_u(parts: &[Surface], tolerance: f64) -> Result<Surface> {
    let mut result = parts[0].clone();
    for next in &parts[1..] {
        check(
            result.degree_u == next.degree_u
                && result.degree_v == next.degree_v
                && result.knots_v == next.knots_v,
            "Cartesian Gordon cell basis mismatch",
        )?;
        let last = result.control_points.len() - 1;
        check(
            result.knots_u[result.control_points.len()] == next.knots_u[next.degree_u],
            "Cartesian Gordon cells are not adjacent",
        )?;
        let scale = result.weights[last][0] / next.weights[0][0];
        for j in 0..result.control_points[0].len() {
            let distance = (0..3)
                .map(|k| (result.control_points[last][j][k] - next.control_points[0][j][k]).powi(2))
                .sum::<f64>()
                .sqrt();
            crate::numeric(distance <= tolerance, "Cartesian Gordon cell seam mismatch")?;
            let w = next.weights[0][j] * scale;
            crate::numeric(
                (w - result.weights[last][j]).abs() <= 1e-10 * w.abs().max(result.weights[last][j]),
                "Cartesian Gordon cell seam weights mismatch",
            )?;
        }
        if result.control_points.len() + next.control_points.len() - 1 > 256 {
            return Err(crate::resource(
                "Cartesian Gordon exceeds 256 controls per axis",
            ));
        }
        // One owner for each C0 seam; whole-source retention must audit this rounding.
        result
            .control_points
            .extend(next.control_points.iter().skip(1).cloned());
        result.weights.extend(
            next.weights
                .iter()
                .skip(1)
                .map(|row| row.iter().map(|w| w * scale).collect::<Vec<_>>()),
        );
        result
            .knots_u
            .truncate(result.knots_u.len() - result.degree_u - 1);
        result
            .knots_u
            .extend(vec![next.knots_u[next.degree_u]; result.degree_u]);
        result
            .knots_u
            .extend_from_slice(&next.knots_u[next.degree_u + 1..]);
        result.validate()?;
    }
    Ok(result)
}
/// Native candidate construction. Numerical curve retention is a separate gate.
pub(super) fn assemble(
    u: &[Curve],
    v: &[Curve],
    pu: &[f64],
    pv: &[f64],
    tolerance: f64,
) -> Result<Surface> {
    assemble_mode(u, v, pu, pv, tolerance, None)
}
fn assemble_mode(
    u: &[Curve],
    v: &[Curve],
    pu: &[f64],
    pv: &[f64],
    tolerance: f64,
    tangents: Option<&[Curve; 2]>,
) -> Result<Surface> {
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Cartesian Gordon needs positive seam tolerance",
    )?;
    let mut patches = cells_mode(u, v, pu, pv, tolerance, tangents)?;
    let du = patches.iter().map(|s| s.degree_u).max().unwrap();
    let dv = patches.iter().map(|s| s.degree_v).max().unwrap();
    for patch in &mut patches {
        *patch = patch.edit_axis(crate::surface::Axis::U, |c| c.elevate(du))?;
        *patch = patch.edit_axis(crate::surface::Axis::V, |c| c.elevate(dv))?;
    }
    let mut udomains: Vec<_> = patches.iter().map(|s| s.knots_u[du]).collect();
    let mut vdomains: Vec<_> = patches.iter().map(|s| s.knots_v[dv]).collect();
    udomains.sort_by(f64::total_cmp);
    udomains.dedup();
    vdomains.sort_by(f64::total_cmp);
    vdomains.dedup();
    if udomains.len() * du + 1 > 256 || vdomains.len() * dv + 1 > 256 {
        return Err(crate::resource(
            "Cartesian Gordon exceeds 256 controls per axis",
        ));
    }
    let mut strips = Vec::new();
    for start in vdomains {
        let row: Vec<_> = patches
            .iter()
            .filter(|s| s.knots_v[dv] == start)
            .cloned()
            .collect();
        strips.push(super::transpose(stitch_u(&row, tolerance)?));
    }
    Ok(super::transpose(stitch_u(&strips, tolerance)?))
}

pub(super) fn checked(
    u: &[Curve],
    v: &[Curve],
    pu: &[f64],
    pv: &[f64],
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<(Surface, Vec<value_codec::Value>)> {
    let surface = assemble(u, v, pu, pv, tolerance)?;
    let certificates = curve_audits(
        &surface,
        u,
        v,
        pu,
        pv,
        tolerance,
        max_cells,
        max_map_evaluations,
    )?;
    Ok((surface, certificates))
}
fn curve_audits(
    surface: &Surface,
    u: &[Curve],
    v: &[Curve],
    pu: &[f64],
    pv: &[f64],
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<Vec<value_codec::Value>> {
    let pu = stations(pu, v.len())?;
    let pv = stations(pv, u.len())?;
    let mut certificates = Vec::new();
    for (curves, stations, axis) in [
        (u, pv, crate::surface::Axis::V),
        (v, pu, crate::surface::Axis::U),
    ] {
        for (curve, station) in curves.iter().zip(stations) {
            let [start, end] = curve.domain();
            let identity = value_codec::json!({"pieces":[{"domain":[0.,1.],"range":[start,end],
                "controlValues":[start,end],"weights":[1.,1.]}]});
            let result = surface.iso(axis, station)?;
            let report = crate::foundation::certify_reparameterized_curve_retention(
                curve,
                &identity,
                &result,
                tolerance,
                max_cells,
                max_map_evaluations,
            )?;
            let certificate = report["certificate"].clone();
            crate::numeric(
                certificate["accepted"] == value_codec::json!(true),
                &format!(
                    "Cartesian Gordon whole-curve retention was not established: {certificate}"
                ),
            )?;
            certificates.push(certificate);
        }
    }
    Ok(certificates)
}

pub(super) fn checked_with_tangents(
    u: &[Curve],
    v: &[Curve],
    pu: &[f64],
    pv: &[f64],
    targets: &[Curve; 2],
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<(Surface, Vec<value_codec::Value>, Vec<value_codec::Value>)> {
    let surface = assemble_mode(u, v, pu, pv, tolerance, Some(targets))?;
    let curves = curve_audits(
        &surface,
        u,
        v,
        pu,
        pv,
        tolerance,
        max_cells,
        max_map_evaluations,
    )?;
    let mut tangents = Vec::new();
    for (end, target) in targets.iter().enumerate() {
        let certificate = super::tangent_audit::verify(
            &surface,
            std::slice::from_ref(target),
            end == 1,
            tolerance,
            max_cells,
        )?;
        crate::numeric(
            certificate["accepted"] == value_codec::json!(true),
            "Cartesian endpoint tangent retention was not established",
        )?;
        tangents.push(certificate);
    }
    Ok((surface, curves, tangents))
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
        let patches = cells(&u, &v, &[0., 1.], &[0., 1.], 0.).unwrap();
        let (_, certificates) = checked(&u, &v, &[0., 1.], &[0., 1.], 1e-6, 50000, 200000).unwrap();
        assert_eq!(certificates.len(), 4);
        assert!(
            certificates
                .iter()
                .all(|c| c["accepted"] == value_codec::json!(true))
        );
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
        let patches = cells(&u, &v, &stations, &stations, 0.).unwrap();
        let joined = assemble(&u, &v, &stations, &stations, 1e-6).unwrap();
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
                        let global = joined.evaluate(x, stations[j]).unwrap().point;
                        for k in 0..3 {
                            assert!((global[k] - actual[k]).abs() < 1e-11);
                        }
                        let expected = c.evaluate(x).unwrap().point;
                        for k in 0..3 {
                            assert!((actual[k] - expected[k]).abs() < 1e-11);
                        }
                    }
                }
                for (i, c) in v.iter().enumerate() {
                    if stations[i] >= ua && stations[i] <= ub {
                        let actual = patch.evaluate(stations[i], y).unwrap().point;
                        let global = joined.evaluate(stations[i], y).unwrap().point;
                        for k in 0..3 {
                            assert!((global[k] - actual[k]).abs() < 1e-11);
                        }
                        let expected = c.evaluate(y).unwrap().point;
                        for k in 0..3 {
                            assert!((actual[k] - expected[k]).abs() < 1e-11);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn checked_curved_rational_network_retains_all_six_authored_curves() {
        let stations = [0., 0.5, 1.];
        let u: Vec<_> = stations
            .iter()
            .enumerate()
            .map(|(i, &y)| {
                let weight = [0.5, 2., 4.][i];
                let height = [0.5, 1., 0.5][i];
                Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: vec![
                        vec![0., y, 0.],
                        vec![0.5, y, height * (1. + weight) / weight],
                        vec![1., y, 0.],
                    ],
                    weights: vec![1., weight, 1.],
                    periodic: false,
                }
            })
            .collect();
        let v: Vec<_> = stations
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let weight = [4., 2., 0.5][i];
                let (end, middle) = if i == 1 {
                    (0.5, 1. + 0.5 / weight)
                } else {
                    (0., 0.)
                };
                Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: vec![vec![x, 0., end], vec![x, 0.5, middle], vec![x, 1., end]],
                    weights: vec![1., weight, 1.],
                    periodic: false,
                }
            })
            .collect();
        let (surface, certificates) =
            super::super::patch_cartesian(&u, &v, &stations, &stations, 1e-6, 50000, 200000)
                .unwrap();
        assert_eq!(certificates.len(), 6);
        assert!(
            certificates
                .iter()
                .all(|c| c["accepted"] == value_codec::json!(true)
                    && c["errorUpper"].as_f64().unwrap() <= 1e-6)
        );
        assert!((surface.evaluate(0.5, 0.5).unwrap().point[2] - 1.).abs() < 1e-11);
        for sample in 0..=100 {
            let t = sample as f64 / 100.;
            for (j, curve) in u.iter().enumerate() {
                let actual = surface.evaluate(t, stations[j]).unwrap().point;
                let expected = curve.evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((actual[k] - expected[k]).abs() < 1e-10);
                }
            }
            for (i, curve) in v.iter().enumerate() {
                let actual = surface.evaluate(stations[i], t).unwrap().point;
                let expected = curve.evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((actual[k] - expected[k]).abs() < 1e-10);
                }
            }
        }
        // Independent closed-form natural-cardinal basis for stations 0,1/2,1.
        // No production spline/control-net construction is used for this oracle.
        let cardinal = |t: f64| {
            if t <= 0.5 {
                let b = 2. * t;
                let q = 0.25 * (b.powi(3) - b);
                [1. - b + q, b - 2. * q, q]
            } else {
                let b = 2. * t - 1.;
                let a = 1. - b;
                let q = 0.25 * (a.powi(3) - a);
                [q, a - 2. * q, b + q]
            }
        };
        for iu in 0..=10 {
            for iv in 0..=10 {
                let x = iu as f64 / 10.;
                let y = iv as f64 / 10.;
                let m = cardinal(x);
                let l = cardinal(y);
                let mut expected = [0.; 3];
                for j in 0..3 {
                    let point = u[j].evaluate(x).unwrap().point;
                    for k in 0..3 {
                        expected[k] += l[j] * point[k];
                    }
                }
                for i in 0..3 {
                    let point = v[i].evaluate(y).unwrap().point;
                    for k in 0..3 {
                        expected[k] += m[i] * point[k];
                    }
                    for j in 0..3 {
                        let crossing = u[j].evaluate(stations[i]).unwrap().point;
                        for k in 0..3 {
                            expected[k] -= m[i] * l[j] * crossing[k];
                        }
                    }
                }
                let actual = surface.evaluate(x, y).unwrap().point;
                for k in 0..3 {
                    assert!(
                        (actual[k] - expected[k]).abs() < 1e-10,
                        "Cartesian Gordon formula mismatch at ({x},{y}), axis {k}"
                    );
                }
            }
        }
    }
    #[test]
    fn clamped_cartesian_network_retains_weights_curves_and_tangent_fields() {
        let line = |a: [f64; 3], b: [f64; 3], weights: Vec<f64>| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights,
            periodic: false,
        };
        let u = [
            line([0., 0., 0.], [1., 0., 0.], vec![1., 2.]),
            line([0., 0., 1.], [1., 0., 1.], vec![2., 1.]),
        ];
        let v = [
            line([0., 0., 0.], [0., 0., 1.], vec![1., 2.]),
            line([1., 0., 0.], [1., 0., 1.], vec![3., 1.]),
        ];
        let tangents = [
            line([0., 0., 2.], [0., 0., 1. / 3.], vec![1., 1.]),
            line([0., 0., 0.5], [0., 0., 3.], vec![1., 1.]),
        ];
        let (surface, curve_certificates, tangent_certificates) =
            super::super::patch_cartesian_with_tangents(
                &u,
                &v,
                &[0., 1.],
                &[0., 1.],
                &tangents,
                1e-6,
                50000,
                200000,
            )
            .unwrap();
        assert_eq!(curve_certificates.len(), 4);
        assert_eq!(tangent_certificates.len(), 2);
        for certificate in tangent_certificates {
            assert_eq!(certificate["accepted"], true);
            assert!(certificate["errorUpper"].as_f64().unwrap() <= 1e-6);
        }
        for sample in 0..=100 {
            let x = sample as f64 / 100.;
            for end in 0..2 {
                let evaluated = surface.evaluate(x, end as f64).unwrap();
                let expected = u[end].evaluate(x).unwrap().point;
                for k in 0..3 {
                    assert!((evaluated.point[k] - expected[k]).abs() < 1e-11);
                }
                let derivative = evaluated.first_derivatives().unwrap().1;
                let expected = tangents[end].evaluate(x).unwrap().point;
                for k in 0..3 {
                    assert!((derivative[k] - expected[k]).abs() < 1e-10);
                }
            }
        }
        let mut conflict = tangents;
        conflict[0].control_points[0][2] += 1.;
        assert!(
            super::super::patch_cartesian_with_tangents(
                &u,
                &v,
                &[0., 1.],
                &[0., 1.],
                &conflict,
                1e-6,
                50000,
                200000
            )
            .is_err()
        );
    }
}
