use super::*;
use value_codec::{Deserialize, Serialize, Value, error, json};
impl Serialize for RoundingTopologyReport {
    fn to_value(&self) -> Value {
        json!({"status":"proved","precision":self.precision,"identity":self.identity,"pairs":self.pairs,"separatedPairs":self.separated,"sharedEdges":self.shared_edges,"sharedVertices":self.shared_vertices,"wholeLinearHomotopy":true})
    }
}
impl Serialize for SourceMesh {
    fn to_value(&self) -> Value {
        json!({"positions":self.positions,"indices":self.indices})
    }
}
impl<'de> Deserialize<'de> for SourceMesh {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        if v["positions"].as_array().is_none_or(|a| a.len() > 3072)
            || v["indices"].as_array().is_none_or(|a| a.len() > 6144)
        {
            return Err(error("Minkowski 3D source budget exceeded"));
        }
        Ok(Self {
            positions: value_codec::from_value(v["positions"].clone())?,
            indices: value_codec::from_value(v["indices"].clone())?,
        })
    }
}
impl Serialize for Certificate {
    fn to_value(&self) -> Value {
        json!({"version":self.version,"scope":SCOPE,"sources":self.sources,"vertices":self.vertices,"triangles":self.triangles,"sourceCells":self.source_cells,"sumCells":self.sum_cells})
    }
}
impl<'de> Deserialize<'de> for Certificate {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        let keys = [
            "version",
            "scope",
            "sources",
            "vertices",
            "triangles",
            "sourceCells",
            "sumCells",
        ];
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
            || v["scope"].as_str() != Some(SCOPE)
            || v["sources"].as_array().is_none_or(|a| {
                a.len() != 2 || a.iter().any(|s| s.as_str().is_none_or(|s| s.len() != 64))
            })
            || v["vertices"].as_array().is_none_or(|a| {
                a.len() > 32768
                    || a.iter().any(|p| {
                        p.as_array().is_none_or(|p| {
                            p.len() != 3
                                || p.iter().any(|r| {
                                    r.as_array().is_none_or(|r| {
                                        r.len() != 2
                                            || r.iter()
                                                .any(|s| s.as_str().is_none_or(|s| s.len() > 2500))
                                    })
                                })
                        })
                    })
            })
            || v["triangles"].as_array().is_none_or(|a| a.len() > 65536)
        {
            return Err(error("Minkowski 3D certificate schema or size exceeded"));
        }
        Ok(Self {
            version: value_codec::from_value(v["version"].clone())?,
            sources: value_codec::from_value(v["sources"].clone())?,
            vertices: value_codec::from_value(v["vertices"].clone())?,
            triangles: value_codec::from_value(v["triangles"].clone())?,
            source_cells: value_codec::from_value(v["sourceCells"].clone())?,
            sum_cells: value_codec::from_value(v["sumCells"].clone())?,
        })
    }
}
