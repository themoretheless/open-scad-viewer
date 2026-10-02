use super::cad_material_segment::{crossing, unresolved};
use super::{Result, Value, field};
use value_codec::json;

pub fn inspect(v: Value) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let origin: [f64; 3] = field(&v, "origin")?;
    let direction: [f64; 3] = field(&v, "direction")?;
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    let config: Value = field(&v, "limits")?;
    let validity_config: Value = field(&config, "validity")?;
    let limits = brep_core::material_segment::Limits {
        volume: super::cad_solid_distance::validity_limits(&validity_config)?,
        point_cells: field(&config, "pointCells")?,
        point_domain_cells: field(&config, "pointDomainCells")?,
        segment_cells: field(&config, "segmentCells")?,
        segment_domain_cells: field(&config, "segmentDomainCells")?,
    };
    let normal_config: Option<Value> = field(&v, "normalAudit")?;
    let (r, normal_evidence, normal_alignment) = if let Some(audit) = normal_config.as_ref() {
        let sine: f64 = field(audit, "maxSineSquared")?;
        let spans: usize = field(audit, "maxSpans")?;
        let nr = brep_core::material_chord::inspect_with_normals(
            &model,
            origin,
            direction,
            tolerance_uv,
            limits,
            sine,
            spans,
        )?;
        let evidence = normal_evidence(&nr);
        let status = match nr.aligned {
            Some(true) => "angular-tolerance",
            Some(false) => "oblique",
            None => "unresolved",
        };
        (nr.chord, Some(evidence), status)
    } else {
        (
            brep_core::material_chord::inspect(&model, origin, direction, tolerance_uv, limits)?,
            None,
            "not-qualified",
        )
    };
    render(
        &model,
        &r,
        origin,
        direction,
        tolerance_uv,
        &config,
        &normal_config,
        normal_evidence,
        normal_alignment,
    )
}
pub(super) fn normal_evidence(nr: &brep_core::material_chord::NormalReport) -> Value {
    json!({"aligned":nr.aligned,"spans":nr.spans,
        "endpoints":nr.endpoints.iter().enumerate().map(|(i,r)|r.as_ref().map(|r|json!({
            "face":nr.chord.boundary.contacts[i].face,"uv":nr.chord.boundary.contacts[i].uv,
            "aligned":r.aligned,"sineSquaredInterval":r.sine_squared_interval,
            "normalComponents":r.normal_components,"spans":r.spans,"reason":r.reason
        }))).collect::<Vec<_>>()})
}
pub(super) fn render(
    model: &brep_core::Model,
    r: &brep_core::material_chord::Report,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    config: &Value,
    normal_config: &Option<Value>,
    normal_evidence: Option<Value>,
    normal_alignment: &str,
) -> Result<Value> {
    let seed = r.seed.as_ref().map(|s| {
        json!({"inside":s.parity,
            "cells":s.cells,"domainCells":s.domain_cells,
            "attempts":s.attempts.iter().map(|a|json!({"inside":a.parity,
                "crossings":a.crossings.iter().map(crossing).collect::<Vec<_>>(),
                "unresolved":a.unresolved.iter().map(unresolved).collect::<Vec<_>>(),
                "cells":a.cells,"domainCells":a.domain_cells})).collect::<Vec<_>>()
        })
    });
    let validity = &r.validity;
    Ok(json!({"method":"continuous-material-chord",
        "scope":"material-between-original-transverse-boundary-roots",
        "normalAlignment":normal_alignment,"normalAudit":normal_config,"normalEvidence":normal_evidence,"minimumWallThickness":"not-qualified",
        "parameterInterval":[0.,1.],"origin":origin,"direction":direction,
        "sourceModel":model,"limits":config,"toleranceUv":tolerance_uv,
        "proven":r.proven,"reason":r.reason,"seed":seed,
        "lengthIntervalMm":r.length_interval_mm,"pointEnclosures":r.point_enclosures,
        "boundary":{"contacts":r.boundary.contacts.iter().map(crossing).collect::<Vec<_>>(),
            "unresolved":r.boundary.unresolved.iter().map(unresolved).collect::<Vec<_>>(),
            "cells":r.boundary.cells,"domainCells":r.boundary.domain_cells},
        "validity":{"proven":validity.proven,"boundaryProven":validity.boundary.proven,
            "exactAgreement":validity.boundary.agreement.all_equal,
            "exactJoins":validity.boundary.agreement.all_joins_exact,
            "trimValid":validity.boundary.trim.all_valid,
            "selfIntersectionAbsent":validity.boundary.intersections.absence_proven,
            "nestingRolesConsistent":validity.nesting.as_ref().and_then(|n|n.roles_consistent),
            "orientations":validity.orientations.iter().map(|o|json!({"shell":o.shell,
                "expectedOutward":o.expected_outward,"outward":o.outward})).collect::<Vec<_>>()}
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dispatch_qualifies_original_boundary_endpoints_and_refuses_extra_crossings() {
        let cube = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
        let cavity = brep_core::operations::boolean(
            &cube,
            &brep_core::cuboid([4.; 3], [6.; 3]).unwrap(),
            "difference",
        )
        .unwrap();
        for (model, expected) in [(cube, true), (cavity, false)] {
            let mut input = super::super::cad_material_segment::tests::request(
                &model,
                [-2., 5., 5.],
                [14., 0., 0.],
            );
            input["op"] = json!("cad_material_chord");
            let r = crate::dispatch(input.clone()).unwrap();
            assert_eq!(r["proven"], json!(expected));
            assert_eq!(r["sourceModel"], input["model"]);
            assert_eq!(r["limits"], input["limits"]);
            assert_eq!(r["normalAlignment"], json!("not-qualified"));
            let length: Option<[f64; 2]> = field(&r, "lengthIntervalMm").unwrap();
            if expected {
                let bounds = length.unwrap();
                assert!(bounds[0] <= 10. && bounds[1] >= 10.);
                assert_eq!(r["seed"]["inside"], json!(false));
            } else {
                assert!(length.is_none());
                assert_eq!(r["pointEnclosures"], Value::Null);
            }
        }
    }
    #[test]
    fn explicit_normal_audit_binds_tolerance_endpoints_and_shared_work() {
        let cube = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
        let mut input =
            super::super::cad_material_segment::tests::request(&cube, [-2., 5., 5.], [14., 0., 0.]);
        input["op"] = json!("cad_material_chord");
        input["normalAudit"] = json!({"maxSineSquared":1e-6,"maxSpans":2});
        let r = crate::dispatch(input.clone()).unwrap();
        assert_eq!(r["proven"], json!(true));
        assert_eq!(r["normalAlignment"], json!("angular-tolerance"));
        assert_eq!(r["normalAudit"], input["normalAudit"]);
        assert_eq!(r["normalEvidence"]["aligned"], json!(true));
        assert_eq!(r["normalEvidence"]["spans"], json!(2));
        input["normalAudit"]["maxSpans"] = json!(1);
        let r = crate::dispatch(input.clone()).unwrap();
        assert_eq!(r["proven"], json!(true));
        assert_eq!(r["normalAlignment"], json!("unresolved"));
        assert_eq!(r["normalEvidence"]["endpoints"][1], Value::Null);
        input["normalAudit"]["maxSpans"] = json!(0);
        assert!(crate::dispatch(input).is_err());
    }
}
