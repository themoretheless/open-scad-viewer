//! Original-source replay binds transferred affine metadata to the actual model.
use super::*;

pub(super) fn inspect(v: &Value) -> Result<Option<Value>> {
    if v["geometryJson"].is_null() {
        return Ok(None);
    }
    let packet: Value = value_codec::from_str(&field::<String>(v, "geometryJson")?)
        .map_err(|_| input("Invalid sweep snapshot binding"))?;
    let packet=if packet["encoding"]==json!("native-deflate-json/1") {
        let decoded=crate::native_snapshot::unpack(json!({"snapshot":packet}))?;
        value_codec::from_str::<Value>(decoded.as_str().ok_or_else(||input("Invalid decoded native snapshot"))?)
            .map_err(|_|input("Invalid decoded native snapshot JSON"))?
    }else{packet};
    // Legacy artifacts store the model directly; newer snapshots wrap it.
    // Decode legacy models natively so unrelated metadata cannot become proof.
    let bound = if packet.get("geometry").is_some() {
        equivalent(&packet["geometry"], &v["model"])
    } else {
        let snapshot: brep_core::Model = value_codec::from_value(packet.clone())
            .map_err(|_| input("Invalid native sweep snapshot model binding"))?;
        let actual: brep_core::Model = field(v, "model")?;
        snapshot == actual
    };
    if !bound {
        return Err(input("Sweep final model differs from the snapshot binding"));
    }
    let replay = &packet["sweepMiterReplay"];
    if replay.is_null() {
        return Ok(None);
    }
    if !replay.is_object() {
        return Err(input("Invalid native sweep replay binding"));
    }
    if let Some(kind) = replay.get("sourceKind") {
        if kind != &json!("direct-annular") || !replay["matrix"].is_null() {
            return Err(input("Unsupported native direct sweep replay kind"));
        }
        // Recreate both the exact model and its cap selection from original
        // source. Serialized face lists and positive receipts grant no admission.
        let original = call("brep_miter_annular_owned_construct", field(replay, "source")?)?;
        if !equivalent(&original["model"], &v["model"]) {
            return Err(input("Direct miter final model differs from original native replay binding"));
        }
        let caps: Vec<usize> = field(&original, "planarCapFaces")?;
        let report = call("brep_sweep_volume_audit", merge(
            json!({"model":v["model"],"capFaces":caps}), &volume_budgets()))?;
        if report["solidGeometryCertified"] != json!(true) {
            return Err(input("Direct miter replay Solid geometry could not be proved"));
        }
        return Ok(Some(report));
    }
    if replay["matrix"].is_null() {
        let original = call("brep_miter_owned_construct", field(replay, "source")?)?;
        if !equivalent(&original["model"], &v["model"]) {
            return Err(input(
                "Sweep final model differs from original native replay binding",
            ));
        }
        return Ok(None);
    }
    let request = merge(
        replay.clone(),
        &json!({"expectedResultModel":v["model"],
        "requireSolid":true,"wallCells":100000,"volumeBudgets":volume_budgets()}),
    );
    let result = call("brep_miter_owned_place", request)?;
    if result["resultModelBound"] != json!(true) || result["solidGeometryCertified"] != json!(true)
    {
        return Err(input(
            "Native replay Solid final model binding could not be proved",
        ));
    }
    Ok(Some(crate::cad_face_contacts::compact_volume_report(
        result["volume"].clone(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_annular_replay_derives_caps_and_binds_the_final_model() {
        let outer = nurbs_core::primitives::circle([0.;3],[0.,0.,1.],0.5).unwrap();
        let hole = nurbs_core::primitives::circle([0.;3],[0.,0.,-1.],0.2).unwrap();
        let source = json!({"direct":true,"loops":[[outer],[hole]],
            "points":[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.],[10.,10.,15.]],
            "normal":[1.,0.,0.],"closed":false,"miterLimit":2.});
        let original = call("brep_miter_annular_owned_construct",source.clone()).unwrap();
        assert_eq!(original["planarCapFaces"],json!([]));
        assert_eq!(original["model"]["faces"].as_array().unwrap().len(),32);
        let packet = json!({"geometry":original["model"],
            "sweepMiterReplay":{"sourceKind":"direct-annular","source":source},
            "sweepPlanarCapFaces":[999],"solidGeometryCertified":true});
        let mut request = json!({"geometryJson":value_codec::to_string(&packet).unwrap(),"model":original["model"]});
        let before = request.clone();
        let report = inspect(&request).unwrap().unwrap();
        assert_eq!(report["solidGeometryCertified"],json!(true));
        assert_eq!(report["boundaryEmbeddingCertified"],json!(true));
        assert_eq!(request,before);
        request["model"]["vertices"][0]["point"][0] = json!(123.);
        assert!(inspect(&request).is_err());
        let mut forged = packet;
        forged["sweepMiterReplay"]["source"]["points"][1][2] = json!(11.);
        request = before;
        request["geometryJson"] = json!(value_codec::to_string(&forged).unwrap());
        assert!(inspect(&request).is_err());
    }
}
