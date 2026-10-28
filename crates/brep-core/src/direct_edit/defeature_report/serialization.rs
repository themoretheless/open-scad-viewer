//! Optional wire adapters; the parent retains native operations and reports.
use super::*;
use value_codec::{Deserialize, Serialize, Value, json};
fn decode<T: for<'a> Deserialize<'a>>(value: &Value, field: &str) -> value_codec::Result<T> {
    value_codec::from_value(value[field].clone())
        .map_err(|error| value_codec::error(format!("Invalid or missing {field}: {error}")))
}

fn kind_name(kind: DefeatureFeatureKind) -> &'static str {
    match kind {
        DefeatureFeatureKind::Fillet => "fillet",
        DefeatureFeatureKind::Hole => "hole",
        DefeatureFeatureKind::Pocket => "pocket",
    }
}

fn parse_kind(value: &Value) -> value_codec::Result<DefeatureFeatureKind> {
    match value.as_str() {
        Some("fillet") => Ok(DefeatureFeatureKind::Fillet),
        Some("hole") => Ok(DefeatureFeatureKind::Hole),
        Some("pocket") => Ok(DefeatureFeatureKind::Pocket),
        _ => Err(value_codec::error("Unknown defeature feature kind")),
    }
}

impl Serialize for DefeatureFeatureSize {
    fn to_value(&self) -> Value {
        match *self {
            Self::Fillet {
                radius_min_mm,
                radius_max_mm,
            } => json!({
                "kind": "fillet",
                "radiusMinMm": radius_min_mm,
                "radiusMaxMm": radius_max_mm
            }),
            Self::Hole {
                diameter_mm,
                depth_mm,
            } => json!({
                "kind": "hole",
                "diameterMm": diameter_mm,
                "depthMm": depth_mm
            }),
            Self::Pocket {
                floor_area_mm2,
                depth_mm,
            } => json!({
                "kind": "pocket",
                "floorAreaMm2": floor_area_mm2,
                "depthMm": depth_mm
            }),
        }
    }
}

impl<'de> Deserialize<'de> for DefeatureFeatureSize {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        match parse_kind(&value["kind"])? {
            DefeatureFeatureKind::Fillet => Ok(Self::Fillet {
                radius_min_mm: decode(&value, "radiusMinMm")?,
                radius_max_mm: decode(&value, "radiusMaxMm")?,
            }),
            DefeatureFeatureKind::Hole => Ok(Self::Hole {
                diameter_mm: decode(&value, "diameterMm")?,
                depth_mm: decode(&value, "depthMm")?,
            }),
            DefeatureFeatureKind::Pocket => Ok(Self::Pocket {
                floor_area_mm2: decode(&value, "floorAreaMm2")?,
                depth_mm: decode(&value, "depthMm")?,
            }),
        }
    }
}

impl Serialize for DefeatureFeatureReport {
    fn to_value(&self) -> Value {
        json!({"kind": kind_name(self.kind), "faceIds": self.face_ids, "size": self.size})
    }
}

impl<'de> Deserialize<'de> for DefeatureFeatureReport {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let kind = parse_kind(&value["kind"])?;
        let size = DefeatureFeatureSize::from_value(value["size"].clone())?;
        let size_kind = match size {
            DefeatureFeatureSize::Fillet { .. } => DefeatureFeatureKind::Fillet,
            DefeatureFeatureSize::Hole { .. } => DefeatureFeatureKind::Hole,
            DefeatureFeatureSize::Pocket { .. } => DefeatureFeatureKind::Pocket,
        };
        if kind != size_kind {
            return Err(value_codec::error("Feature kind and size kind disagree"));
        }
        Ok(Self {
            kind,
            face_ids: decode(&value, "faceIds")?,
            size,
        })
    }
}

impl Serialize for DefeatureChangedZone {
    fn to_value(&self) -> Value {
        json!({
            "featureIndices": self.feature_indices,
            "sourceFaceIds": self.source_face_ids,
            "resultFaceIds": self.result_face_ids
        })
    }
}

impl<'de> Deserialize<'de> for DefeatureChangedZone {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        Ok(Self {
            feature_indices: decode(&value, "featureIndices")?,
            source_face_ids: decode(&value, "sourceFaceIds")?,
            result_face_ids: decode(&value, "resultFaceIds")?,
        })
    }
}

impl Serialize for DefeatureReport {
    fn to_value(&self) -> Value {
        json!({
            "schema": DEFEATURE_REPORT_SCHEMA,
            "version": DEFEATURE_REPORT_VERSION,
            "dryRun": self.dry_run,
            "removedFeatures": self.removed_features,
            "changedZones": self.changed_zones,
            "volumeBeforeMm3": self.volume_before_mm3,
            "volumeAfterMm3": self.volume_after_mm3,
            "absDeltaVolumeMm3": self.abs_delta_volume_mm3,
            "absRelativeDeltaVolume": self.abs_relative_delta_volume
        })
    }
}

impl<'de> Deserialize<'de> for DefeatureReport {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        let schema: String = decode(&value, "schema")?;
        let version: u32 = decode(&value, "version")?;
        if schema != DEFEATURE_REPORT_SCHEMA || version != DEFEATURE_REPORT_VERSION {
            return Err(value_codec::error(format!(
                "Unsupported defeature report schema/version: {schema}/{version}"
            )));
        }
        Ok(Self {
            dry_run: decode(&value, "dryRun")?,
            removed_features: decode(&value, "removedFeatures")?,
            changed_zones: decode(&value, "changedZones")?,
            volume_before_mm3: decode(&value, "volumeBeforeMm3")?,
            volume_after_mm3: decode(&value, "volumeAfterMm3")?,
            abs_delta_volume_mm3: decode(&value, "absDeltaVolumeMm3")?,
            abs_relative_delta_volume: decode(&value, "absRelativeDeltaVolume")?,
        })
    }
}

impl DefeatureReport {
    pub fn to_json(&self) -> Result<String> {
        value_codec::to_string(&self.to_value()).map_err(|error| {
            report_error(
                DEFEATURE_REPORT_CODEC,
                format!("Cannot encode defeature report: {error}"),
            )
        })
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let value: Value = value_codec::from_str(text).map_err(|error| {
            report_error(
                DEFEATURE_REPORT_CODEC,
                format!("Cannot decode defeature report JSON: {error}"),
            )
        })?;
        Self::from_value(value).map_err(|error| {
            report_error(
                DEFEATURE_REPORT_CODEC,
                format!("Invalid defeature report: {error}"),
            )
        })
    }
}
