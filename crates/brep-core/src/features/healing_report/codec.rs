//! Encoding and strict decoding of the top-level healing artifact.
use super::*;

impl Serialize for HealingReport {
    fn to_value(&self) -> Value {
        json!({
            "schema": HEALING_REPORT_SCHEMA,
            "version": HEALING_REPORT_VERSION,
            "inputHash": self.input_hash,
            "outputHash": self.output_hash,
            "status": self.status.as_str(),
            "operations": self.operations,
            "gaps": self.gaps,
            "sewnEdges": self.sewn_edges,
            "conversions": self.conversions,
            "displacement": self.displacement,
            "certificates": self.certificates,
            "displacementBudgetMm": self.displacement_budget_mm,
            "cumulativeDisplacementMm": self.cumulative_displacement_mm,
            "unresolvedDefects": self.unresolved_defects,
            "refusalReasons": self.refusal_reasons,
            "toleranceUnification": self.tolerance_unification,
            "stages": self.stages
        })
    }
}

impl<'de> Deserialize<'de> for HealingReport {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let schema: String = decode(&value, "schema")?;
        let version: u32 = decode(&value, "version")?;
        if schema != HEALING_REPORT_SCHEMA || !(1..=HEALING_REPORT_VERSION).contains(&version) {
            return Err(value_codec::error(format!(
                "Unsupported healing report schema/version: {schema}/{version}"
            )));
        }
        let mut fields = vec![
            "schema",
            "version",
            "inputHash",
            "outputHash",
            "status",
            "operations",
            "gaps",
            "sewnEdges",
            "conversions",
            "displacement",
            "certificates",
            "displacementBudgetMm",
            "cumulativeDisplacementMm",
            "unresolvedDefects",
            "refusalReasons",
        ];
        if version >= 2 {
            fields.push("toleranceUnification");
        }
        if version >= 3 {
            fields.push("stages");
        }
        require_keys(&value, &fields)?;
        Ok(Self {
            input_hash: decode(&value, "inputHash")?,
            output_hash: decode(&value, "outputHash")?,
            status: HealingStatus::parse(&value["status"])?,
            operations: decode(&value, "operations")?,
            gaps: decode(&value, "gaps")?,
            sewn_edges: decode(&value, "sewnEdges")?,
            conversions: decode(&value, "conversions")?,
            displacement: decode(&value, "displacement")?,
            certificates: decode(&value, "certificates")?,
            displacement_budget_mm: decode(&value, "displacementBudgetMm")?,
            cumulative_displacement_mm: decode(&value, "cumulativeDisplacementMm")?,
            unresolved_defects: decode(&value, "unresolvedDefects")?,
            refusal_reasons: decode(&value, "refusalReasons")?,
            tolerance_unification: if version == 1 {
                None
            } else {
                decode(&value, "toleranceUnification")?
            },
            stages: if version < 3 {
                Vec::new()
            } else {
                decode(&value, "stages")?
            },
        })
    }
}
