use super::{Result, Value, field};
use value_codec::json;

pub(super) fn crossing(c: &brep_core::ray_parity::Crossing) -> Value {
    json!({"face":c.face,"uv":c.uv,"parameter":c.parameter})
}
pub(super) fn unresolved(c: &brep_core::ray_parity::Unresolved) -> Value {
    json!({"face":c.face,"uv":c.uv,"reason":c.reason})
}
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
    let r = brep_core::material_segment::inspect(&model, origin, direction, tolerance_uv, limits)?;
    let validity = &r.validity;
    let seed = r.seed.as_ref().map(|s| {
        json!({
            "inside":s.parity,"cells":s.cells,"domainCells":s.domain_cells,
            "attempts":s.attempts.iter().map(|a|json!({"inside":a.parity,
                "crossings":a.crossings.iter().map(crossing).collect::<Vec<_>>(),
                "unresolved":a.unresolved.iter().map(unresolved).collect::<Vec<_>>(),
                "cells":a.cells,"domainCells":a.domain_cells})).collect::<Vec<_>>()
        })
    });
    let segment = r.segment.as_ref().map(|s| {
        json!({
            "boundaryFree":s.boundary_free,
            "contacts":s.contacts.iter().map(crossing).collect::<Vec<_>>(),
            "unresolved":s.unresolved.iter().map(unresolved).collect::<Vec<_>>(),
            "cells":s.cells,"domainCells":s.domain_cells
        })
    });
    Ok(json!({"method":"continuous-material-segment",
        "scope":"strict-interior-authored-parametric-segment",
        "parameterInterval":[0.,1.],"origin":origin,"direction":direction,
        "sourceModel":model,"toleranceUv":tolerance_uv,"limits":config,
        "proven":r.proven,"reason":r.reason,"seed":seed,"segment":segment,
        "validity":{"proven":validity.proven,
            "boundaryProven":validity.boundary.proven,
            "exactAgreement":validity.boundary.agreement.all_equal,
            "exactJoins":validity.boundary.agreement.all_joins_exact,
            "trimValid":validity.boundary.trim.all_valid,
            "selfIntersectionAbsent":validity.boundary.intersections.absence_proven,
            "nestingRolesConsistent":validity.nesting.as_ref().and_then(|n|n.roles_consistent),
            "orientations":validity.orientations.iter().map(|o|json!({
                "shell":o.shell,"expectedOutward":o.expected_outward,"outward":o.outward
            })).collect::<Vec<_>>()}
    }))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn request(
        model: &brep_core::Model,
        origin: [f64; 3],
        direction: [f64; 3],
    ) -> Value {
        json!({"op":"cad_material_segment","model":model,"origin":origin,"direction":direction,
            "toleranceUv":1e-8,"limits":{"pointCells":100000,"pointDomainCells":1000000,
            "segmentCells":100000,"segmentDomainCells":1000000,"validity":{
                "exactWork":1000000,"trimPairs":10000,"trimCells":100000,"trimDomainCells":1000000,"spans":4096,
                "facePairs":10000,"faceCells":150000,"faceDomainCells":1500000,
                "faceCellsPerPair":1024,"faceDomainCellsPerPair":100000,
                "nestingPairs":100,"nestingCells":100000,"nestingDomainCells":1000000,
                "orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100}}})
    }
    #[test]
    fn dispatch_preserves_source_and_refuses_cavity_endpoint_and_exhausted_work() {
        let cube = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
        let cavity = brep_core::operations::boolean(
            &cube,
            &brep_core::cuboid([4.; 3], [6.; 3]).unwrap(),
            "difference",
        )
        .unwrap();
        for (model, origin, direction, expected, reason) in [
            (&cube, [2., 5., 5.], [6., 0., 0.], true, "interior-segment"),
            (
                &cavity,
                [2., 5., 5.],
                [6., 0., 0.],
                false,
                "boundary-contact",
            ),
            (&cube, [-2., 5., 5.], [-1., 0., 0.], false, "seed-outside"),
            (
                &cube,
                [2., 5., 5.],
                [8., 0., 0.],
                false,
                "segment-unresolved",
            ),
        ] {
            let input = request(model, origin, direction);
            let r = crate::dispatch(input.clone()).unwrap();
            assert_eq!(r["sourceModel"], input["model"]);
            assert_eq!(r["limits"], input["limits"]);
            assert_eq!(r["origin"], input["origin"]);
            assert_eq!(r["direction"], input["direction"]);
            assert_eq!(r["proven"], json!(expected));
            assert_eq!(r["reason"], json!(reason));
            if expected {
                assert_eq!(r["seed"]["inside"], json!(true));
                assert_eq!(r["segment"]["boundaryFree"], json!(true));
            }
        }
        let mut input = request(&cube, [2., 5., 5.], [6., 0., 0.]);
        input["limits"]["segmentCells"] = json!(1);
        let r = crate::dispatch(input.clone()).unwrap();
        assert_eq!(r["proven"], json!(false));
        assert_eq!(r["reason"], json!("segment-unresolved"));
        input["limits"]["segmentCells"] = json!(0);
        assert!(crate::dispatch(input).is_err());
    }
}
