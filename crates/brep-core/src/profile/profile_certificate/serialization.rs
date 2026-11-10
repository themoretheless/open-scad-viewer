use super::*;
use value_codec::{Deserialize, Serialize, Value, error, json};
impl Serialize for ProfileCertificate {
    fn to_value(&self) -> Value {
        json!({"version":self.version,"source":self.source,
            "subdivisionDepth":self.subdivision_depth,"monotoneAxes":self.monotone_axes,
            "separatingNormals":self.separating_normals,"orientation":self.orientation,
            "nestingDepth":self.nesting_depth,"orientationDepth":self.orientation_depth,
            "scope":"represented-profile-topology"})
    }
}
impl<'de> Deserialize<'de> for ProfileCertificate {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        let o = v
            .as_object()
            .ok_or_else(|| error("Expected profile certificate"))?;
        let keys = [
            "version",
            "source",
            "subdivisionDepth",
            "monotoneAxes",
            "separatingNormals",
            "orientation",
            "nestingDepth",
            "orientationDepth",
            "scope",
        ];
        if o.len() != keys.len()
            || keys.iter().any(|k| !o.contains_key(*k))
            || v["scope"].as_str() != Some("represented-profile-topology")
        {
            return Err(error(
                "Profile certificate fields or scope do not match the schema",
            ));
        }
        // Check lengths before allocating decoded vectors.
        if v["source"].as_str().is_none_or(|s| s.len() > 65536)
            || [
                ("monotoneAxes", MAX_PIECES),
                ("separatingNormals", MAX_PIECES * (MAX_PIECES - 1) / 2),
                ("orientation", 32),
                ("nestingDepth", 32),
            ]
            .iter()
            .any(|(k, n)| v[*k].as_array().is_none_or(|a| a.len() > *n))
        {
            return Err(error("Profile certificate exceeds its size budget"));
        }
        Ok(Self {
            version: Deserialize::from_value(v["version"].clone())?,
            source: Deserialize::from_value(v["source"].clone())?,
            subdivision_depth: Deserialize::from_value(v["subdivisionDepth"].clone())?,
            monotone_axes: Deserialize::from_value(v["monotoneAxes"].clone())?,
            separating_normals: Deserialize::from_value(v["separatingNormals"].clone())?,
            orientation: Deserialize::from_value(v["orientation"].clone())?,
            nesting_depth: Deserialize::from_value(v["nestingDepth"].clone())?,
            orientation_depth: Deserialize::from_value(v["orientationDepth"].clone())?,
        })
    }
}
