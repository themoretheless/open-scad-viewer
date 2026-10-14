//! Native smooth station construction and fresh retained-body qualification.
//! The correction bound refers to authored ruled walls, not an ideal sweep.
use crate::{field, input, Result};
use nurbs_core::curve::Curve;
use value_codec::{json, Value};
mod source;

pub fn dispatch(op: &str, v: &Value) -> Result<Value> {
    match op {
        "brep_nurbs_section_loft_surfaces" => {
            crate::encode(brep_core::analytic::section_loft_surfaces(
                &field::<Vec<Vec<Vec<Curve>>>>(v, "sections")?,
                &field::<Vec<Vec<nurbs_core::surface::Surface>>>(v, "sides")?,
                field(&v, "closed")?,
            )?)
        }
        "brep_nurbs_rational_section_loft" => {
            crate::encode(brep_core::rational_section_loft(&field::<
                Vec<Vec<Vec<Curve>>>,
            >(
                v, "sections"
            )?)?)
        }
        _ => construct(v.clone()),
    }
}

pub fn construct(v: Value) -> Result<Value> {
    let sections: Vec<Vec<Vec<Curve>>> = field(&v, "sections")?;
    let closed: bool = field(&v, "closed")?;
    let max_work: u64 = field(&v, "maxWork")?;
    let result = brep_core::analytic::smooth_polygon_station_walls(
        &sections,
        &[],
        closed,
        field(&v, "quantum")?,
        field(&v, "maxDisplacement")?,
        max_work,
    )?;
    let construction = json!({"method":"dyadic-quintic-polygon-stations",
        "scope":"authored-ruled-wall-correction","reason":result.reason,
        "maxWork":max_work,"work":result.work,
        "wallDisplacementUpper":result.wall_displacement_upper,
        "stationOrders":result.station_orders,"continuousBound":false});
    let (Some(sections), Some(sides)) = (result.sections, result.sides) else {
        return Ok(json!({"model":null,"construction":construction,"accepted":false}));
    };
    let continuous = if v["source"].is_null() {
        None
    } else {
        let report = source::certify(&v["source"], &sides, closed)?;
        if report["withinBudget"] != json!(true) {
            return Ok(
                json!({"model":null,"accepted":false,"construction":construction,
                "continuousCertificate":report,"continuousBound":false}),
            );
        }
        Some(report)
    };
    let model = brep_core::analytic::section_loft_surfaces(&sections, &sides, closed)?;
    let caps = if closed {
        vec![]
    } else {
        if model.faces.len() < 2 {
            return Err(input("Missing polygon endpoint caps"));
        }
        vec![model.faces.len() - 2, model.faces.len() - 1]
    };
    let remaining = max_work - result.work;
    let station = brep_core::miter_seams::inspect(&model, &caps, remaining, true)?;
    let station_json = crate::miter_smoothness::station(&station, remaining);
    if !station.g2.certified {
        return Ok(
            json!({"model":null,"accepted":false,"construction":construction,"station":station_json,"stationContinuity":"unproved","continuousBound":false}),
        );
    }
    let mut audit = crate::sweep_pipeline::volume_budgets();
    audit["op"] = json!("brep_sweep_volume_audit");
    audit["model"] = crate::encode(&model)?;
    audit["capFaces"] = json!(caps);
    let volume = crate::dispatch(audit)?;
    let accepted = station.g2.certified && volume["solidGeometryCertified"] == json!(true);
    Ok(json!({"model":if accepted {Some(model)} else {None},
        "accepted":accepted,"construction":construction,"station":station_json,
        "volume":volume,"stationContinuity":"G2",
        "profileContinuity":"C0","continuousCertificate":continuous,"continuousBound":accepted && continuous.is_some(),"globalEmbeddingCertified":false,"retainedBodyGeometryCertified":accepted}))
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn request() -> Value {
        let snap = |x: f64| (1024. * x).round() / 1024.;
        let mut sections = Vec::new();
        for station in 0..5 {
            let angle = std::f64::consts::TAU * station as f64 / 5.;
            let (s, c) = angle.sin_cos();
            let dz = 0.25 * (2. * angle).cos();
            let center = [snap(3. * c), snap(3. * s), snap(0.125 * (2. * angle).sin())];
            let radial = [snap(c / 8.), snap(s / 8.), 0.];
            let normal = [snap(-dz * s / 8.), snap(dz * c / 8.), -0.125];
            let points: Vec<Vec<f64>> = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]
                .into_iter()
                .map(|(a, b)| {
                    (0..3)
                        .map(|k| center[k] + a * radial[k] + b * normal[k])
                        .collect()
                })
                .collect();
            sections.push(vec![(0..4)
                .map(|i| Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: vec![points[i].clone(), points[(i + 1) % 4].clone()],
                    weights: vec![1., 2.],
                    periodic: false,
                })
                .collect::<Vec<_>>()]);
        }
        sections.push(sections[0].clone());
        json!({"sections":sections,"closed":true,"quantum":1./1024.,
            "maxDisplacement":1.,"maxWork":1000000})
    }
    #[test]
    fn native_json_qualifies_weighted_spatial_closed_body_and_refuses_limits() {
        let req = request();
        let result = construct(req.clone()).unwrap();
        assert_eq!(result["accepted"], json!(true), "{result}");
        assert_eq!(result["stationContinuity"], json!("G2"));
        assert_eq!(result["station"]["stationG2Certified"], json!(true));
        assert_eq!(result["volume"]["solidGeometryCertified"], json!(true));
        assert_eq!(result["continuousBound"], json!(false));
        let mut inverted: brep_core::Model = field(&result, "model").unwrap();
        for face_use in &mut inverted.shells[0].faces {
            face_use.reversed = !face_use.reversed;
        }
        let mut audit = crate::sweep_pipeline::volume_budgets();
        audit["op"] = json!("brep_sweep_volume_audit");
        audit["model"] = crate::encode(inverted).unwrap();
        audit["capFaces"] = json!([]);
        let rejected = crate::dispatch(audit).unwrap();
        assert_eq!(rejected["solidGeometryCertified"], json!(false));
        for (field_name, value) in [("maxWork", json!(1)), ("maxDisplacement", json!(0))] {
            let mut denied = req.clone();
            denied[field_name] = value;
            let result = construct(denied).unwrap();
            assert_eq!(result["accepted"], json!(false));
            assert!(result["model"].is_null());
        }
    }
    #[test]
    fn weighted_polygon_budget_probe_retains_unresolved_pairs() {
        let req = request();
        let sections: Vec<Vec<Vec<Curve>>> = field(&req, "sections").unwrap();
        let candidate = brep_core::analytic::smooth_polygon_station_walls(
            &sections,
            &[],
            true,
            1. / 1024.,
            1.,
            100000,
        )
        .unwrap();
        let model = brep_core::analytic::section_loft_surfaces(
            candidate.sections.as_ref().unwrap(),
            candidate.sides.as_ref().unwrap(),
            true,
        )
        .unwrap();
        let mut audit = crate::sweep_pipeline::volume_budgets();
        audit["model"] = crate::encode(model).unwrap();
        audit["capFaces"] = json!([]);
        audit["maxCells"] = json!(5000);
        let report = crate::cad_face_contacts::diagnose_sweep_embedding(audit.clone()).unwrap();
        eprintln!(
            "weighted source body probe: charts={} linearCells={} pairs={} next={}",
            report["allFacesInjective"],
            report["linearCells"],
            report["allPairsClassified"],
            report["nextPair"]
        );
        audit["maxCells"] = json!(1);
        let denied = crate::cad_face_contacts::diagnose_sweep_embedding(audit).unwrap();
        assert_eq!(denied["boundaryEmbeddingCertified"], json!(false));
        assert!(!denied["nextPair"].is_null());
    }
    #[test]
    fn native_json_composes_source_bound_and_actual_closed_body_proofs() {
        let mut req = request();
        req["source"] = source::test_source();
        let result = construct(req.clone()).unwrap();
        assert_eq!(result["accepted"], json!(true));
        assert_eq!(result["continuousBound"], json!(true));
        assert_eq!(result["globalEmbeddingCertified"], json!(false));
        assert_eq!(result["retainedBodyGeometryCertified"], json!(true));
        assert_eq!(result["station"]["stationG2Certified"], json!(true));
        assert_eq!(result["volume"]["solidGeometryCertified"], json!(true));
        req["source"]["maxCells"] = json!(1);
        let denied = construct(req).unwrap();
        assert_eq!(denied["accepted"], json!(false));
        assert!(denied["model"].is_null());
        assert_eq!(denied["continuousBound"], json!(false));
    }
}
