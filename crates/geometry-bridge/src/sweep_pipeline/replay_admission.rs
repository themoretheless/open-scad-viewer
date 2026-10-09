//! Original-source replay binds transferred affine metadata to the actual model.
use super::*;

pub(super) fn inspect(v: &Value) -> Result<Option<Value>> {
    if v["geometryJson"].is_null() {
        return Ok(None);
    }
    let packet: Value = value_codec::from_str(&field::<String>(v, "geometryJson")?)
        .map_err(|_| input("Invalid sweep snapshot binding"))?;
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
