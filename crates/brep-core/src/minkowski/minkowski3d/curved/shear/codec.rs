use super::*;
#[cfg(feature = "codec")]
impl Serialize for Report {
    fn to_value(&self) -> Value {
        json!({"kind":"polynomial-shear-ambient-isotopy","axis":self.axis,"polynomial":self.polynomial,"sourceChartCells":self.source_chart_cells,"referenceTriangles":self.reference_triangles,"homotopySegments":self.homotopy_segments,"checkedPairs":self.checked_pairs,"preservesComplement":true})
    }
}
#[cfg(feature = "codec")]
impl<'de> Deserialize<'de> for Report {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        let keys = [
            "kind",
            "axis",
            "polynomial",
            "sourceChartCells",
            "referenceTriangles",
            "homotopySegments",
            "checkedPairs",
            "preservesComplement",
        ];
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
            || v["kind"].as_str() != Some("polynomial-shear-ambient-isotopy")
            || v["preservesComplement"].as_bool() != Some(true)
        {
            return Err(error("Polynomial ambient topology schema mismatch"));
        }
        Ok(Self {
            axis: value_codec::from_value(v["axis"].clone())?,
            polynomial: value_codec::from_value(v["polynomial"].clone())?,
            source_chart_cells: value_codec::from_value(v["sourceChartCells"].clone())?,
            reference_triangles: value_codec::from_value(v["referenceTriangles"].clone())?,
            homotopy_segments: value_codec::from_value(v["homotopySegments"].clone())?,
            checked_pairs: value_codec::from_value(v["checkedPairs"].clone())?,
        })
    }
}
