//! Authored rational parameter maps for complete loft sections.
use crate::{Result, check, curve::Curve, surface::Surface};
use value_codec::{Value, json};

pub struct PreparedSections {
    pub curves: Vec<Curve>,
    pub certificates: Vec<Value>,
}

fn full_cover(mapping: &Value, depth: usize) -> Result<()> {
    check(depth <= 8, "Loft section mapping nesting exceeds 8")?;
    if let Some(parts) = mapping.get("composition").and_then(Value::as_array) {
        check(
            !parts.is_empty(),
            "Loft section mapping composition is empty",
        )?;
        for part in parts {
            full_cover(part, depth + 1)?;
        }
    } else {
        let pieces = mapping["pieces"]
            .as_array()
            .ok_or_else(|| crate::input("Loft section mapping needs pieces"))?;
        let first = pieces
            .first()
            .ok_or_else(|| crate::input("Empty loft section mapping"))?;
        let last = pieces.last().unwrap();
        check(
            first["domain"][0].as_f64() == Some(0.)
                && last["domain"][1].as_f64() == Some(1.)
                && first["range"][0].as_f64() == Some(0.)
                && last["range"][1].as_f64() == Some(1.),
            "Loft section mapping must cover the complete normalized domain and range",
        )?;
    }
    Ok(())
}

/// Each map takes common loft U to the corresponding normalized source U.
/// Null entries retain the source parameterization. Strict monotonicity and
/// complete coverage are mandatory; cropping and fitting are not used.
/// Materialization retains the foundation resource limits and requires a
/// whole-domain numerical retention certificate within 1e-6 model units.
pub fn prepare(sections: &[Curve], mappings: &[Option<Value>]) -> Result<PreparedSections> {
    prepare_with_budget(sections, mappings, 1e-6, 50_000, 200_000)
}

pub fn prepare_with_budget(
    sections: &[Curve],
    mappings: &[Option<Value>],
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<PreparedSections> {
    check(
        tolerance.is_finite()
            && tolerance > 0.
            && (1..=1_000_000).contains(&max_cells)
            && (1..=1_000_000).contains(&max_map_evaluations),
        "Invalid mapped section audit budget",
    )?;
    check(
        sections.len() == mappings.len(),
        "Loft needs one mapping entry per section",
    )?;
    let mut curves = Vec::with_capacity(sections.len());
    let mut certificates = Vec::with_capacity(sections.len());
    for (section, mapping) in sections.iter().zip(mappings) {
        let source = crate::gordon::normalized(section)?;
        if let Some(mapping) = mapping {
            full_cover(mapping, 0)?;
            let result = crate::foundation::materialize_reparameterized_curve_bounded(
                &source,
                mapping,
                tolerance,
                max_cells,
                max_map_evaluations,
            )?;
            let curve: Curve = value_codec::from_value(result["curve"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            check(
                curve.domain() == [0., 1.],
                "Mapped loft section domain must be [0,1]",
            )?;
            // Compare with the authored definition as well, including the
            // normalization trim and knot remapping performed above.
            let domain = section.domain();
            let reference_mapping = if domain == [0., 1.] {
                mapping.clone()
            } else {
                let affine = json!({"pieces":[{"domain":[0.,1.],"range":domain,
                    "controlValues":domain,"weights":[1.,1.]}]});
                let mut factors = mapping
                    .get("composition")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_else(|| vec![mapping.clone()]);
                factors.push(affine);
                json!({"composition":factors})
            };
            let retained = crate::foundation::certify_reparameterized_curve_retention(
                section,
                &reference_mapping,
                &curve,
                tolerance,
                max_cells,
                max_map_evaluations,
            )?;
            if retained["certificate"]["accepted"].as_bool() != Some(true) {
                return Err(crate::numeric_err(
                    "Mapped section failed original-definition retention",
                ));
            }
            let mut certificate = result["certificate"].clone();
            certificate["normalizedSourceRetention"] = certificate["retention"].clone();
            certificate["retention"] = retained["certificate"].clone();
            curves.push(curve);
            certificates.push(certificate);
        } else {
            curves.push(source);
            certificates.push(json!({"operation":"identity-section-mapping","domain":[0.,1.]}));
        }
    }
    Ok(PreparedSections {
        curves,
        certificates,
    })
}

/// Audit the final loft against original authored section compositions, rather
/// than adding separately established intermediate construction bounds.
pub fn certify_final_sections(
    surface: &Surface,
    sections: &[Curve],
    parameters: &[f64],
    mappings: &[Option<Value>],
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<Vec<Value>> {
    check(
        sections.len() >= 2 && mappings.len() == sections.len(),
        "Final loft retention needs matching original sections and maps",
    )?;
    surface.validate()?;
    check(
        surface.knots_u[surface.degree_u] == 0.
            && surface.knots_u[surface.control_points.len()] == 1.
            && surface.knots_v[surface.degree_v] == 0.
            && surface.knots_v[surface.control_points[0].len()] == 1.,
        "Final loft retention needs normalized surface domains",
    )?;
    let stations = crate::gordon::stations(parameters, sections.len())?;
    sections.iter().zip(mappings).zip(stations).map(|((section, mapping), station)| {
        section.validate()?;
        let domain = section.domain();
        let affine = json!({"pieces":[{"domain":[0.,1.],"range":domain,
            "controlValues":domain,"weights":[1.,1.]}]});
        let reference = match mapping {
            None => affine,
            Some(mapping) if domain == [0.,1.] => mapping.clone(),
            Some(mapping) => {
                let mut factors = mapping.get("composition").and_then(Value::as_array)
                    .cloned().unwrap_or_else(|| vec![mapping.clone()]);
                factors.push(affine);
                json!({"composition":factors})
            }
        };
        let retained = crate::foundation::certify_reparameterized_surface_section_retention(section,
            &reference, surface, station,
            tolerance, max_cells, max_map_evaluations)?;
        let mut certificate = retained["certificate"].clone();
        crate::numeric(certificate["accepted"].as_bool() == Some(true),
            "Final loft could not retain the original composed section within the requested budget")?;
        certificate["operation"] = json!("original-composed-loft-section-retention");
        certificate["sectionStation"] = json!(station);
        Ok(certificate)
    }).collect()
}

pub fn interpolate(
    sections: &[Curve],
    parameters: &[f64],
    mappings: &[Option<Value>],
) -> Result<Surface> {
    let prepared = prepare(sections, mappings)?;
    let surface = crate::natural_loft::interpolate(&prepared.curves, parameters)?;
    certify_final_sections(
        &surface, sections, parameters, mappings, 1e-6, 50_000, 200_000,
    )?;
    Ok(surface)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sections() -> Vec<Curve> {
        [0., 2.]
            .into_iter()
            .map(|z| crate::primitives::line([0., 0., z], [1., 0., z]).unwrap())
            .collect()
    }
    fn mapping(weight: f64) -> Value {
        json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],
            "controlValues":[0.,0.2,1.],"weights":[1.,weight,1.]}]})
    }
    #[test]
    fn final_loft_audit_uses_original_tensor_not_a_rounded_isocurve() {
        let x = (1_u64 << 29) as f64;
        let station = 1e-8;
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![x, 0., 0.], vec![x + 1., 0., 1.]],
                vec![vec![x, 1., 0.], vec![x + 1., 1., 1.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let rounded = [
            surface.iso(crate::surface::Axis::V, 0.).unwrap(),
            surface.iso(crate::surface::Axis::V, station).unwrap(),
            surface.iso(crate::surface::Axis::V, 1.).unwrap(),
        ];
        assert_eq!(rounded[1].control_points[0][0], x);
        assert!(station > 1e-10);
        assert!(
            certify_final_sections(
                &surface,
                &rounded,
                &[0., station, 1.],
                &[None, None, None],
                1e-10,
                50000,
                200000
            )
            .is_err()
        );
        let originals = [
            crate::primitives::line([x, 0., 0.], [x, 1., 0.]).unwrap(),
            crate::primitives::line([x + 1., 0., 1.], [x + 1., 1., 1.]).unwrap(),
        ];
        let certificates = certify_final_sections(
            &surface,
            &originals,
            &[0., 1.],
            &[None, None],
            1e-10,
            50000,
            200000,
        )
        .unwrap();
        assert!(
            certificates
                .iter()
                .all(|c| c["targetAuthority"] == json!("original-rational-tensor-controls"))
        );
    }
    #[test]
    fn final_loft_audit_retains_original_nonunit_composition_and_refuses_damage() {
        let mut source = sections();
        for section in &mut source {
            section.knots = vec![2., 2., 7., 7.];
        }
        let mappings = vec![
            None,
            Some(json!({"composition":[mapping(0.75),mapping(1.)]})),
        ];
        let surface = interpolate(&source, &[2., 7.], &mappings).unwrap();
        let certificates =
            certify_final_sections(&surface, &source, &[2., 7.], &mappings, 1e-6, 50000, 200000)
                .unwrap();
        assert!(
            certificates
                .iter()
                .all(|c| c["accepted"].as_bool() == Some(true)
                    && c["errorUpper"].as_f64().unwrap() <= 1e-6)
        );
        let mut damaged = surface.clone();
        let nv = damaged.control_points[0].len();
        damaged.control_points[1][nv - 1][1] += 0.1;
        assert!(
            certify_final_sections(&damaged, &source, &[2., 7.], &mappings, 1e-6, 256, 200000)
                .is_err()
        );
        assert!(
            certify_final_sections(&surface, &source, &[2., 7.], &mappings, 1e-6, 50000, 1)
                .is_err()
        );
    }
    #[test]
    fn rational_mapping_retains_the_complete_section_and_independent_parameter_formula() {
        let sections = sections();
        let surface = interpolate(&sections, &[0., 1.], &[None, Some(mapping(0.75))]).unwrap();
        for i in 0..=100 {
            let u = i as f64 / 100.;
            let mapped =
                (0.3 * u * (1. - u) + u * u) / ((1. - u).powi(2) + 1.5 * u * (1. - u) + u * u);
            let p = surface.evaluate(u, 1.).unwrap().point;
            assert!((p[0] - mapped).abs() < 1e-12);
            assert!((p[2] - 2.).abs() < 1e-12);
            assert!((surface.evaluate(u, 0.).unwrap().point[0] - u).abs() < 1e-12);
        }
    }
    #[test]
    fn mapped_sections_accept_a_guide_that_crossed_different_source_parameters() {
        let sections = sections();
        let guide = crate::primitives::line([0.5, 0., 0.], [0.35, 0., 2.]).unwrap();
        assert!(
            crate::loft_alignment::interpolate(&sections, &[0., 1.], &[guide.clone()], 1e-6)
                .is_err()
        );
        let prepared = prepare(&sections, &[None, Some(mapping(1.))]).unwrap();
        let result =
            crate::loft_alignment::interpolate(&prepared.curves, &[0., 1.], &[guide], 1e-6)
                .unwrap();
        for i in 0..=20 {
            let v = i as f64 / 20.;
            let p = result.surface.evaluate(0.5, v).unwrap().point;
            assert!((p[0] - (0.5 - 0.15 * v)).abs() < 1e-7);
            assert!((p[2] - 2. * v).abs() < 1e-7);
        }
    }
    #[test]
    fn partial_nonmonotone_and_mismatched_maps_refuse() {
        let sections = sections();
        assert!(prepare(&sections, &[None]).is_err());
        let partial = json!({"pieces":[{"domain":[0.,1.],"range":[0.1,1.],
            "controlValues":[0.1,1.],"weights":[1.,1.]}]});
        assert!(prepare(&sections, &[None, Some(partial)]).is_err());
        let reversing = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],
            "controlValues":[0.,2.,1.],"weights":[1.,1.,1.]}]});
        assert!(prepare(&sections, &[None, Some(reversing)]).is_err());
    }
    #[test]
    fn piecewise_rational_maps_preserve_each_piece_with_different_endpoint_scales() {
        let mapping = json!({"pieces":[
            {"domain":[0.,0.5],"range":[0.,0.5],"controlValues":[0.,0.5],"weights":[1.,2.]},
            {"domain":[0.5,1.],"range":[0.5,1.],"controlValues":[0.5,1.],"weights":[3.,1.]}
        ]});
        let prepared = prepare(&sections(), &[None, Some(mapping)]).unwrap();
        for i in 0..=100 {
            let u = i as f64 / 100.;
            let expected = if u <= 0.5 {
                let t = 2. * u;
                t / (1. + t)
            } else {
                let t = 2. * u - 1.;
                (1.5 * (1. - t) + t) / (3. * (1. - t) + t)
            };
            assert!((prepared.curves[1].evaluate(u).unwrap().point[0] - expected).abs() < 1e-12);
        }
    }
    #[test]
    fn affine_maps_preserve_multispan_source_geometry() {
        let source = crate::natural_spline::interpolate(
            &[[0., 0., 0.], [1., 2., 0.], [2., -1., 0.], [3., 0., 0.]],
            &[0., 0.25, 0.75, 1.],
        )
        .unwrap();
        let map = json!({"pieces":[
            {"domain":[0.,0.5],"range":[0.,0.25],"controlValues":[0.,0.25],"weights":[1.,1.]},
            {"domain":[0.5,1.],"range":[0.25,1.],"controlValues":[0.25,1.],"weights":[1.,1.]}
        ]});
        let prepared = prepare(&[source.clone()], &[Some(map)]).unwrap();
        for i in 0..=100 {
            let u = i as f64 / 100.;
            let t = if u <= 0.5 {
                u / 2.
            } else {
                0.25 + (u - 0.5) * 1.5
            };
            let p = source.evaluate(t).unwrap().point;
            let q = prepared.curves[0].evaluate(u).unwrap().point;
            for k in 0..3 {
                assert!((p[k] - q[k]).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn nested_noncommuting_maps_match_authored_order_and_direct_evaluator() {
        let first = mapping(1.);
        let second = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],
            "controlValues":[0.,0.7,1.],"weights":[1.,1.,1.]}]});
        let nested = json!({"composition":[first,second]});
        let source = sections()[1].clone();
        let prepared = prepare(&[source.clone()], &[Some(nested.clone())]).unwrap();
        for i in 0..=100 {
            let u = i as f64 / 100.;
            let first = 0.4 * u + 0.6 * u * u;
            let expected = 1.4 * first - 0.4 * first * first;
            let direct =
                crate::foundation::evaluate_reparameterized_curve(&source, &nested, u, None)
                    .unwrap();
            assert!((direct["sourceParameter"].as_f64().unwrap() - expected).abs() < 1e-12);
            assert!((prepared.curves[0].evaluate(u).unwrap().point[0] - expected).abs() < 1e-12);
        }
    }
    #[test]
    fn composed_piecewise_maps_certify_full_extents_instead_of_individual_piece_ranges() {
        let map = json!({"pieces":[
            {"domain":[0.,0.5],"range":[0.,0.25],"controlValues":[0.,0.25],"weights":[1.,1.]},
            {"domain":[0.5,1.],"range":[0.25,1.],"controlValues":[0.25,1.],"weights":[1.,1.]}
        ]});
        let outer = json!({"composition":[map.clone(),json!({"composition":[map]})]});
        let source = sections()[1].clone();
        let prepared = prepare(&[source.clone()], &[Some(outer.clone())]).unwrap();
        for i in 0..=100 {
            let u = i as f64 / 100.;
            let direct =
                crate::foundation::evaluate_reparameterized_curve(&source, &outer, u, None)
                    .unwrap();
            assert!(
                (prepared.curves[0].evaluate(u).unwrap().point[0]
                    - direct["sourceParameter"].as_f64().unwrap())
                .abs()
                    < 1e-12
            );
        }
    }
    #[test]
    fn nonlinear_maps_cross_interior_section_knots_with_whole_retention() {
        let section = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 1., 0.], vec![1., 0., 0.]],
            weights: vec![1., 0.75, 1.],
            periodic: false,
        };
        let map = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,0.25,1.],"weights":[1.,1.,1.]}]});
        let prepared = prepare(&[section.clone()], &[Some(map)]).unwrap();
        let upper = prepared.certificates[0]["retention"]["errorUpper"]
            .as_f64()
            .unwrap();
        assert!(upper <= 1e-6);
        for i in 0..=1000 {
            let u = i as f64 / 1000.;
            let expected = section.evaluate((u + u * u) * 0.5).unwrap().point;
            let actual = prepared.curves[0].evaluate(u).unwrap().point;
            let distance = expected
                .iter()
                .zip(actual)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(distance <= upper);
        }
    }
    #[test]
    fn retention_includes_original_nonunit_section_normalization() {
        let section = Curve {
            degree: 1,
            knots: vec![-3., -3., 2., 7., 7.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 1., 0.], vec![1., 0., 0.]],
            weights: vec![1., 0.75, 1.],
            periodic: false,
        };
        let map = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,0.25,1.],"weights":[1.,1.,1.]}]});
        let prepared = prepare(&[section.clone()], &[Some(map)]).unwrap();
        let upper = prepared.certificates[0]["retention"]["errorUpper"]
            .as_f64()
            .unwrap();
        for i in 0..=1000 {
            let u = i as f64 / 1000.;
            let expected = section
                .evaluate(-3. + 10. * (u + u * u) * 0.5)
                .unwrap()
                .point;
            let actual = prepared.curves[0].evaluate(u).unwrap().point;
            let distance = expected
                .iter()
                .zip(actual)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(distance <= upper);
        }
        assert_eq!(
            prepared.certificates[0]["retention"]["mapCertificate"]["range"],
            json!([-3., 7.])
        );
    }
    #[test]
    fn nonlinear_degree_nine_map_uses_the_actual_composed_degree_budget() {
        let controls = (0..=9).map(|i| (i as f64 / 9.).powi(2)).collect::<Vec<_>>();
        let map = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],
            "controlValues":controls,"weights":vec![1.;10]}]});
        let source = sections()[0].clone();
        let prepared = prepare(&[source.clone()], &[Some(map.clone())]).unwrap();
        assert_eq!(prepared.curves[0].degree, 9);
        for i in 0..=100 {
            let u = i as f64 / 100.;
            let expected = u * u + u * (1. - u) / 9.;
            assert!((prepared.curves[0].evaluate(u).unwrap().point[0] - expected).abs() < 1e-11);
        }
        let elevated = source.elevate(3).unwrap();
        let error = prepare(&[elevated], &[Some(map)]).err().unwrap();
        assert_eq!(error.code, "NURBS_RESOURCE_LIMIT");
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_loft_routes_section_maps_and_preserves_existing_unmapped_calls() {
        let request = json!({"op":"surface_natural_loft","curves":sections(),
            "parameters":[0.,1.],"section_mappings":[Value::Null,mapping(0.75)]});
        let mapped: Surface = value_codec::from_value(crate::dispatch(request).unwrap()).unwrap();
        assert!((mapped.evaluate(0.5, 1.).unwrap().point[0] - 0.325 / 0.875).abs() < 1e-12);
        let request = json!({"op":"surface_auto_guided_loft","curves":sections(),
            "parameters":[0.,1.],"section_mappings":[Value::Null,mapping(1.)],"budget":1e-6,
            "guides":[crate::primitives::line([0.5,0.,0.],[0.35,0.,2.]).unwrap()]});
        let result = crate::dispatch(request).unwrap();
        assert_eq!(result["sections"].as_array().unwrap().len(), 2);
        assert_eq!(
            result["section_mapping_certificates"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
