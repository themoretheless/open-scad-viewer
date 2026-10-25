//! Unified, serializable report for fillet and hole/pocket defeaturing.
//!
//! Dry-run executes the same staged batch path as a real run and then discards
//! the staged model. Thus recognition, ordering, per-feature refusals, volume
//! measurement and dirty-zone construction cannot diverge merely because the
//! caller requested a preview. Changed zones contain persistent face ids only,
//! never model-local face indices.

use super::defeature_fillet::{DefeatureFilletOptions, DefeatureFilletStatus, defeature_fillets};
use super::defeature_hole_pocket::{
    DefeatureBatchReport, DefeatureKind, DefeatureOperationReport, DefeatureOptions,
    defeature_holes_and_pockets,
};
use super::*;
use crate::TopoId;
use crate::aag::Aag;
use crate::aag_features::{find_holes, find_pockets};
use value_codec::{Deserialize, Serialize, Value, json};

pub const DEFEATURE_REPORT_SCHEMA: &str = "brep-defeature-report";
pub const DEFEATURE_REPORT_VERSION: u32 = 1;
pub const DEFEATURE_REPORT_CODEC: &str = "BREP_DEFEATURE_REPORT_CODEC";
pub const DEFEATURE_REPORT_INVALID: &str = "BREP_DEFEATURE_REPORT_INVALID";

#[derive(Clone, Debug)]
pub struct DefeatureReportOptions {
    /// Execute and measure the staged result, but return the original model.
    pub dry_run: bool,
    pub fillets: bool,
    pub holes_and_pockets: DefeatureOptions,
}

impl Default for DefeatureReportOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            fillets: true,
            holes_and_pockets: DefeatureOptions::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefeatureFeatureKind {
    Fillet,
    Hole,
    Pocket,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DefeatureFeatureSize {
    Fillet {
        radius_min_mm: f64,
        radius_max_mm: f64,
    },
    Hole {
        diameter_mm: f64,
        depth_mm: f64,
    },
    Pocket {
        floor_area_mm2: f64,
        depth_mm: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefeatureFeatureReport {
    pub kind: DefeatureFeatureKind,
    pub face_ids: Vec<TopoId>,
    pub size: DefeatureFeatureSize,
}

/// One conservative changed region. `feature_indices` refers to
/// [`DefeatureReport::removed_features`]; all topology references in the zone
/// itself are persistent ids and survive result-model reindexing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefeatureChangedZone {
    pub feature_indices: Vec<usize>,
    pub source_face_ids: Vec<TopoId>,
    pub result_face_ids: Vec<TopoId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefeatureReport {
    pub dry_run: bool,
    pub removed_features: Vec<DefeatureFeatureReport>,
    pub changed_zones: Vec<DefeatureChangedZone>,
    pub volume_before_mm3: f64,
    pub volume_after_mm3: f64,
    pub abs_delta_volume_mm3: f64,
    /// |after − before| / |before|.
    pub abs_relative_delta_volume: f64,
}

#[derive(Clone, Debug)]
pub struct DefeatureRun {
    /// Staged result for a real run; bit-identical input for a dry-run.
    pub model: Model,
    pub report: DefeatureReport,
}

fn report_error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

fn persistent_ids(model: &Model, faces: &BTreeSet<usize>) -> Result<Vec<TopoId>> {
    faces
        .iter()
        .map(|&face| {
            model.1.faces.get(face).cloned().ok_or_else(|| {
                report_error(
                    DEFEATURE_REPORT_INVALID,
                    format!("No persistent id for face {face}"),
                )
            })
        })
        .collect()
}

fn hole_core(hole: &HoleFeature) -> BTreeSet<usize> {
    hole.faces
        .iter()
        .chain(&hole.step_faces)
        .chain(&hole.bottom_faces)
        .copied()
        .collect()
}

fn attributed_graph(model: &Model, budget: &Budget) -> Result<Aag> {
    let mut graph = Aag::build(model, budget)?;
    graph.attach_face_attrs(model, budget)?;
    Ok(graph)
}

fn operation_size(
    model: &Model,
    operation: &DefeatureOperationReport,
    budget: &Budget,
) -> Result<DefeatureFeatureSize> {
    let graph = attributed_graph(model, budget)?;
    let source: BTreeSet<usize> = operation.source_faces.iter().copied().collect();
    match operation.kind {
        DefeatureKind::Hole => {
            let hole = find_holes(model, &graph, budget)?
                .into_iter()
                .find(|hole| hole_core(hole) == source)
                .ok_or_else(|| {
                    report_error(
                        DEFEATURE_REPORT_INVALID,
                        "Cannot re-recognize removed hole for its size report",
                    )
                })?;
            Ok(DefeatureFeatureSize::Hole {
                diameter_mm: hole.diameter,
                depth_mm: hole.depth,
            })
        }
        DefeatureKind::Pocket => {
            let pocket = find_pockets(model, &graph, budget)?
                .into_iter()
                .find(|pocket| pocket.faces.iter().copied().collect::<BTreeSet<_>>() == source)
                .ok_or_else(|| {
                    report_error(
                        DEFEATURE_REPORT_INVALID,
                        "Cannot re-recognize removed pocket for its size report",
                    )
                })?;
            Ok(DefeatureFeatureSize::Pocket {
                floor_area_mm2: pocket.floor_area,
                depth_mm: pocket.depth,
            })
        }
    }
}

fn changed_zone(
    source: &Model,
    result: &Model,
    seeds: &BTreeSet<usize>,
    feature_indices: Vec<usize>,
    budget: &Budget,
) -> Result<DefeatureChangedZone> {
    let dirty = dirty_region_for_result(
        source,
        result,
        &seeds.iter().copied().collect::<Vec<_>>(),
        budget,
    )?;
    Ok(DefeatureChangedZone {
        feature_indices,
        source_face_ids: persistent_ids(source, &dirty.faces)?,
        result_face_ids: persistent_ids(result, &dirty.result_faces)?,
    })
}

fn collect_hole_pocket_features(
    source: &Model,
    batch: &DefeatureBatchReport,
    features: &mut Vec<DefeatureFeatureReport>,
    zones: &mut Vec<DefeatureChangedZone>,
    budget: &Budget,
) -> Result<()> {
    let mut before = source.clone();
    for operation in batch
        .operations
        .iter()
        .filter(|operation| operation.removed)
    {
        let size = operation_size(&before, operation, budget)?;
        let seed_indices: BTreeSet<usize> = operation
            .source_faces
            .iter()
            .chain(&operation.entry_fillet_faces)
            .copied()
            .collect();
        let feature = DefeatureFeatureReport {
            kind: match operation.kind {
                DefeatureKind::Hole => DefeatureFeatureKind::Hole,
                DefeatureKind::Pocket => DefeatureFeatureKind::Pocket,
            },
            face_ids: persistent_ids(&before, &seed_indices)?,
            size,
        };
        let feature_index = features.len();
        features.push(feature);
        zones.push(changed_zone(
            &before,
            &operation.model,
            &seed_indices,
            vec![feature_index],
            budget,
        )?);
        before = operation.model.clone();
    }
    Ok(())
}

/// Run both existing batch paths and capture their common report. Hole/pocket
/// removal runs first so a subsequent fillet suppression sees the same staged
/// curved-neighborhood reduction in dry-run and real execution.
pub fn defeature_with_report(
    model: &Model,
    options: &DefeatureReportOptions,
    budget: &Budget,
) -> Result<DefeatureRun> {
    model.validate()?;
    let mut staged = model.clone();
    let mut removed_features = Vec::new();
    let mut changed_zones = Vec::new();

    if options.holes_and_pockets.holes || options.holes_and_pockets.pockets {
        let batch = defeature_holes_and_pockets(&staged, &options.holes_and_pockets, budget)?;
        collect_hole_pocket_features(
            &staged,
            &batch,
            &mut removed_features,
            &mut changed_zones,
            budget,
        )?;
        staged = batch.model;
    }

    if options.fillets {
        let fillet_source = staged.clone();
        let batch = defeature_fillets(&staged, DefeatureFilletOptions::default(), budget)?;
        let successful: Vec<_> = batch
            .outcomes
            .iter()
            .filter(|outcome| outcome.status == DefeatureFilletStatus::Removed)
            .collect();
        let mut seeds = BTreeSet::new();
        let mut feature_indices = Vec::new();
        for outcome in successful {
            seeds.extend(outcome.chain.faces.iter().copied());
            feature_indices.push(removed_features.len());
            removed_features.push(DefeatureFeatureReport {
                kind: DefeatureFeatureKind::Fillet,
                face_ids: outcome.face_ids.clone(),
                size: DefeatureFeatureSize::Fillet {
                    radius_min_mm: outcome.chain.radius_min,
                    radius_max_mm: outcome.chain.radius_max,
                },
            });
        }
        if !seeds.is_empty() {
            changed_zones.push(changed_zone(
                &fillet_source,
                &batch.model,
                &seeds,
                feature_indices,
                budget,
            )?);
        }
        staged = batch.model;
    }

    let volume_before = crate::analysis::mass_properties(model, 1e-9, 500_000)?.signed_volume_mm3;
    let volume_after = crate::analysis::mass_properties(&staged, 1e-9, 500_000)?.signed_volume_mm3;
    let abs_delta_volume_mm3 = (volume_after - volume_before).abs();
    if !volume_before.is_finite() || volume_before.abs() <= f64::MIN_POSITIVE {
        return Err(report_error(
            DEFEATURE_REPORT_INVALID,
            "Relative defeature volume requires a finite nonzero source volume",
        ));
    }
    let report = DefeatureReport {
        dry_run: options.dry_run,
        removed_features,
        changed_zones,
        volume_before_mm3: volume_before,
        volume_after_mm3: volume_after,
        abs_delta_volume_mm3,
        abs_relative_delta_volume: abs_delta_volume_mm3 / volume_before.abs(),
    };
    Ok(DefeatureRun {
        model: if options.dry_run {
            model.clone()
        } else {
            staged
        },
        report,
    })
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytic_features::analytic_fillet;

    fn budget() -> Budget {
        Budget::new(10_000_000, 8, 60_000).unwrap()
    }

    fn rounded_cuboid() -> Model {
        let sharp = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edge = sharp
            .edges
            .iter()
            .enumerate()
            .find_map(|(index, edge)| {
                let a = sharp.vertices[edge.vertices[0]].point;
                let b = sharp.vertices[edge.vertices[1]].point;
                ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12).then_some(index)
            })
            .unwrap();
        analytic_fillet(&sharp, edge, 1.).unwrap().0
    }

    fn cup() -> Model {
        crate::analytic::revolve(&[[0., 0.], [5., 0.], [5., 6.], [2., 6.], [2., 2.], [0., 2.]])
            .unwrap()
    }

    fn volume(model: &Model) -> f64 {
        crate::analysis::mass_properties(model, 1e-9, 500_000)
            .unwrap()
            .signed_volume_mm3
    }

    #[test]
    fn defeature_report_dry_run_matches_real_and_independent_volume() {
        let model = rounded_cuboid();
        let source_hash = direct_edit_model_hash(&model);
        let dry = defeature_with_report(
            &model,
            &DefeatureReportOptions {
                dry_run: true,
                ..Default::default()
            },
            &budget(),
        )
        .unwrap();
        let real =
            defeature_with_report(&model, &DefeatureReportOptions::default(), &budget()).unwrap();
        assert_eq!(dry.model, model);
        assert_eq!(direct_edit_model_hash(&dry.model), source_hash);
        assert_eq!(dry.report.removed_features, real.report.removed_features);
        assert_eq!(dry.report.changed_zones, real.report.changed_zones);
        assert_eq!(
            dry.report.abs_relative_delta_volume,
            real.report.abs_relative_delta_volume
        );
        assert_eq!(real.report.removed_features.len(), 1);
        assert!(!real.report.removed_features[0].face_ids.is_empty());
        assert!(!real.report.changed_zones[0].source_face_ids.is_empty());
        assert!(!real.report.changed_zones[0].result_face_ids.is_empty());

        let before = volume(&model);
        let after = volume(&real.model);
        let independent = (after - before).abs() / before.abs();
        assert_eq!(real.report.volume_before_mm3, before);
        assert_eq!(real.report.volume_after_mm3, after);
        assert_eq!(real.report.abs_delta_volume_mm3, (after - before).abs());
        assert_eq!(real.report.abs_relative_delta_volume, independent);
    }

    #[test]
    fn defeature_report_aggregates_hole_batch_with_persistent_zone_ids() {
        let model = cup();
        let options = DefeatureReportOptions {
            fillets: false,
            ..Default::default()
        };
        let dry = defeature_with_report(
            &model,
            &DefeatureReportOptions {
                dry_run: true,
                ..options.clone()
            },
            &budget(),
        )
        .unwrap();
        let real = defeature_with_report(&model, &options, &budget()).unwrap();
        assert_eq!(dry.report.removed_features, real.report.removed_features);
        assert_eq!(real.report.removed_features.len(), 1);
        let feature = &real.report.removed_features[0];
        assert_eq!(feature.kind, DefeatureFeatureKind::Hole);
        assert!(
            matches!(feature.size, DefeatureFeatureSize::Hole { diameter_mm, .. } if diameter_mm > 0.)
        );
        assert_eq!(real.report.changed_zones.len(), 1);
        assert_eq!(real.report.changed_zones[0].feature_indices, vec![0]);
        assert!(real.report.abs_delta_volume_mm3 > 0.);
    }

    #[test]
    fn defeature_report_json_round_trip_preserves_all_feature_sizes() {
        let model = crate::cuboid([0.; 3], [2., 2., 2.]).unwrap();
        let ids = model.1.faces.clone();
        let report = DefeatureReport {
            dry_run: true,
            removed_features: vec![
                DefeatureFeatureReport {
                    kind: DefeatureFeatureKind::Fillet,
                    face_ids: vec![ids[0]],
                    size: DefeatureFeatureSize::Fillet {
                        radius_min_mm: 0.5,
                        radius_max_mm: 0.6,
                    },
                },
                DefeatureFeatureReport {
                    kind: DefeatureFeatureKind::Hole,
                    face_ids: vec![ids[1], ids[2]],
                    size: DefeatureFeatureSize::Hole {
                        diameter_mm: 4.,
                        depth_mm: 10.,
                    },
                },
                DefeatureFeatureReport {
                    kind: DefeatureFeatureKind::Pocket,
                    face_ids: vec![ids[3]],
                    size: DefeatureFeatureSize::Pocket {
                        floor_area_mm2: 25.,
                        depth_mm: 3.,
                    },
                },
            ],
            changed_zones: vec![DefeatureChangedZone {
                feature_indices: vec![0, 1, 2],
                source_face_ids: ids[..4].to_vec(),
                result_face_ids: ids[4..].to_vec(),
            }],
            volume_before_mm3: 100.,
            volume_after_mm3: 125.,
            abs_delta_volume_mm3: 25.,
            abs_relative_delta_volume: 0.25,
        };
        let json = report.to_json().unwrap();
        assert!(json.contains("brep-defeature-report"));
        assert!(json.contains("sourceFaceIds"));
        let decoded = DefeatureReport::from_json(&json).unwrap();
        assert_eq!(decoded, report);
        assert_eq!(decoded.to_json().unwrap(), json);
    }
}
