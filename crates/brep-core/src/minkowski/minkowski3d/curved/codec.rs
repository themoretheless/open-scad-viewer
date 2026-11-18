use super::*;
impl Serialize for Source {
    fn to_value(&self) -> Value {
        match self {
            Self::Mesh(m) => json!({"kind":"mesh","mesh":m}),
            Self::Brep(m) => json!({"kind":"brep","model":m}),
        }
    }
}
impl<'de> Deserialize<'de> for Source {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        match v["kind"].as_str() {
            Some("mesh") => Ok(Self::Mesh(value_codec::from_value(v["mesh"].clone())?)),
            Some("brep") => Ok(Self::Brep(value_codec::from_value(v["model"].clone())?)),
            _ => Err(error("Unknown Minkowski source kind")),
        }
    }
}
impl Serialize for EnclosureCertificate {
    fn to_value(&self) -> Value {
        let mut v = json!({"version":self.version,"scope":SCOPE,"sources":self.sources,"depths":self.depths,"maximumSourceDeviationMm":self.maximum_source_deviation_mm,"sourceHomotopyBounds":self.source_homotopy_bounds,"filledHausdorffBound":self.filled_hausdorff_bound,"nominal":self.nominal,"inner":self.inner,"outer":self.outer});
        if self.version >= 2 {
            v["sourceTopology"] = self.source_topology.to_value()
        }
        if self.version >= 3 {
            v["sourceAtlasTopology"] = self.source_atlas_topology.to_value();
            v["sumTopology"] = self.sum_topology.to_value();
        }
        v
    }
}
impl<'de> Deserialize<'de> for EnclosureCertificate {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        let mut keys = vec![
            "version",
            "scope",
            "sources",
            "depths",
            "maximumSourceDeviationMm",
            "sourceHomotopyBounds",
            "filledHausdorffBound",
            "nominal",
            "inner",
            "outer",
        ];
        if matches!(v["version"].as_u64(), Some(2 | 3 | 4)) {
            keys.push("sourceTopology")
        }
        if matches!(v["version"].as_u64(), Some(3 | 4)) {
            keys.extend(["sourceAtlasTopology", "sumTopology"]);
        }
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
            || v["scope"].as_str() != Some(SCOPE)
        {
            return Err(error("NURBS Minkowski enclosure schema mismatch"));
        }
        Ok(Self {
            version: value_codec::from_value(v["version"].clone())?,
            sources: value_codec::from_value(v["sources"].clone())?,
            depths: value_codec::from_value(v["depths"].clone())?,
            maximum_source_deviation_mm: value_codec::from_value(
                v["maximumSourceDeviationMm"].clone(),
            )?,
            source_homotopy_bounds: value_codec::from_value(v["sourceHomotopyBounds"].clone())?,
            filled_hausdorff_bound: value_codec::from_value(v["filledHausdorffBound"].clone())?,
            nominal: value_codec::from_value(v["nominal"].clone())?,
            inner: value_codec::from_value(v["inner"].clone())?,
            outer: value_codec::from_value(v["outer"].clone())?,
            source_topology: if matches!(v["version"].as_u64(), Some(2 | 3 | 4)) {
                value_codec::from_value(v["sourceTopology"].clone())?
            } else {
                [None, None]
            },
            source_atlas_topology: if matches!(v["version"].as_u64(), Some(3 | 4)) {
                value_codec::from_value(v["sourceAtlasTopology"].clone())?
            } else {
                [None, None]
            },
            sum_topology: if matches!(v["version"].as_u64(), Some(3 | 4)) {
                value_codec::from_value(v["sumTopology"].clone())?
            } else {
                None
            },
        })
    }
}
