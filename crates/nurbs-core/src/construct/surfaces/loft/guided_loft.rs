//! Section interpolation with complete retained guide curves and optional
//! endpoint tangent controls. Compatibility is homogeneous, without snapping.
use crate::{
    Result, check,
    curve::{Curve, basis},
    surface::{Axis, Surface},
};

fn derivative(c: &Curve, parameter: f64) -> Result<[f64; 4]> {
    let b = basis(c.degree, &c.knots, c.control_points.len(), parameter, false)?;
    let mut h = [0.; 4];
    for ((p, w), n) in c.control_points.iter().zip(&c.weights).zip(b.d1) {
        for k in 0..3 {
            h[k] += p[k] * w * n;
        }
        h[3] += w * n;
    }
    check(
        h.iter().all(|x| x.is_finite()),
        "Guided loft derivative overflow",
    )?;
    Ok(h)
}

fn across_guides(curves: &[Curve], stations: &[f64]) -> Result<Surface> {
    if curves.len() == 1 {
        return Ok(crate::gordon::transpose(crate::surface::loft(&[
            curves[0].clone(),
            curves[0].clone(),
        ])?));
    }
    Ok(crate::gordon::transpose(crate::natural_loft::interpolate(
        curves, stations,
    )?))
}

/// `guide_parameters` are normalized U locations. One guide may lie anywhere
/// in [0,1]; with multiple guides, missing boundaries retain base isocurves.
/// Effective guide count including retained boundaries must not exceed 86. Section stations are
/// authored V parameters. Guide domains normalize to V=[0,1].
/// Optional tangent controls use the aligned section U basis, in dP/dt units.
/// Guides must match homogeneous section crossings exactly. With tangents,
/// guide endpoint homogeneous derivatives must match the base loft exactly.
pub fn interpolate(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    guide_parameters: &[f64],
    tangents: Option<[Vec<[f64; 3]>; 2]>,
) -> Result<Surface> {
    interpolate_budgeted(sections, parameters, guides, guide_parameters, tangents, 0.)
}

pub(crate) fn interpolate_budgeted(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    guide_parameters: &[f64],
    tangents: Option<[Vec<[f64; 3]>; 2]>,
    crossing_budget: f64,
) -> Result<Surface> {
    check(
        (2..=86).contains(&sections.len()),
        "Guided loft needs 2..86 sections",
    )?;
    check(
        (1..=86).contains(&guides.len()) && guide_parameters.len() == guides.len(),
        "Guided loft needs 1..86 guides and matching U stations",
    )?;
    check(
        guide_parameters
            .iter()
            .all(|u| u.is_finite() && (0. ..=1.).contains(u))
            && guide_parameters.windows(2).all(|u| u[0] < u[1]),
        "Guided loft U stations must increase within [0,1]",
    )?;
    let pv = crate::gordon::stations(parameters, sections.len())?;
    let sections: Vec<_> = sections
        .iter()
        .map(crate::gordon::normalized)
        .collect::<Result<_>>()?;
    let mut guides: Vec<_> = guides
        .iter()
        .map(crate::gordon::normalized)
        .collect::<Result<_>>()?;
    for (guide, &u) in guides.iter().zip(guide_parameters) {
        for (section, &v) in sections.iter().zip(&pv) {
            let a = crate::gordon::homogeneous(section, u)?;
            let b = crate::gordon::homogeneous(guide, v)?;
            let compatible = if crossing_budget == 0. {
                a == b
            } else {
                (0..3)
                    .map(|k| (a[k] / a[3] - b[k] / b[3]).powi(2))
                    .sum::<f64>()
                    .sqrt()
                    <= crossing_budget
                    && (a[3] - b[3]).abs() <= crossing_budget * a[3].abs().max(b[3].abs())
            };
            check(
                compatible,
                "Guided loft homogeneous crossings must match exactly or within the explicit alignment budget",
            )?;
        }
    }
    let base = match &tangents {
        Some(t) => {
            crate::natural_loft::clamped_control_tangents(&sections, parameters, &t[0], &t[1])?
        }
        None => crate::natural_loft::interpolate(&sections, parameters)?,
    };
    let iso: Vec<_> = guide_parameters
        .iter()
        .map(|&u| base.iso(Axis::U, u))
        .collect::<Result<_>>()?;
    if tangents.is_some() {
        for (guide, original) in guides.iter().zip(&iso) {
            for v in [0., 1.] {
                check(
                    derivative(guide, v)? == derivative(original, v)?,
                    "Guide endpoint derivatives conflict with loft tangent controls",
                )?;
            }
        }
    }
    let mut iso = iso;
    let mut stations = guide_parameters.to_vec();
    if guides.len() > 1 {
        if stations[0] != 0. {
            let edge = base.iso(Axis::U, 0.)?;
            guides.insert(0, edge.clone());
            iso.insert(0, edge);
            stations.insert(0, 0.);
        }
        if *stations.last().unwrap() != 1. {
            let edge = base.iso(Axis::U, 1.)?;
            guides.push(edge.clone());
            iso.push(edge);
            stations.push(1.);
        }
        check(
            guides.len() <= 86,
            "Guided loft exceeds 86 effective guides including retained boundaries",
        )?;
    }
    // The correction vanishes on each section. At every guide station its
    // value is the retained guide minus the corresponding base isocurve.
    crate::gordon::combine([
        base,
        across_guides(&guides, &stations)?,
        across_guides(&iso, &stations)?,
    ])
}

/// Build authored dP/dt fields in the aligned section basis and normalized
/// candidates for Cartesian construction. Original fields remain audit targets.
pub fn authored_control_tangent_fields(
    sections: &[Curve],
    parameters: &[f64],
    controls: &[Vec<[f64; 3]>; 2],
) -> Result<([Curve; 2], [Curve; 2])> {
    check(
        (2..=86).contains(&sections.len()),
        "Cartesian tangent loft needs 2..86 sections",
    )?;
    crate::gordon::stations(parameters, sections.len())?;
    let aligned = crate::surface::loft_aligned(sections)?;
    check(
        controls
            .iter()
            .all(|c| c.len() == aligned.control_points.len())
            && controls.iter().flatten().flatten().all(|x| x.is_finite()),
        "Tangent controls must be finite and match the aligned U control count",
    )?;
    let fields: [Curve; 2] = std::array::from_fn(|end| Curve {
        degree: aligned.degree_u,
        knots: aligned.knots_u.clone(),
        control_points: controls[end].iter().map(|p| p.to_vec()).collect(),
        weights: aligned
            .weights
            .iter()
            .map(|w| if end == 0 { w[0] } else { *w.last().unwrap() })
            .collect(),
        periodic: false,
    });
    let extent = parameters[parameters.len() - 1] - parameters[0];
    let mut normalized = fields.clone();
    for field in &mut normalized {
        for point in &mut field.control_points {
            for x in point {
                *x *= extent;
            }
        }
        field.validate()?;
    }
    for field in &fields {
        field.validate()?;
    }
    Ok((fields, normalized))
}

pub fn interpolate_cartesian_with_control_tangents(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    guide_parameters: &[f64],
    controls: &[Vec<[f64; 3]>; 2],
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<(Surface, Vec<value_codec::Value>, Vec<value_codec::Value>)> {
    let (authored, normalized) = authored_control_tangent_fields(sections, parameters, controls)?;
    let (surface, curves, _) = interpolate_cartesian_with_tangents(
        sections,
        parameters,
        guides,
        guide_parameters,
        &normalized,
        tolerance,
        max_cells,
        max_map_evaluations,
    )?;
    let mut certificates = Vec::new();
    for end in 0..2 {
        let certificate = crate::gordon::certify_authored_boundary_v_tangent(
            &surface,
            &authored[end],
            end == 1,
            parameters,
            tolerance,
            max_cells,
        )?;
        crate::numeric(
            certificate["accepted"] == value_codec::json!(true),
            "Authored tangent unit conversion could not be certified",
        )?;
        certificates.push(certificate);
    }
    Ok((surface, curves, certificates))
}

/// Guided Cartesian interpolation with independent rational weights.
/// Missing U boundaries are natural Cartesian splines through section endpoints.
/// The returned numerical certificates cover authored sections, guides and the
/// generated boundary guides. Authored dP/dt arrays are supported by
/// interpolate_cartesian_with_control_tangents; normalized rational fields use
/// interpolate_cartesian_with_tangents.
pub fn interpolate_cartesian(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    guide_parameters: &[f64],
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<(Surface, Vec<value_codec::Value>)> {
    let (surface, curves, _) = interpolate_cartesian_mode(
        sections,
        parameters,
        guides,
        guide_parameters,
        None,
        tolerance,
        max_cells,
        max_map_evaluations,
    )?;
    Ok((surface, curves))
}
/// Tangent fields describe dP/dV with normalized U and V parameters.
pub fn interpolate_cartesian_with_tangents(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    guide_parameters: &[f64],
    tangents: &[Curve; 2],
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<(Surface, Vec<value_codec::Value>, Vec<value_codec::Value>)> {
    interpolate_cartesian_mode(
        sections,
        parameters,
        guides,
        guide_parameters,
        Some(tangents),
        tolerance,
        max_cells,
        max_map_evaluations,
    )
}
fn interpolate_cartesian_mode(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    guide_parameters: &[f64],
    tangents: Option<&[Curve; 2]>,
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<(Surface, Vec<value_codec::Value>, Vec<value_codec::Value>)> {
    check(
        (2..=86).contains(&sections.len()),
        "Cartesian guided loft needs 2..86 sections",
    )?;
    check(
        (1..=86).contains(&guides.len()) && guides.len() == guide_parameters.len(),
        "Cartesian guided loft needs matching guides and stations",
    )?;
    check(
        guide_parameters
            .iter()
            .all(|u| u.is_finite() && (0. ..=1.).contains(u))
            && guide_parameters.windows(2).all(|u| u[0] < u[1]),
        "Cartesian guided loft guide stations must increase in [0,1]",
    )?;
    let normalized = sections
        .iter()
        .map(crate::gordon::normalized)
        .collect::<Result<Vec<_>>>()?;
    let mut network = guides.to_vec();
    let mut stations = guide_parameters.to_vec();
    let normalized_parameters = crate::gordon::stations(parameters, sections.len())?;
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
    }
    let boundary = |parameter| -> Result<Curve> {
        let sites = normalized
            .iter()
            .map(|c| {
                c.evaluate(parameter)
                    .map(|e| [e.point[0], e.point[1], e.point[2]])
            })
            .collect::<Result<Vec<_>>>()?;
        if let Some(targets) = tangents {
            let a = targets[0].evaluate(parameter)?.point;
            let b = targets[1].evaluate(parameter)?.point;
            crate::natural_spline::clamped(
                &sites,
                &normalized_parameters,
                [a[0], a[1], a[2]],
                [b[0], b[1], b[2]],
            )
        } else {
            crate::natural_spline::interpolate(&sites, parameters)
        }
    };
    if stations[0] != 0. {
        network.insert(0, boundary(0.)?);
        stations.insert(0, 0.);
    }
    if *stations.last().unwrap() != 1. {
        network.push(boundary(1.)?);
        stations.push(1.);
    }
    check(
        network.len() <= 86,
        "Cartesian guided loft exceeds 86 effective guides",
    )?;
    if let Some(targets) = tangents {
        crate::gordon::patch_cartesian_with_tangents(
            sections,
            &network,
            &stations,
            parameters,
            targets,
            tolerance,
            max_cells,
            max_map_evaluations,
        )
    } else {
        let (surface, curves) = crate::gordon::patch_cartesian(
            sections,
            &network,
            &stations,
            parameters,
            tolerance,
            max_cells,
            max_map_evaluations,
        )?;
        Ok((surface, curves, Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn section(z: f64) -> Curve {
        crate::primitives::line([0., 0., z], [1., 0., z]).unwrap()
    }
    fn guide(u: f64) -> Curve {
        crate::paths::bezier(
            vec![vec![u, 0., 0.], vec![u, 1., 1.], vec![u, 0., 2.]],
            None,
        )
        .unwrap()
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_preserves_guides_and_refuses_partial_tangent_fields() {
        let request = value_codec::json!({"op":"surface_guided_loft",
            "curves":[section(0.),section(2.)],"parameters":[0.,1.],
            "guides":[guide(0.5)],"guide_parameters":[0.5]});
        let s: Surface =
            value_codec::from_value(crate::transport::dispatch(request.clone()).unwrap()).unwrap();
        assert!((s.evaluate(0.5, 0.5).unwrap().point[1] - 0.5).abs() < 1e-10);
        let mut bad = request;
        bad["start_tangents"] = value_codec::json!([[0., 0., 2.], [0., 0., 2.]]);
        assert!(crate::transport::dispatch(bad).is_err());
    }
    #[test]
    fn rational_sections_and_full_guides_remain_on_independent_cylinder() {
        let a = crate::primitives::circle_arc([0., 0., 0.], [0., 0., 1.], 2., 0., 90.).unwrap();
        let sections: Vec<_> = [0., 1., 2.]
            .into_iter()
            .map(|z| {
                let mut c = a.clone();
                for p in &mut c.control_points {
                    p[2] = z;
                }
                c
            })
            .collect();
        let guides: Vec<_> = [0, sections[0].control_points.len() - 1]
            .into_iter()
            .map(|i| {
                let p = &sections[0].control_points[i];
                crate::primitives::line([p[0], p[1], 0.], [p[0], p[1], 2.]).unwrap()
            })
            .collect();
        let s = interpolate(&sections, &[0., 0.5, 1.], &guides, &[0., 1.], None).unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[0] * p[0] + p[1] * p[1] - 4.).abs() < 1e-9);
                assert!((p[2] - 2. * v).abs() < 1e-9);
            }
        }
    }
    #[test]
    fn multiple_interior_guides_preserve_unconstrained_boundaries() {
        let s = interpolate(
            &[section(0.), section(2.)],
            &[0., 1.],
            &[guide(0.25), guide(0.75)],
            &[0.25, 0.75],
            None,
        )
        .unwrap();
        for v in [0., 0.17, 0.5, 0.83, 1.] {
            for (u, factor) in [
                (0., 0.),
                (0.125, 0.546875),
                (0.25, 1.),
                (0.5, 1.375),
                (0.75, 1.),
                (1., 0.),
            ] {
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[0] - u).abs() < 1e-10);
                assert!((p[1] - 2. * v * (1. - v) * factor).abs() < 1e-10);
                assert!((p[2] - 2. * v).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn single_interior_guide_matches_independent_quadratic_everywhere() {
        let s = interpolate(
            &[section(0.), section(2.)],
            &[2., 7.],
            &[guide(0.5)],
            &[0.5],
            None,
        )
        .unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                let q = [u, 2. * v * (1. - v), 2. * v];
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-10);
                }
            }
        }
    }
    #[test]
    fn guides_and_tangent_controls_are_preserved_together() {
        let t = [vec![[0., 2., 2.]; 2], vec![[0., -2., 2.]; 2]];
        let s = interpolate(
            &[section(0.), section(2.)],
            &[0., 1.],
            &[guide(0.), guide(1.)],
            &[0., 1.],
            Some(t),
        )
        .unwrap();
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            for (v, y) in [(0., 2.), (1., -2.)] {
                let (_, dv) = s.evaluate(u, v).unwrap().first_derivatives().unwrap();
                assert!((dv[1] - y).abs() < 1e-10 && (dv[2] - 2.).abs() < 1e-10);
            }
            for v in [0.13, 0.5, 0.87] {
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[1] - 2. * v * (1. - v)).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn refuses_crossing_and_tangent_conflicts_and_invalid_guide_stations() {
        let sections = [section(0.), section(2.)];
        assert!(interpolate(&sections, &[0., 1.], &[guide(0.5)], &[0.4], None).is_err());
        assert!(interpolate(&sections, &[0., 1.], &[guide(0.5)], &[f64::NAN], None).is_err());
        assert!(
            interpolate(
                &sections,
                &[0., 1.],
                &[guide(0.5)],
                &[0.5],
                Some(std::array::from_fn(|_| vec![[0., 0., 2.]; 2]))
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod cartesian_tests {
    use super::*;
    #[test]
    fn independent_guide_weights_and_missing_boundaries_are_retained() {
        let sections = [
            crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap(),
            crate::primitives::line([0., 0., 1.], [1., 0., 1.]).unwrap(),
        ];
        let mut guide = crate::primitives::line([0.5, 0., 0.], [0.5, 0., 1.]).unwrap();
        guide.weights = vec![1., 2.];
        assert!(interpolate(&sections, &[0., 1.], &[guide.clone()], &[0.5], None).is_err());
        let (surface, certificates) = interpolate_cartesian(
            &sections,
            &[0., 1.],
            &[guide.clone()],
            &[0.5],
            1e-6,
            50000,
            200000,
        )
        .unwrap();
        assert_eq!(certificates.len(), 5);
        for sample in 0..=100 {
            let t = sample as f64 / 100.;
            let actual = surface.evaluate(0.5, t).unwrap().point;
            let expected = guide.evaluate(t).unwrap().point;
            for k in 0..3 {
                assert!((actual[k] - expected[k]).abs() < 1e-11);
            }
        }
        assert!(
            interpolate_cartesian(&sections, &[0., 1.], &[guide], &[0.5], 1e-6, 50000, 1).is_err()
        );
    }
}

#[cfg(test)]
mod clamped_cartesian_tests {
    use super::*;
    #[test]
    fn clamped_guided_loft_retains_missing_boundaries_and_tangents() {
        let sections = [
            crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap(),
            crate::primitives::line([0., 0., 1.], [1., 0., 1.]).unwrap(),
        ];
        let mut guide = crate::primitives::line([0.5, 0., 0.], [0.5, 0., 1.]).unwrap();
        guide.weights = vec![1., 2.];
        let field = |z| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., z]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let targets = [field(2.), field(0.5)];
        let (surface, curves, tangents) = interpolate_cartesian_with_tangents(
            &sections,
            &[2., 7.],
            &[guide.clone()],
            &[0.5],
            &targets,
            1e-6,
            50000,
            200000,
        )
        .unwrap();
        assert_eq!(curves.len(), 5);
        assert_eq!(tangents.len(), 2);
        for sample in 0..=100 {
            let t = sample as f64 / 100.;
            let actual = surface.evaluate(0.5, t).unwrap().point;
            let expected = guide.evaluate(t).unwrap().point;
            for k in 0..3 {
                assert!((actual[k] - expected[k]).abs() < 1e-11);
            }
            for end in 0..2 {
                // Stored C0 U knots intentionally withhold a two-sided full jet.
                // Both neighboring samples and the full-domain tangent certificate are checked.
                for u in [t.next_down().max(0.), t.next_up().min(1.)] {
                    let derivative = surface
                        .evaluate(u, end as f64)
                        .unwrap()
                        .first_derivatives()
                        .unwrap()
                        .1;
                    let expected = targets[end].evaluate(u).unwrap().point;
                    for k in 0..3 {
                        assert!((derivative[k] - expected[k]).abs() < 1e-10);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod authored_cartesian_tests {
    use super::*;
    #[test]
    fn authored_units_retain_varying_rational_fields_and_refuse_exhaustion() {
        let mut first = crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap();
        first.weights = vec![1., 2.];
        let mut last = first.clone();
        for point in &mut last.control_points {
            point[2] = 1.;
        }
        last.weights = vec![3., 1.];
        let guide = crate::primitives::line([0., 0., 0.], [0., 0., 1.]).unwrap();
        let controls = [
            vec![[0., 0., 0.2], [0., 0., 0.4]],
            vec![[0., 0., 0.2], [0., 0., 0.1]],
        ];
        let sections = [first, last];
        let (surface, _, certificates) = interpolate_cartesian_with_control_tangents(
            &sections,
            &[2., 7.],
            &[guide.clone()],
            &[0.],
            &controls,
            1e-6,
            50000,
            200000,
        )
        .unwrap();
        assert!(
            certificates
                .iter()
                .all(|c| c["accepted"] == value_codec::json!(true)
                    && c["targetUnits"] == value_codec::json!("authored-dP/dt"))
        );
        let (authored, _) =
            authored_control_tangent_fields(&sections, &[2., 7.], &controls).unwrap();
        for end in 0..2 {
            for i in 0..=100 {
                let u = i as f64 / 100.;
                let expected = authored[end].evaluate(u).unwrap().point;
                let (_, dv) = surface
                    .evaluate(u, end as f64)
                    .unwrap()
                    .first_derivatives()
                    .unwrap();
                for k in 0..3 {
                    assert!((dv[k] / 5. - expected[k]).abs() < 1e-10);
                }
            }
        }
        assert!(
            interpolate_cartesian_with_control_tangents(
                &sections,
                &[2., 7.],
                &[guide],
                &[0.],
                &controls,
                1e-6,
                50000,
                1
            )
            .is_err()
        );
    }
    #[test]
    fn automatic_authored_units_survive_reversed_guide() {
        let sections = [
            crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap(),
            crate::primitives::line([0., 0., 1.], [1., 0., 1.]).unwrap(),
        ];
        let mut guide = crate::primitives::line([0.5, 0., 1.], [0.5, 0., 0.]).unwrap();
        guide.weights = vec![3., 1.];
        let controls = [vec![[0., 0., 0.6]; 2], vec![[0., 0., 1. / 15.]; 2]];
        let result = crate::loft_alignment::interpolate_cartesian_with_control_tangents(
            &sections,
            &[2., 7.],
            &[guide],
            &controls,
            1e-6,
            1e-8,
            50000,
            200000,
        )
        .unwrap();
        let mut conflicting = controls.clone();
        conflicting[0][0][2] = 0.2;
        let forward = crate::primitives::line([0.5, 0., 0.], [0.5, 0., 1.]).unwrap();
        assert!(
            interpolate_cartesian_with_control_tangents(
                &sections,
                &[2., 7.],
                &[forward],
                &[0.5],
                &conflicting,
                1e-6,
                50000,
                200000
            )
            .is_err()
        );
        assert_eq!(result.reversed, vec![true]);
        assert!(
            result
                .tangent_certificates
                .iter()
                .all(|c| c["accepted"] == value_codec::json!(true))
        );
    }
}
