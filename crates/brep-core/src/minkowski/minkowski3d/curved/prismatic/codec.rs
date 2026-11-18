use super::*;
#[cfg(feature = "codec")]
impl Serialize for ShellReport {
    fn to_value(&self) -> Value {
        json!({"shell":self.shell,"interval":self.interval,"curves":self.curves,"coefficients":self.coefficients})
    }
}
#[cfg(feature = "codec")]
impl<'de> Deserialize<'de> for ShellReport {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        schema(&v, &["shell", "interval", "curves", "coefficients"])?;
        Ok(Self {
            shell: value_codec::from_value(v["shell"].clone())?,
            interval: value_codec::from_value(v["interval"].clone())?,
            curves: value_codec::from_value(v["curves"].clone())?,
            coefficients: value_codec::from_value(v["coefficients"].clone())?,
        })
    }
}
#[cfg(feature = "codec")]
fn schema(v: &Value, keys: &[&str]) -> value_codec::Result<()> {
    if v.as_object()
        .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
    {
        return Err(error("Prismatic topology schema mismatch"));
    }
    Ok(())
}
#[cfg(feature = "codec")]
impl Serialize for Report {
    fn to_value(&self) -> Value {
        json!({"kind":"signed-prismatic-ambient-isotopy","axis":self.axis,"levels":self.levels,"shells":self.shells,"curveParents":self.parents,"nominalProfileVertices":self.nominal_profile_vertices,"preservesComplement":true})
    }
}
#[cfg(feature = "codec")]
impl<'de> Deserialize<'de> for Report {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        schema(
            &v,
            &[
                "kind",
                "axis",
                "levels",
                "shells",
                "curveParents",
                "nominalProfileVertices",
                "preservesComplement",
            ],
        )?;
        if v["kind"].as_str() != Some("signed-prismatic-ambient-isotopy")
            || v["preservesComplement"].as_bool() != Some(true)
        {
            return Err(error("Prismatic topology claim mismatch"));
        }
        Ok(Self {
            axis: value_codec::from_value(v["axis"].clone())?,
            levels: value_codec::from_value(v["levels"].clone())?,
            shells: value_codec::from_value(v["shells"].clone())?,
            parents: value_codec::from_value(v["curveParents"].clone())?,
            nominal_profile_vertices: value_codec::from_value(v["nominalProfileVertices"].clone())?,
        })
    }
}
#[cfg(feature = "codec")]
impl Serialize for HoleReport {
    fn to_value(&self) -> Value {
        json!({"hole":self.hole,"open":self.open,"depth":self.depth,"innerCore":self.inner_core,"outerCore":self.outer_core})
    }
}
#[cfg(feature = "codec")]
impl<'de> Deserialize<'de> for HoleReport {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        schema(&v, &["hole", "open", "depth", "innerCore", "outerCore"])?;
        Ok(Self {
            hole: value_codec::from_value(v["hole"].clone())?,
            open: value_codec::from_value(v["open"].clone())?,
            depth: value_codec::from_value(v["depth"].clone())?,
            inner_core: value_codec::from_value(v["innerCore"].clone())?,
            outer_core: value_codec::from_value(v["outerCore"].clone())?,
        })
    }
}
#[cfg(feature = "codec")]
impl Serialize for SumReport {
    fn to_value(&self) -> Value {
        json!({"kind":"convex-hole-prismatic-minkowski-homeomorphism","axis":self.axis,"interval":self.interval,"holedOperand":self.holed_operand,"holes":self.holes,"survivingHoles":self.surviving_holes})
    }
}
#[cfg(feature = "codec")]
impl<'de> Deserialize<'de> for SumReport {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        schema(
            &v,
            &[
                "kind",
                "axis",
                "interval",
                "holedOperand",
                "holes",
                "survivingHoles",
            ],
        )?;
        if v["kind"].as_str() != Some("convex-hole-prismatic-minkowski-homeomorphism") {
            return Err(error("Prismatic sum claim mismatch"));
        }
        Ok(Self {
            axis: value_codec::from_value(v["axis"].clone())?,
            interval: value_codec::from_value(v["interval"].clone())?,
            holed_operand: value_codec::from_value(v["holedOperand"].clone())?,
            holes: value_codec::from_value(v["holes"].clone())?,
            surviving_holes: value_codec::from_value(v["survivingHoles"].clone())?,
        })
    }
}
