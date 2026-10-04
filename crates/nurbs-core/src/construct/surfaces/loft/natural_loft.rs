//! Natural cubic interpolation of aligned rational sections in homogeneous space.
use crate::{Result, check, curve::Curve, surface::Surface};

/// Parameters are finite, strictly increasing section stations. V is normalized
/// to [0,1]. The natural boundary condition applies to homogeneous controls;
/// rational Cartesian second derivatives need not vanish at the boundary.
pub fn interpolate(sections: &[Curve], parameters: &[f64]) -> Result<Surface> {
    construct(sections, parameters, Boundary::Natural)
}

/// End derivatives dP/dt are constant across each boundary section. Endpoint
/// homogeneous weight derivatives are zero; V is normalized after construction.
pub fn clamped(
    sections: &[Curve],
    parameters: &[f64],
    start_tangent: [f64; 3],
    end_tangent: [f64; 3],
) -> Result<Surface> {
    check(
        start_tangent
            .iter()
            .chain(&end_tangent)
            .all(|x| x.is_finite()),
        "Clamped loft tangents must be finite",
    )?;
    construct(
        sections,
        parameters,
        Boundary::Clamped([start_tangent, end_tangent]),
    )
}

/// Tangent controls use the aligned U basis and endpoint weights. The resulting
/// Cartesian dP/dt field is sum(N_i*w_i*t_i)/sum(N_i*w_i); W'=0.
/// Arrays must match the U control count after section basis alignment.
pub fn clamped_control_tangents(
    sections: &[Curve],
    parameters: &[f64],
    start: &[[f64; 3]],
    end: &[[f64; 3]],
) -> Result<Surface> {
    check(
        start.iter().chain(end).flatten().all(|x| x.is_finite()),
        "Loft tangent controls must be finite",
    )?;
    construct(
        sections,
        parameters,
        Boundary::ControlTangents([start.to_vec(), end.to_vec()]),
    )
}

/// Closed cubic loft requires the repeated end section to match the first in
/// aligned homogeneous controls. Output uses clamped knots, not periodic wrapping.
pub fn closed(sections: &[Curve], parameters: &[f64]) -> Result<Surface> {
    check(
        (4..=11).contains(&sections.len()),
        "Closed loft needs 4..11 sections including the repeated endpoint",
    )?;
    construct(sections, parameters, Boundary::Closed)
}

#[derive(Clone)]
enum Boundary {
    Natural,
    Clamped([[f64; 3]; 2]),
    Closed,
    ControlTangents([Vec<[f64; 3]>; 2]),
}

fn construct(sections: &[Curve], parameters: &[f64], boundary: Boundary) -> Result<Surface> {
    check(
        (2..=11).contains(&sections.len()) && parameters.len() == sections.len(),
        "Natural loft needs 2..11 sections and matching parameters",
    )?;
    let aligned = crate::surface::loft_aligned(sections)?;
    if let Boundary::ControlTangents(t) = &boundary {
        check(
            t.iter().all(|x| x.len() == aligned.control_points.len()),
            "Loft tangent controls must match the aligned U control count",
        )?;
    }
    let mut points = Vec::with_capacity(aligned.control_points.len());
    let mut weights = Vec::with_capacity(aligned.control_points.len());
    let mut knots_v = Vec::new();
    for (i, (row, wr)) in aligned
        .control_points
        .iter()
        .zip(&aligned.weights)
        .enumerate()
    {
        let mut xyz = Vec::with_capacity(row.len());
        for (p, w) in row.iter().zip(wr) {
            let h: [f64; 3] = std::array::from_fn(|k| p[k] * w);
            check(
                (0..3).all(|k| h[k].is_finite() && (p[k] == 0. || h[k] != 0.)),
                "Natural loft homogeneous control is unrepresentable",
            )?;
            xyz.push(h);
        }
        let scalar: Vec<[f64; 3]> = wr.iter().map(|&w| [w, 0., 0.]).collect();
        let tangents = match &boundary {
            Boundary::Clamped(t) => Some(*t),
            Boundary::ControlTangents(t) => Some([t[0][i], t[1][i]]),
            _ => None,
        };
        let (h, w) = if let Some(t) = tangents {
            let start = t[0].map(|x| x * wr[0]);
            let end = t[1].map(|x| x * wr[wr.len() - 1]);
            check(
                (0..3).all(|k| {
                    start[k].is_finite()
                        && end[k].is_finite()
                        && (t[0][k] == 0. || start[k] != 0.)
                        && (t[1][k] == 0. || end[k] != 0.)
                }),
                "Clamped loft homogeneous tangent is unrepresentable",
            )?;
            (
                crate::natural_spline::clamped(&xyz, parameters, start, end)?,
                crate::natural_spline::clamped(&scalar, parameters, [0.; 3], [0.; 3])?,
            )
        } else if matches!(&boundary, Boundary::Closed) {
            (
                crate::closed_spline::interpolate(&xyz, parameters)?,
                crate::closed_spline::interpolate(&scalar, parameters)?,
            )
        } else {
            (
                crate::natural_spline::interpolate(&xyz, parameters)?,
                crate::natural_spline::interpolate(&scalar, parameters)?,
            )
        };
        let mut pr = Vec::with_capacity(h.control_points.len());
        let mut weights_row = Vec::with_capacity(h.control_points.len());
        for (p, weight) in h.control_points.iter().zip(&w.control_points) {
            let weight = weight[0];
            check(
                weight.is_finite() && weight > 0.,
                "Natural loft homogeneous interpolation needs positive control weights",
            )?;
            let q: Vec<f64> = p.iter().map(|x| x / weight).collect();
            check(
                q.iter()
                    .zip(p)
                    .all(|(q, h)| q.is_finite() && (*h == 0. || *q != 0.)),
                "Natural loft control is unrepresentable",
            )?;
            pr.push(q);
            weights_row.push(weight);
        }
        knots_v = h.knots;
        points.push(pr);
        weights.push(weights_row);
    }
    let s = Surface {
        degree_u: aligned.degree_u,
        degree_v: 3,
        knots_u: aligned.knots_u,
        knots_v,
        control_points: points,
        weights,
        periodic_u: aligned.periodic_u,
        periodic_v: false,
    };
    s.validate()?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cyclic_sections() -> Vec<Curve> {
        [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.]]
            .into_iter()
            .map(|xy| {
                let mut c = section(0., 1.);
                for p in &mut c.control_points {
                    p[0] += xy[0];
                    p[1] += xy[1];
                }
                c
            })
            .collect()
    }
    #[test]
    fn closed_loft_matches_independent_cyclic_cubic_and_section_sites() {
        let c = cyclic_sections();
        let s = closed(&c, &[0., 1., 2., 3., 4.]).unwrap();
        assert!(!s.periodic_v);
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            for j in 0..5 {
                let p = s.evaluate(u, j as f64 / 4.).unwrap().point;
                let q = c[j].evaluate(u).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-10);
                }
            }
            // First span is the cubic with center controls (1,0),(1,.5),(.5,1),(0,1).
            let base = section(0., 1.).evaluate(u).unwrap().point;
            let p = s.evaluate(u, 0.125).unwrap().point;
            assert!((p[0] - base[0] - 0.6875).abs() < 1e-10);
            assert!((p[1] - base[1] - 0.6875).abs() < 1e-10);
        }
    }
    #[test]
    fn closed_loft_rational_seam_has_matching_second_jets() {
        let s = closed(&cyclic_sections(), &[-2., -1., 0.5, 2., 4.]).unwrap();
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            let a = s.evaluate(u, 0.).unwrap();
            let b = s.evaluate(u, 1.).unwrap();
            let (au, av) = a.first_derivatives().unwrap();
            let (bu, bv) = b.first_derivatives().unwrap();
            let (auu, auv, avv) = a.second_derivatives().unwrap();
            let (buu, buv, bvv) = b.second_derivatives().unwrap();
            for (x, y) in [
                (a.point, b.point),
                (au, bu),
                (av, bv),
                (auu, buu),
                (auv, buv),
                (avv, bvv),
            ] {
                for k in 0..3 {
                    assert!((x[k] - y[k]).abs() < 1e-8);
                }
            }
        }
        let mut c = cyclic_sections();
        c[4].weights[0] *= 1.001;
        assert!(closed(&c, &[0., 1., 2., 3., 4.]).is_err());
        assert!(closed(&c[..3], &[0., 1., 2.]).is_err());
    }
    #[test]
    fn clamped_loft_preserves_constant_cartesian_boundary_tangents() {
        let s = clamped(
            &[section(0., 1.), section(3., 1.2), section(7., 1.4)],
            &[2., 3., 7.],
            [1., 2., 3.],
            [-2., 1., 4.],
        )
        .unwrap();
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            for (v, t) in [(0., [1., 2., 3.]), (1., [-2., 1., 4.])] {
                let (_, dv) = s.evaluate(u, v).unwrap().first_derivatives().unwrap();
                for k in 0..3 {
                    assert!((dv[k] - 5. * t[k]).abs() < 1e-9);
                }
            }
            let q = s.evaluate(u, 0.2).unwrap().point;
            let p = section(3., 1.2).evaluate(u).unwrap().point;
            for k in 0..3 {
                assert!((q[k] - p[k]).abs() < 1e-10);
            }
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_control_tangent_loft_keeps_variable_boundary_derivative() {
        let curves = [section(0., 1.), section(7., 1.)];
        let t = [[0., 0., 1.], [0., 0., 3.], [0., 0., -2.]];
        let result = crate::transport::dispatch(value_codec::json!({
            "op":"surface_control_tangent_loft", "curves":curves,
            "parameters":[2.,7.], "start_tangents":t,"end_tangents":t
        }))
        .unwrap();
        let s: Surface = value_codec::from_value(result).unwrap();
        for (u, z) in [(0., 5.), (1., -10.)] {
            let (_, dv) = s.evaluate(u, 0.).unwrap().first_derivatives().unwrap();
            assert!((dv[2] - z).abs() < 1e-9);
        }
        assert!(
            crate::transport::dispatch(value_codec::json!({
                "op":"surface_control_tangent_loft", "curves":curves,
                "parameters":[2.,7.], "start_tangents":[],"end_tangents":t
            }))
            .is_err()
        );
    }

    #[test]
    fn varying_tangent_field_matches_independent_rational_hermite() {
        let a = section(0., 1.);
        let b = section(7., 1.);
        let start = [[0., 0., 1.], [0., 0., 3.], [0., 0., -2.]];
        let end = [[0., 0., -1.], [0., 0., 2.], [0., 0., 4.]];
        let s = clamped_control_tangents(&[a.clone(), b], &[2., 7.], &start, &end).unwrap();
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            let basis = [(1. - u) * (1. - u), 2. * u * (1. - u), u * u];
            let denom: f64 = (0..3).map(|i| basis[i] * a.weights[i]).sum();
            let field = |t: &[[f64; 3]]| {
                (0..3)
                    .map(|i| basis[i] * a.weights[i] * t[i][2])
                    .sum::<f64>()
                    / denom
            };
            for (v, t) in [(0., &start), (1., &end)] {
                let (_, dv) = s.evaluate(u, v).unwrap().first_derivatives().unwrap();
                assert!((dv[2] - 5. * field(t)).abs() < 1e-9);
            }
            for v in [0.13_f64, 0.5, 0.87] {
                let z = (-2. * v.powi(3) + 3. * v * v) * 7.
                    + (v.powi(3) - 2. * v * v + v) * 5. * field(&start)
                    + (v.powi(3) - v * v) * 5. * field(&end);
                let p = s.evaluate(u, v).unwrap().point;
                let q = a.evaluate(u).unwrap().point;
                assert!((p[2] - z).abs() < 1e-9);
                assert!((p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9);
            }
        }
        assert!(
            clamped_control_tangents(&[a.clone(), a.clone()], &[0., 1.], &start[..2], &end)
                .is_err()
        );
        let mut bad = start;
        bad[1][0] = f64::NAN;
        assert!(clamped_control_tangents(&[a.clone(), a], &[0., 1.], &bad, &end).is_err());
    }

    #[test]
    fn two_section_clamped_loft_matches_independent_hermite_basis() {
        let s = clamped(
            &[section(0., 1.), section(7., 1.)],
            &[2., 7.],
            [0., 0., 3.],
            [0., 0., -1.],
        )
        .unwrap();
        for v in [0.13_f64, 0.3, 0.5, 0.7, 0.87] {
            let z = (-2. * v * v * v + 3. * v * v) * 7. + (v * v * v - 2. * v * v + v) * 15.
                - (v * v * v - v * v) * 5.;
            assert!((s.evaluate(0.37, v).unwrap().point[2] - z).abs() < 1e-10);
        }
        assert!(
            clamped(
                &[section(0., 1.), section(1., 1.)],
                &[0., 1.],
                [f64::NAN, 0., 0.],
                [0.; 3]
            )
            .is_err()
        );
    }
    #[test]
    fn rational_second_jets_match_across_nonuniform_section_seam() {
        let s = interpolate(
            &[section(0., 1.), section(3., 1.2), section(7., 1.4)],
            &[-2., -1., 3.],
        )
        .unwrap();
        let left = s.trim([0., 1., 0., 0.2]).unwrap();
        let right = s.trim([0., 1., 0.2, 1.]).unwrap();
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            let a = left.evaluate(u, 0.2).unwrap();
            let b = right.evaluate(u, 0.2).unwrap();
            let (au, av) = a.first_derivatives().unwrap();
            let (bu, bv) = b.first_derivatives().unwrap();
            let (auu, auv, avv) = a.second_derivatives().unwrap();
            let (buu, buv, bvv) = b.second_derivatives().unwrap();
            for (x, y) in [
                (a.point, b.point),
                (au, bu),
                (av, bv),
                (auu, buu),
                (auv, buv),
                (avv, bvv),
            ] {
                for k in 0..3 {
                    assert!((x[k] - y[k]).abs() < 1e-8);
                }
            }
        }
    }
    fn section(z: f64, scale: f64) -> Curve {
        let mut c = crate::primitives::circle_arc([0., 0., z], [0., 0., 1.], 2., 0., 90.).unwrap();
        c.weights.iter_mut().for_each(|w| *w *= scale);
        c
    }
    #[test]
    fn natural_homogeneous_boundary_has_nonzero_cartesian_second_derivative() {
        // Independent natural spline endpoint slopes: W'=1.075, H_z'=19.025.
        // H_z''=W''=0 and Z=0 give Z''=-2 W' Z'/W.
        let s = interpolate(
            &[section(0., 1.), section(3., 1.2), section(7., 1.4)],
            &[0., 0.2, 1.],
        )
        .unwrap();
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            let q = s.evaluate(u, 0.).unwrap();
            let (_, dv) = q.first_derivatives().unwrap();
            let (_, _, dvv) = q.second_derivatives().unwrap();
            assert!((dv[2] - 19.025).abs() < 1e-10);
            assert!((dvv[2] + 2. * 1.075 * 19.025).abs() < 1e-9);
        }
        let s = interpolate(
            &[section(0., 1.), section(3., 1.), section(7., 1.)],
            &[0., 0.2, 1.],
        )
        .unwrap();
        for v in [0., 1.] {
            let (_, _, dvv) = s.evaluate(0.37, v).unwrap().second_derivatives().unwrap();
            assert!(dvv.iter().all(|x| x.abs() < 1e-9));
        }
    }
    #[test]
    fn interpolates_rational_sections_at_nonuniform_stations() {
        let sections = vec![section(0., 1.), section(3., 1.2), section(7., 1.4)];
        let s = interpolate(&sections, &[-2., -1., 3.]).unwrap();
        for (j, v) in [0., 0.2, 1.].into_iter().enumerate() {
            for u in [0., 0.17, 0.5, 0.83, 1.] {
                let expected = sections[j].evaluate(u).unwrap().point;
                let actual = s.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!((actual[k] - expected[k]).abs() < 1e-11);
                }
            }
        }
    }
    #[test]
    fn matches_independent_cubic_between_sections() {
        let sections = vec![section(0., 1.), section(3., 1.), section(0., 1.)];
        let s = interpolate(&sections, &[0., 0.5, 1.]).unwrap();
        for v in [0.13_f64, 0.3, 0.5, 0.7, 0.87] {
            let x = v.min(1. - v);
            let expected = 9. * x - 12. * x * x * x;
            let p = s.evaluate(0.37, v).unwrap().point;
            assert!((p[2] - expected).abs() < 1e-11);
            assert!((p[0] * p[0] + p[1] * p[1] - 4.).abs() < 1e-11);
        }
    }
    #[test]
    fn rejects_invalid_stations_and_weight_overshoot() {
        assert!(interpolate(&[], &[]).is_err());
        let sections = vec![section(0., 1.), section(1., 1.)];
        assert!(interpolate(&sections, &[0., 0.]).is_err());
        assert!(interpolate(&sections, &[0., f64::NAN]).is_err());
        let sections = vec![section(0., 1.), section(1., 0.01), section(2., 10.)];
        assert!(interpolate(&sections, &[0., 0.5, 1.]).is_err());
    }
    #[test]
    fn aligns_section_degree_knots_and_domain() {
        let first = section(0., 1.);
        let mut second = section(3., 1.).elevate(3).unwrap().insert(0.4, 1).unwrap();
        second.knots.iter_mut().for_each(|k| *k = 7. + 2. * *k);
        let s = interpolate(&[first.clone(), second.clone()], &[4., 9.]).unwrap();
        for u in [0., 0.17, 0.4, 0.7, 1.] {
            for (v, c, t) in [(0., &first, u), (1., &second, 7. + 2. * u)] {
                let p = s.evaluate(u, v).unwrap().point;
                let q = c.evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-11);
                }
            }
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_boundary_retains_the_constructor_contract() {
        let sections = vec![section(0., 1.), section(2., 1.)];
        let v = crate::transport::dispatch(value_codec::json!({
            "op":"surface_natural_loft", "curves":sections, "parameters":[2.,5.]
        }))
        .unwrap();
        let s: Surface = value_codec::from_value(v).unwrap();
        assert!((s.evaluate(0.5, 0.3).unwrap().point[2] - 0.6).abs() < 1e-11);
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_clamped_boundary_uses_authored_parameter_derivatives() {
        let v = crate::transport::dispatch(value_codec::json!({
            "op":"surface_clamped_loft", "curves":[section(0.,1.),section(2.,1.2)],
            "parameters":[2.,5.], "start_tangent":[0.,0.,4.], "end_tangent":[0.,0.,-1.]
        }))
        .unwrap();
        let s: Surface = value_codec::from_value(v).unwrap();
        for u in [0., 0.37, 1.] {
            assert!(
                (s.evaluate(u, 0.).unwrap().first_derivatives().unwrap().1[2] - 12.).abs() < 1e-10
            );
            assert!(
                (s.evaluate(u, 1.).unwrap().first_derivatives().unwrap().1[2] + 3.).abs() < 1e-10
            );
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_closed_loft_preserves_the_cycle_without_periodic_encoding() {
        let v = crate::transport::dispatch(value_codec::json!({
            "op":"surface_closed_loft", "curves":cyclic_sections(),
            "parameters":[0.,1.,2.,3.,4.]
        }))
        .unwrap();
        let s: Surface = value_codec::from_value(v).unwrap();
        assert!(!s.periodic_v);
        let a = s.evaluate(0.37, 0.).unwrap().point;
        let b = s.evaluate(0.37, 1.).unwrap().point;
        assert_eq!(a, b);
    }
}
