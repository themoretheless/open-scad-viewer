use super::*;
#[cfg(feature = "codec")]
impl Serialize for Report {
    fn to_value(&self) -> Value {
        json!({"kind":"rational-atlas-ambient-isotopy","chartTriangles":self.chart_triangles,"vertexStars":self.vertex_stars,"disjointPairs":self.disjoint_pairs,"boundaryPairs":self.boundary_pairs,"preservesComplement":true})
    }
}
#[cfg(feature = "codec")]
impl<'de> Deserialize<'de> for Report {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        let keys = [
            "kind",
            "chartTriangles",
            "vertexStars",
            "disjointPairs",
            "boundaryPairs",
            "preservesComplement",
        ];
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
            || v["kind"].as_str() != Some("rational-atlas-ambient-isotopy")
            || v["preservesComplement"].as_bool() != Some(true)
        {
            return Err(error("Rational atlas report schema mismatch"));
        }
        Ok(Self {
            chart_triangles: value_codec::from_value(v["chartTriangles"].clone())?,
            vertex_stars: value_codec::from_value(v["vertexStars"].clone())?,
            disjoint_pairs: value_codec::from_value(v["disjointPairs"].clone())?,
            boundary_pairs: value_codec::from_value(v["boundaryPairs"].clone())?,
        })
    }
}
#[cfg(feature = "codec")]
impl Serialize for Certificate {
    fn to_value(&self) -> Value {
        json!({"version":self.version,"scope":"represented-nurbs-source-atlas-isotopy","source":self.source,"depth":self.depth,"positions":self.positions,"triangles":self.triangles,"report":self.report})
    }
}
#[cfg(feature = "codec")]
impl<'de> Deserialize<'de> for Certificate {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        let keys = [
            "version",
            "scope",
            "source",
            "depth",
            "positions",
            "triangles",
            "report",
        ];
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
            || v["scope"].as_str() != Some("represented-nurbs-source-atlas-isotopy")
            || v["version"].as_u64() != Some(1)
        {
            return Err(error("Rational atlas certificate schema mismatch"));
        }
        Ok(Self {
            version: 1,
            source: value_codec::from_value(v["source"].clone())?,
            depth: value_codec::from_value(v["depth"].clone())?,
            positions: value_codec::from_value(v["positions"].clone())?,
            triangles: value_codec::from_value(v["triangles"].clone())?,
            report: value_codec::from_value(v["report"].clone())?,
        })
    }
}
