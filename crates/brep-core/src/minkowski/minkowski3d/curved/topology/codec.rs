use super::*;
#[cfg(feature = "codec")]
impl Serialize for Report {
    fn to_value(&self) -> Value {
        json!({"kind":"radial-homeomorphism","center":self.center,"radialDegree":1,"chartCells":self.chart_cells,"nominalTriangles":self.nominal_triangles,"filledSet":"closed-three-ball"})
    }
}
#[cfg(feature = "codec")]
impl<'de> Deserialize<'de> for Report {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        let keys = [
            "kind",
            "center",
            "radialDegree",
            "chartCells",
            "nominalTriangles",
            "filledSet",
        ];
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
            || v["kind"].as_str() != Some("radial-homeomorphism")
            || v["radialDegree"].as_i64() != Some(1)
            || v["filledSet"].as_str() != Some("closed-three-ball")
        {
            return Err(error("Source topology certificate schema mismatch"));
        }
        Ok(Self {
            center: value_codec::from_value(v["center"].clone())?,
            chart_cells: value_codec::from_value(v["chartCells"].clone())?,
            nominal_triangles: value_codec::from_value(v["nominalTriangles"].clone())?,
        })
    }
}
