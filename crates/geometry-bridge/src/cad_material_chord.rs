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
    let r = brep_core::material_chord::inspect(&model, origin, direction, tolerance_uv, limits)?;
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
        "normalAlignment":"not-qualified","minimumWallThickness":"not-qualified",
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
}
