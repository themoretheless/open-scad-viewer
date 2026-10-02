//! Bounded automatic guide intersection, orientation and piecewise station mapping.
use crate::{
    Result, check,
    curve::Curve,
    surface::{Axis, Surface},
};
pub struct AlignedLoft {
    pub surface: Surface,
    pub guides: Vec<Curve>,
    pub guide_parameters: Vec<f64>,
    pub guide_order: Vec<usize>,
    pub reversed: Vec<bool>,
    pub section_error_upper: Vec<f64>,
    pub guide_error_upper: Vec<f64>,
    pub curve_certificates: Vec<value_codec::Value>,
}
fn curve_error(a: &Curve, b: &Curve) -> Result<f64> {
    let a = crate::gordon::normalized(a)?;
    let b = crate::gordon::normalized(b)?;
    crate::continuity::deviation::positional_upper(
        &crate::surface::loft(&[a.clone(), a])?,
        &crate::surface::loft(&[b.clone(), b])?,
    )
}
fn remap(c: &Curve, source: &[f64], target: &[f64]) -> Result<(Curve, f64)> {
    let mut result: Option<Curve> = None;
    for (s, t) in source.windows(2).zip(target.windows(2)) {
        let mut part = c.trim(s[0], s[1])?;
        for k in &mut part.knots {
            *k = t[0] + (*k - s[0]) / (s[1] - s[0]) * (t[1] - t[0]);
        }
        for k in part.knots.iter_mut().take(part.degree + 1) {
            *k = t[0];
        }
        let n = part.knots.len();
        for k in &mut part.knots[n - part.degree - 1..] {
            *k = t[1];
        }
        if let Some(r) = &mut result {
            // Independent trims can round the same endpoint differently.
            // Retain one shared control and one homogeneous scale; the
            // whole-curve mapping bounds below include every resulting error.
            let factor = r.weights.last().unwrap() / part.weights[0];
            check(
                factor.is_finite() && factor > 0.,
                "Guide remapping scale overflow",
            )?;
            for weight in &mut part.weights {
                *weight *= factor;
            }
            part.validate()?;
            r.knots.pop();
            r.knots.extend_from_slice(&part.knots[part.degree + 1..]);
            r.control_points
                .extend_from_slice(&part.control_points[1..]);
            r.weights.extend_from_slice(&part.weights[1..]);
        } else {
            result = Some(part);
        }
    }
    let result = result.ok_or_else(|| crate::input("Guide remapping needs two crossings"))?;
    result.validate()?;
    let mut error = 0_f64;
    for (s, t) in source.windows(2).zip(target.windows(2)) {
        error = error.max(curve_error(
            &c.trim(s[0], s[1])?,
            &result.trim(t[0], t[1])?,
        )?);
    }
    Ok((result, error))
}
/// Discovers one isolated point crossing per guide/section pair. Overlaps,
/// multiple solutions, unresolved intersection boxes and inconsistent U stations
/// are refused. Guide portions outside the first/last section are trimmed.
/// Piecewise affine V mapping retains the guide locus but can introduce C0 knots.
/// Returned section/guide errors are whole-curve outward bounds within budget.
pub fn interpolate(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    budget: f64,
) -> Result<AlignedLoft> {
    interpolate_with_parameter_tolerance(sections, parameters, guides, budget, 1e-8)
}

/// Uses a separate dimensionless U-station tolerance; the spatial budget is in mm.
pub fn interpolate_with_parameter_tolerance(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    budget: f64,
    parameter_tolerance: f64,
) -> Result<AlignedLoft> {
    interpolate_mode(
        sections,
        parameters,
        guides,
        budget,
        parameter_tolerance,
        None,
    )
}

/// Automatic orientation/station mapping with independent rational guide weights.
/// Curve audit budgets apply per curve; returned bounds include remapping error.
pub fn interpolate_cartesian_with_parameter_tolerance(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    budget: f64,
    parameter_tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<AlignedLoft> {
    interpolate_mode(
        sections,
        parameters,
        guides,
        budget,
        parameter_tolerance,
        Some((max_cells, max_map_evaluations)),
    )
}

fn interpolate_mode(
    sections: &[Curve],
    parameters: &[f64],
    guides: &[Curve],
    budget: f64,
    parameter_tolerance: f64,
    cartesian: Option<(usize, usize)>,
) -> Result<AlignedLoft> {
    if let Some((cells, evaluations)) = cartesian {
        check(
            (1..=1_000_000).contains(&cells) && (1..=1_000_000).contains(&evaluations),
            "Automatic Cartesian loft audit budgets must be in 1..1000000",
        )?;
    }
    check(
        parameter_tolerance.is_finite() && parameter_tolerance > 0. && parameter_tolerance <= 1.,
        "Automatic loft parameter tolerance must be finite and in (0,1]",
    )?;
    check(
        budget.is_finite() && budget > 0.,
        "Automatic loft alignment needs a positive finite budget",
    )?;
    check(
        (2..=86).contains(&sections.len()) && (1..=84).contains(&guides.len()),
        "Automatic loft alignment needs 2..86 sections and 1..84 guides",
    )?;
    let stations = crate::gordon::stations(parameters, sections.len())?;
    let sections = sections
        .iter()
        .map(crate::gordon::normalized)
        .collect::<Result<Vec<_>>>()?;
    let mut entries = Vec::new();
    for (index, g) in guides.iter().enumerate() {
        let mut guide = crate::gordon::normalized(g)?;
        let mut crossings = Vec::new();
        let mut us = Vec::new();
        for section in &sections {
            let r = crate::intersection::intersect_curve_curve(section, &guide, None)?;
            let components = r["components"]
                .as_array()
                .ok_or_else(|| crate::input("Missing intersection components"))?;
            check(
                r["unresolved"].as_array().is_some_and(|v| v.is_empty())
                    && components.len() == 1
                    && components[0]["kind"].as_str() == Some("point"),
                "Guide alignment needs one resolved isolated section crossing",
            )?;
            crossings.push(
                components[0]["second"]
                    .as_f64()
                    .ok_or_else(|| crate::input("Missing guide parameter"))?,
            );
            us.push(
                components[0]["first"]
                    .as_f64()
                    .ok_or_else(|| crate::input("Missing section parameter"))?,
            );
        }
        let reversed = crossings.windows(2).all(|x| x[0] > x[1]);
        if reversed {
            guide = guide.reverse()?;
            for t in &mut crossings {
                *t = 1. - *t;
            }
        }
        check(
            crossings.windows(2).all(|x| x[0] < x[1]),
            "Guide section crossings are not monotone",
        )?;
        let u = us[0];
        check(
            us.iter().all(|x| (*x - u).abs() <= parameter_tolerance),
            "Guide crosses sections at inconsistent U stations",
        )?;
        let (mut guide, mapping_error) = remap(&guide, &crossings, &stations)?;
        let mapping_error = if cartesian.is_some() {
            mapping_error
        } else {
            let before_scaling = guide.clone();
            let sw = crate::gordon::homogeneous(&sections[0], u)?[3];
            let gw = crate::gordon::homogeneous(&guide, 0.)?[3];
            for w in &mut guide.weights {
                *w *= sw / gw;
            }
            crate::intersection::next_up(mapping_error + curve_error(&before_scaling, &guide)?)
        };
        entries.push((u, index, reversed, guide, mapping_error));
    }
    entries.sort_by(|a, b| a.0.total_cmp(&b.0));
    check(
        entries.windows(2).all(|e| e[0].0 < e[1].0),
        "Guides occupy duplicate U stations",
    )?;
    let guide_parameters = entries.iter().map(|e| e.0).collect::<Vec<_>>();
    let guide_order = entries.iter().map(|e| e.1).collect();
    let reversed = entries.iter().map(|e| e.2).collect();
    let mapping_errors = entries.iter().map(|e| e.4).collect::<Vec<_>>();
    let guides = entries.into_iter().map(|e| e.3).collect::<Vec<_>>();
    let (surface, section_error_upper, guide_error_upper, curve_certificates) =
        if let Some((max_cells, max_map_evaluations)) = cartesian {
            let (surface, certificates) = crate::guided_loft::interpolate_cartesian(
                &sections,
                parameters,
                &guides,
                &guide_parameters,
                budget,
                max_cells,
                max_map_evaluations,
            )?;
            let upper = |index: usize| -> Result<f64> {
                certificates[index]["errorUpper"]
                    .as_f64()
                    .ok_or_else(|| crate::numeric_err("Missing Cartesian retention upper bound"))
            };
            let section_errors = (0..sections.len()).map(upper).collect::<Result<Vec<_>>>()?;
            let offset = usize::from(guide_parameters[0] != 0.);
            let guide_errors = mapping_errors
                .iter()
                .enumerate()
                .map(|(i, mapping)| {
                    upper(sections.len() + offset + i)
                        .map(|error| crate::intersection::next_up(mapping + error))
                })
                .collect::<Result<Vec<_>>>()?;
            (surface, section_errors, guide_errors, certificates)
        } else {
            let surface = crate::guided_loft::interpolate_budgeted(
                &sections,
                parameters,
                &guides,
                &guide_parameters,
                None,
                budget,
            )?;
            let section_error_upper = sections
                .iter()
                .zip(stations)
                .map(|(c, v)| curve_error(c, &surface.iso(Axis::V, v)?))
                .collect::<Result<Vec<_>>>()?;
            let guide_error_upper = guides
                .iter()
                .zip(&guide_parameters)
                .zip(mapping_errors)
                .map(|((c, &u), mapping)| {
                    Ok(crate::intersection::next_up(
                        mapping + curve_error(c, &surface.iso(Axis::U, u)?)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            (surface, section_error_upper, guide_error_upper, Vec::new())
        };
    check(
        section_error_upper
            .iter()
            .chain(&guide_error_upper)
            .all(|e| *e <= budget),
        "Automatic loft retention bound exceeds budget",
    )?;
    Ok(AlignedLoft {
        surface,
        guides,
        guide_parameters,
        guide_order,
        reversed,
        section_error_upper,
        guide_error_upper,
        curve_certificates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn section(z: f64) -> Curve {
        crate::primitives::line([0., 0., z], [1., 0., z]).unwrap()
    }
    #[test]
    fn automatic_orientation_order_and_nonlinear_station_mapping() {
        let sections = [section(0.), section(1.), section(2.)];
        let a = crate::primitives::line([0.25, 0., 0.], [0.25, 0., 2.]).unwrap();
        let b = crate::primitives::line([0.75, 0., 2.], [0.75, 0., 0.]).unwrap();
        let result = interpolate(&sections, &[0., 0.25, 1.], &[b, a], 1e-8).unwrap();
        assert_eq!(result.guide_order, vec![1, 0]);
        assert_eq!(result.reversed, vec![false, true]);
        for u in [0., 0.25, 0.5, 0.75, 1.] {
            assert!((result.surface.evaluate(u, 0.25).unwrap().point[2] - 1.).abs() < 1e-8);
        }
    }
    #[test]
    fn spatial_budget_cannot_relax_parameter_correspondence() {
        let sections = [section(0.), section(2.)];
        let diagonal = crate::primitives::line([0.25, 0., 0.], [0.75, 0., 2.]).unwrap();
        let error = interpolate(&sections, &[0., 1.], &[diagonal], 10.)
            .err()
            .unwrap();
        assert!(error.to_string().contains("inconsistent U stations"));
        let guide = crate::primitives::line([0.25, 0., 0.], [0.25, 0., 2.]).unwrap();
        for invalid in [0., -1., f64::NAN, f64::INFINITY, 1.01] {
            assert!(
                interpolate_with_parameter_tolerance(
                    &sections,
                    &[0., 1.],
                    &[guide.clone()],
                    1e-8,
                    invalid
                )
                .is_err()
            );
        }
    }
    #[test]
    fn overlap_and_inconsistent_stations_refuse() {
        let sections = [section(0.), section(2.)];
        assert!(interpolate(&sections, &[0., 1.], &[section(0.)], 1e-8).is_err());
        let diagonal = crate::primitives::line([0.25, 0., 0.], [0.75, 0., 2.]).unwrap();
        assert!(interpolate(&sections, &[0., 1.], &[diagonal], 1e-8).is_err());
    }
}

#[cfg(test)]
mod cartesian_tests {
    use super::*;
    #[test]
    fn automatic_orientation_retains_independent_rational_guide_weights() {
        let sections = [
            crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap(),
            crate::primitives::line([0., 0., 1.], [1., 0., 1.]).unwrap(),
        ];
        let mut guide = crate::primitives::line([0.5, 0., 1.], [0.5, 0., 0.]).unwrap();
        guide.weights = vec![3., 1.];
        assert!(interpolate(&sections, &[0., 1.], &[guide.clone()], 1e-6).is_err());
        let result = interpolate_cartesian_with_parameter_tolerance(
            &sections,
            &[0., 1.],
            &[guide.clone()],
            1e-6,
            1e-8,
            50000,
            200000,
        )
        .unwrap();
        assert_eq!(result.reversed, vec![true]);
        assert_eq!(result.guide_order, vec![0]);
        assert!((result.guide_parameters[0] - 0.5).abs() < 1e-8);
        assert_eq!(result.curve_certificates.len(), 5);
        assert!(
            result
                .section_error_upper
                .iter()
                .chain(&result.guide_error_upper)
                .all(|e| *e <= 1e-6)
        );
        let reference = guide.reverse().unwrap();
        for sample in 0..=100 {
            let v = sample as f64 / 100.;
            let actual = result.surface.evaluate(0.5, v).unwrap().point;
            let expected = reference.evaluate(v).unwrap().point;
            for k in 0..3 {
                assert!((actual[k] - expected[k]).abs() < 1e-11);
            }
        }
    }
}
