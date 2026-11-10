use super::*;

/// Fail-closed analytic Boolean. On success returns a model plus an all-Complete
/// certificate. On refusal the caller's operands are untouched.
pub fn analytic_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, BooleanCertificate)> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Boolean operation must be union, difference, intersection, or xor",
        ));
    }
    let class_a = classify_model(a)?;
    let class_b = classify_model(b)?;
    if class_a == AnalyticClass::AxisAlignedBox && class_b == AnalyticClass::AxisAlignedBox {
        return Err(unsupported(
            "Planar box Boolean must use operations::boolean; analytic_boolean is curved-only",
        ));
    }
    if class_a == AnalyticClass::Sphere && class_b == AnalyticClass::Sphere {
        if operation == "xor" {
            return Err(refuse(
                "Sphere xor is outside the closed matrix walking slice",
            ));
        }
        return sphere_pair_boolean(a, b, operation);
    }
    if matches!(
        (class_a, class_b),
        (AnalyticClass::AxisAlignedBox, AnalyticClass::Sphere)
            | (AnalyticClass::Sphere, AnalyticClass::AxisAlignedBox)
    ) {
        if operation == "xor" {
            return Err(refuse(
                "Box/sphere xor is outside the closed matrix walking slice",
            ));
        }
        return box_sphere_pair_boolean(a, b, operation);
    }

    // Cylinder × cylinder (F1 wall imprint + empty algebra).
    if class_a == AnalyticClass::FiniteCylinder && class_b == AnalyticClass::FiniteCylinder {
        return cylinder_pair_boolean(a, b, operation);
    }

    if matches!(class_a, AnalyticClass::Tube | AnalyticClass::Frustum)
        || matches!(class_b, AnalyticClass::Tube | AnalyticClass::Frustum)
    {
        let (amin, amax) = model_bounds(a);
        let (bmin, bmax) = model_bounds(b);
        let separated = (0..3).any(|i| amax[i] < bmin[i] - 1e-9 || bmax[i] < amin[i] - 1e-9);
        return if separated {
            empty_algebra_pair(
                a,
                b,
                operation,
                SpatialRelation::Disjoint,
                vec![Coverage::Complete],
            )
        } else {
            Err(refuse(
                "Tube/frustum non-empty Boolean contact is outside the frozen matrix",
            ))
        };
    }

    if matches!(class_a, AnalyticClass::Cone | AnalyticClass::Torus)
        || matches!(class_b, AnalyticClass::Cone | AnalyticClass::Torus)
    {
        return Err(refuse(
            "Cone/torus Boolean is query-only and outside the frozen matrix",
        ));
    }

    // Axial cylinder × sphere: exact rings through the sphere-mate engine.
    if matches!(
        (class_a, class_b),
        (AnalyticClass::FiniteCylinder, AnalyticClass::Sphere)
            | (AnalyticClass::Sphere, AnalyticClass::FiniteCylinder)
    ) && operation != "xor"
        && let Some(result) = cylinder_sphere_boolean::boolean(a, b, operation)?
    {
        result.validate()?;
        let classification = classify_chart_events(
            ChartKind::AnalyticCircle,
            vec![],
            &[(0.5, CellLabel::Outside)],
        )?;
        let coverages = vec![Coverage::Complete];
        let cert = certificate_for(operation, coverages, classification, true, &result)?;
        return Ok((result, cert));
    }

    // Mixed cylinder/sphere: Complete empty → empty algebra; else frozen refuse.
    let options = Options::default();
    let report = match (class_a, class_b) {
        (AnalyticClass::FiniteCylinder, AnalyticClass::Sphere)
        | (AnalyticClass::Sphere, AnalyticClass::FiniteCylinder) => {
            let (cyl, sph) = if class_a == AnalyticClass::FiniteCylinder {
                (a, b)
            } else {
                (b, a)
            };
            let (co, cd, cr, ch) = cylinder_envelope(cyl)?;
            let (sc, sr) = sphere_envelope(sph)?;
            cylinder_sphere(co, cd, cr, ch, sc, sr, options)?
        }
        (AnalyticClass::Sphere, AnalyticClass::Sphere) => unreachable!(),
        (AnalyticClass::Cone, AnalyticClass::Cone) => {
            let (aa, ad, ar, ah) = cone_envelope(a)?;
            let (ba, bd, br, bh) = cone_envelope(b)?;
            cone_cone(
                FiniteCone {
                    apex: aa,
                    axis: ad,
                    radius: ar,
                    height: ah,
                },
                FiniteCone {
                    apex: ba,
                    axis: bd,
                    radius: br,
                    height: bh,
                },
                options,
            )?
        }
        (AnalyticClass::Torus, AnalyticClass::Torus) => {
            let (ac, amaj, amin) = torus_envelope(a)?;
            let (bc, bmaj, bmin) = torus_envelope(b)?;
            torus_torus(ac, amaj, amin, bc, bmaj, bmin, options)?
        }
        (AnalyticClass::Cone, _)
        | (_, AnalyticClass::Cone)
        | (AnalyticClass::Torus, _)
        | (_, AnalyticClass::Torus) => {
            let (amin, amax) = model_bounds(a);
            let (bmin, bmax) = model_bounds(b);
            let separated = (0..3).any(|i| amax[i] < bmin[i] - 1e-9 || bmax[i] < amin[i] - 1e-9);
            return if separated {
                empty_algebra_pair(
                    a,
                    b,
                    operation,
                    SpatialRelation::Disjoint,
                    vec![Coverage::Complete],
                )
            } else {
                Err(refuse(
                    "Mixed cone/torus Boolean contact is schema-frozen refuse until dedicated imprint",
                ))
            };
        }
        _ => {
            return Err(unsupported(
                "Analytic Boolean pair is outside the full matrix walking slice",
            ));
        }
    };

    if report.coverage != Coverage::Complete {
        return Err(refuse(
            "Analytic pair contact is not Complete; schema-frozen refuse (no prism authorship)",
        ));
    }
    verify_complete_report(&report, true)?;
    let only_empty = report
        .components
        .iter()
        .all(|c| matches!(c, AnalyticSsComponent::Empty))
        || report.components.is_empty();
    if only_empty {
        return empty_algebra_pair(
            a,
            b,
            operation,
            SpatialRelation::Disjoint,
            vec![Coverage::Complete],
        );
    }
    // Complete non-empty mixed sections remain out of the frozen matrix until
    // their own shared UV split/assembly path is qualified. Complete SS alone
    // must never be reinterpreted as disjoint empty algebra.
    if matches!(
        (class_a, class_b),
        (AnalyticClass::FiniteCylinder, AnalyticClass::Sphere)
            | (AnalyticClass::Sphere, AnalyticClass::FiniteCylinder)
    ) && report
        .components
        .iter()
        .any(|c| matches!(c, AnalyticSsComponent::Circle { .. }))
    {
        return Err(refuse(
            "Sphere×cylinder non-empty contact lacks a qualified shared imprint assembly",
        ));
    }
    Err(refuse(
        "Complete non-empty analytic section without certified imprint for this pair",
    ))
}

/// Typestate-returning production entry point. The compatibility API above
/// remains available, while callers that publish topology can require the
/// global audit certificate in the return type.
pub fn analytic_boolean_audited(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(
    crate::solid_audit::GloballyAuditedSolidSet,
    BooleanCertificate,
)> {
    let (model, certificate) = analytic_boolean(a, b, operation)?;
    let audited = crate::solid_audit::LocallyValidatedModel::new(model)?.audit()?;
    Ok((audited, certificate))
}

pub(super) fn cylinder_pair_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, BooleanCertificate)> {
    let hint = relation_hint(a, b)?;
    if matches!(hint, "tangent" | "coincident") {
        return Err(refuse(
            "Tangent/coincident contacts are not certified for analytic Boolean",
        ));
    }

    let reports = imprint_reports(a, b)?;
    let coverages: Vec<_> = reports.iter().map(|r| r.coverage).collect();
    if coverages.iter().any(|c| *c != Coverage::Complete) {
        return Err(refuse(
            "Intersection reports were not all Complete; refuse topology change",
        ));
    }

    let (classification, uv_arrangement_certified) =
        classification_from_imprint(a, &reports, hint)?;
    let relation = hint_to_relation(hint)?;
    let result = if relation == SpatialRelation::WallIntersect {
        if operation == "xor" {
            return Err(refuse(
                "Cylinder wall-intersect xor is outside the certified walking slice",
            ));
        }
        let (ao, ad, ar, ah) = cylinder_envelope(a)?;
        let (bo, bd, br, bh) = cylinder_envelope(b)?;
        let events = wall_imprint_events(&reports)?;
        imprint_pipeline::parallel_cylinder_wall_imprint(
            a,
            b,
            operation,
            &events,
            imprint_pipeline::CylinderEnvelope {
                origin: ao,
                axis: ad,
                radius: ar,
                height: ah,
            },
            imprint_pipeline::CylinderEnvelope {
                origin: bo,
                axis: bd,
                radius: br,
                height: bh,
            },
        )?
    } else {
        imprint_pipeline::regularized_empty_algebra(a, b, operation, relation)?
    };
    let certificate = certificate_for(
        operation,
        coverages,
        classification,
        uv_arrangement_certified,
        &result,
    )?;
    Ok((result, certificate))
}
