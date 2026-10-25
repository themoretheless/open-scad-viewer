//! Atomic hole/pocket defeaturing, including a hole together with its entry
//! fillet. Every feature is staged independently: a refused operation returns
//! the unchanged input model and a typed reason, while a successful one has
//! passed topology validation, the global solid audit and a volume check.

use super::*;
use crate::aag::Aag;
use crate::aag_features::{
    find_holes, find_pockets, scan_small_features, HoleEntry, SmallFeatureKind,
    SmallFeatureOptions,
};
use crate::analysis::surface_classify::SurfaceClass;

pub const DEFEATURE_VOLUME_MISMATCH: &str = "BREP_DEFEATURE_VOLUME_MISMATCH";
pub const DEFEATURE_AUDIT_FAILED: &str = "BREP_DEFEATURE_AUDIT_FAILED";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DefeatureKind {
    Hole,
    Pocket,
}

/// Closed set of per-feature refusal reasons. Batch processing continues after
/// these; malformed global input and exhausted budgets remain ordinary errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DefeatureFailure {
    UnsupportedGeometry,
    OpenPocket,
    SharedRim,
    DivergentExtension,
    ValidationFailed,
    AuditFailed,
    VolumeCheckFailed,
    FeatureUnavailable,
}

#[derive(Clone, Debug)]
pub struct DefeatureOperationReport {
    /// Result on success, bit-identical input on a per-feature refusal.
    pub model: Model,
    pub kind: DefeatureKind,
    pub source_faces: Vec<usize>,
    pub entry_fillet_faces: Vec<usize>,
    pub removed: bool,
    pub failure: Option<DefeatureFailure>,
    pub failure_code: Option<String>,
    pub delta_volume_estimate_mm3: Option<f64>,
    pub delta_volume_actual_mm3: Option<f64>,
    pub audit: Option<SolidAuditCertificate>,
    /// Post-operation Euler–Poincaré characteristic of every shell. Its value
    /// is allowed to change: removing a through hole changes surface genus.
    pub shell_euler: Vec<i64>,
}

#[derive(Clone, Debug)]
pub struct DefeatureBatchReport {
    pub model: Model,
    pub operations: Vec<DefeatureOperationReport>,
    pub removed_count: usize,
    pub refused_count: usize,
}

#[derive(Clone, Debug)]
pub struct DefeatureOptions {
    pub holes: bool,
    pub pockets: bool,
    /// When present, only holes/pockets also reported by this small-feature
    /// scan are selected. Recognition is repeated after every successful edit.
    pub small: Option<SmallFeatureOptions>,
}

impl Default for DefeatureOptions {
    fn default() -> Self {
        Self { holes: true, pockets: true, small: None }
    }
}

fn graph(model: &Model, budget: &Budget) -> Result<Aag> {
    let mut graph = Aag::build(model, budget)?;
    graph.attach_face_attrs(model, budget)?;
    Ok(graph)
}

fn hole_faces(hole: &HoleFeature) -> BTreeSet<usize> {
    hole.faces
        .iter()
        .chain(&hole.step_faces)
        .chain(&hole.bottom_faces)
        .copied()
        .collect()
}

fn angular_span(hole: &HoleFeature) -> f64 {
    let full = 2. * std::f64::consts::PI;
    if full - hole.angle_span <= 0.35 { full } else { hole.angle_span }
}

/// Bore plus the countersink/counterbore frustum. The main and step faces
/// occupy adjacent axial intervals, so their volumes do not overlap. Radii
/// and axial levels come from authored trim vertices, not surface cages.
fn hole_estimate(model: &Model, hole: &HoleFeature) -> Result<f64> {
    let r = 0.5 * hole.diameter;
    let span = angular_span(hole);
    let mut estimate = 0.5 * span * r * r * hole.depth;
    if hole.entry == HoleEntry::Plain || hole.step_faces.is_empty() {
        return Ok(estimate);
    }
    let mut rings = Vec::<(f64, f64)>::new();
    for &face in &hole.step_faces {
        for wire in std::iter::once(model.faces[face].outer).chain(model.faces[face].holes.iter().copied()) {
            for vertex in crate::operations::loop_vertices(model, wire)? {
                let p = model.vertices[vertex].point;
                let delta = sub(p, hole.axis.point);
                let t = dot(delta, hole.axis.direction);
                let radial = norm(sub(delta, mul(hole.axis.direction, t)));
                rings.push((t, radial));
            }
        }
    }
    rings.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    rings.dedup_by(|a, b| (a.0 - b.0).abs() <= 1e-8 && (a.1 - b.1).abs() <= 1e-8);
    let scale = model.tolerance_mm.max(1e-9) * 16.;
    for pair in rings.windows(2) {
        let (t0, r0) = pair[0];
        let (t1, r1) = pair[1];
        let height = t1 - t0;
        if height <= scale {
            continue;
        }
        let outer = (r0 * r0 + r0 * r1 + r1 * r1) / 3.;
        estimate += 0.5 * span * height * outer;
    }
    if estimate.is_finite() && estimate > 0. {
        Ok(estimate)
    } else {
        Err(unsupported("Hole volume estimate is degenerate"))
    }
}

fn coaxial(a: &Axis, b: &Axis, tolerance: f64) -> bool {
    let d = sub(a.point, b.point);
    let off = sub(d, mul(a.direction, dot(d, a.direction)));
    dot(a.direction, b.direction).abs() >= 1. - 1e-7 && norm(off) <= tolerance
}

/// Concave circular entry blend: square-minus-quarter-disc, revolved by
/// Pappus. The moment form avoids a separately rounded centroid.
fn entry_fillet_estimate(
    model: &Model,
    graph: &Aag,
    hole: &HoleFeature,
    faces: &[usize],
) -> Option<f64> {
    let mut radius = None;
    for &face in faces {
        let attrs = graph.nodes.get(face)?.attrs.as_ref()?;
        let a = attrs.radius?;
        if attrs.class != SurfaceClass::Torus
            || !coaxial(attrs.axis.as_ref()?, &hole.axis, model.tolerance_mm * 100.)
            || radius.is_some_and(|r: f64| (r - a).abs() > model.tolerance_mm * 100.)
        {
            return None;
        }
        radius = Some(a);
    }
    let a = radius?;
    if !(a.is_finite() && a > 0.) {
        return None;
    }
    let mut levels = Vec::new();
    for &face in faces {
        for wire in std::iter::once(model.faces[face].outer).chain(model.faces[face].holes.iter().copied()) {
            for vertex in crate::operations::loop_vertices(model, wire).ok()? {
                levels.push(dot(sub(model.vertices[vertex].point, hole.axis.point), hole.axis.direction));
            }
        }
    }
    let min = levels.iter().copied().reduce(f64::min)?;
    let max = levels.iter().copied().reduce(f64::max)?;
    let height = max - min;
    if !(height > 0. && (height - a).abs() <= model.tolerance_mm * 100.) {
        return None;
    }
    let area = a * a * (1. - std::f64::consts::FRAC_PI_4);
    let r = 0.5 * hole.diameter;
    let moment = r * area + a.powi(3) * (5. / 6. - std::f64::consts::FRAC_PI_4);
    let bore_continuation = 0.5 * r * r * height;
    Some(angular_span(hole) * (bore_continuation + moment))
}

/// A ring fillet is represented by sibling torus patches and is not required
/// to form one of the open, two-cap chains recognized by `find_fillet_chains`.
/// Detect its patches directly from tangential AAG adjacency to the bore.
fn entry_fillet_faces(
    graph: &Aag,
    hole: &HoleFeature,
    budget: &Budget,
) -> Result<Vec<usize>> {
    let core = hole_faces(hole);
    let mut guard = budget.guard("defeature-entry-fillets");
    let mut faces = Vec::new();
    for (face, node) in graph.nodes.iter().enumerate() {
        guard.tick()?;
        if core.contains(&face)
            || node.attrs.as_ref().is_none_or(|attrs| attrs.class != SurfaceClass::Torus)
            || !node.neighbors.iter().any(|neighbor| core.contains(neighbor))
        {
            continue;
        }
        // Recognition requires smooth contact; a sharp toroidal feature is
        // not an entry blend and must not be swept into the deletion cluster.
        if !node.edges.iter().any(|edge| {
            let edge = &graph.edges[*edge];
            matches!(edge.class, DihedralClass::Smooth | DihedralClass::Tangent)
                && edge.uses.iter().any(|usage| core.contains(&usage.face))
        }) {
            continue;
        }
        faces.push(face);
    }
    Ok(faces)
}

fn map_failure(error: &Error) -> DefeatureFailure {
    match error.code {
        SUPPRESS_DIVERGENT => DefeatureFailure::DivergentExtension,
        DIRECT_EDIT_AUDIT_REFUSED => DefeatureFailure::AuditFailed,
        SUPPRESS_UNSUPPORTED => {
            if error.message.contains("shared rim") {
                DefeatureFailure::SharedRim
            } else {
                DefeatureFailure::UnsupportedGeometry
            }
        }
        _ => DefeatureFailure::ValidationFailed,
    }
}

fn refused(
    model: &Model,
    kind: DefeatureKind,
    source_faces: Vec<usize>,
    entry_fillet_faces: Vec<usize>,
    failure: DefeatureFailure,
    code: impl Into<String>,
) -> DefeatureOperationReport {
    DefeatureOperationReport {
        model: model.clone(),
        kind,
        source_faces,
        entry_fillet_faces,
        removed: false,
        failure: Some(failure),
        failure_code: Some(code.into()),
        delta_volume_estimate_mm3: None,
        delta_volume_actual_mm3: None,
        audit: None,
        shell_euler: Vec::new(),
    }
}

fn run(
    model: &Model,
    graph: &Aag,
    feature: &FeatureRef,
    budget: &Budget,
) -> Result<DefeatureOperationReport> {
    let (kind, core) = match feature {
        FeatureRef::Hole(hole) => (DefeatureKind::Hole, hole_faces(hole)),
        FeatureRef::Pocket(pocket) => {
            if pocket.kind != PocketKind::Closed {
                return Ok(refused(
                    model,
                    DefeatureKind::Pocket,
                    pocket.faces.clone(),
                    Vec::new(),
                    DefeatureFailure::OpenPocket,
                    SUPPRESS_UNSUPPORTED,
                ));
            }
            (DefeatureKind::Pocket, pocket.faces.iter().copied().collect())
        }
        FeatureRef::FilletChain(_) => {
            return Err(error(SUPPRESS_INVALID, "Defeature accepts only holes and pockets"));
        }
    };
    let source_faces = core.iter().copied().collect::<Vec<_>>();
    let entry_fillet_faces = match feature {
        FeatureRef::Hole(hole) => entry_fillet_faces(graph, hole, budget)?,
        _ => Vec::new(),
    };

    let estimate = match feature {
        FeatureRef::Pocket(pocket) => pocket.floor_area * pocket.depth,
        FeatureRef::Hole(hole) => {
            let base = match hole_estimate(model, hole) {
                Ok(value) => value,
                Err(error) => {
                    return Ok(refused(model, kind, source_faces, entry_fillet_faces, map_failure(&error), error.code));
                }
            };
            if entry_fillet_faces.is_empty() {
                base
            } else {
                let Some(extra) = entry_fillet_estimate(model, graph, hole, &entry_fillet_faces) else {
                    return Ok(refused(
                        model,
                        kind,
                        source_faces,
                        entry_fillet_faces,
                        DefeatureFailure::UnsupportedGeometry,
                        SUPPRESS_UNSUPPORTED,
                    ));
                };
                base + extra
            }
        }
        FeatureRef::FilletChain(_) => unreachable!(),
    };

    let outcome = if entry_fillet_faces.is_empty() {
        suppress_feature(model, feature, budget)
    } else {
        let cluster = core
            .iter()
            .chain(entry_fillet_faces.iter())
            .copied()
            .collect::<BTreeSet<_>>();
        (|| {
            let snapshot = ModelSnapshot::new(model.clone())?;
            let mut guard = budget.guard("defeature-hole-entry-fillet");
            let (staged, removed_edges) =
                super::suppression::suppress_by_deletion(snapshot.model(), &cluster, &mut guard)?;
            staged.validate()?;
            let audit = crate::solid_audit::audit_solid(&staged)?;
            let before = crate::analysis::mass_properties(snapshot.model(), 1e-6, 500_000)?.signed_volume_mm3;
            let after = crate::analysis::mass_properties(&staged, 1e-6, 500_000)?.signed_volume_mm3;
            guard.check()?;
            Ok(SuppressReport {
                model: staged,
                kind: "hole",
                suppressed_faces: cluster.iter().copied().collect(),
                removed_edges,
                new_sharp_edges: 0,
                volume_before_mm3: before,
                volume_after_mm3: after,
                delta_volume_estimate_mm3: estimate,
                delta_volume_actual_mm3: after - before,
                audit,
            })
        })()
    };
    let report = match outcome {
        Ok(report) => report,
        Err(error) => {
            if matches!(error.code, "NURBS_RESOURCE_LIMIT" | "BREP_RESOURCE_LIMIT") {
                return Err(error);
            }
            return Ok(refused(model, kind, source_faces, entry_fillet_faces, map_failure(&error), error.code));
        }
    };
    // Curved entry blends need tighter quadrature than the general-purpose
    // suppression report to make the promised one-percent volume gate useful.
    let before = crate::analysis::mass_properties(model, 1e-9, 500_000)?.signed_volume_mm3;
    let after = crate::analysis::mass_properties(&report.model, 1e-9, 500_000)?.signed_volume_mm3;
    let actual = after - before;
    let tolerance = 0.01 * estimate.abs().max(model.tolerance_mm.powi(3));
    if !actual.is_finite() || (actual - estimate).abs() > tolerance {
        let mut refusal = refused(
            model,
            kind,
            source_faces,
            entry_fillet_faces,
            DefeatureFailure::VolumeCheckFailed,
            DEFEATURE_VOLUME_MISMATCH,
        );
        refusal.delta_volume_estimate_mm3 = Some(estimate);
        refusal.delta_volume_actual_mm3 = Some(actual);
        return Ok(refusal);
    }
    let mut shell_euler = Vec::with_capacity(report.model.shells.len());
    for shell in 0..report.model.shells.len() {
        let (Some(chi), Some(genus)) = (
            report.model.shell_euler_characteristic(shell),
            report.model.shell_genus(shell),
        ) else {
            return Ok(refused(
                model,
                kind,
                source_faces,
                entry_fillet_faces,
                DefeatureFailure::AuditFailed,
                DEFEATURE_AUDIT_FAILED,
            ));
        };
        if chi != 2 - 2 * genus {
            return Ok(refused(
                model,
                kind,
                source_faces,
                entry_fillet_faces,
                DefeatureFailure::AuditFailed,
                DEFEATURE_AUDIT_FAILED,
            ));
        }
        shell_euler.push(chi);
    }
    Ok(DefeatureOperationReport {
        model: report.model,
        kind,
        source_faces,
        entry_fillet_faces,
        removed: true,
        failure: None,
        failure_code: None,
        delta_volume_estimate_mm3: Some(estimate),
        delta_volume_actual_mm3: Some(actual),
        audit: Some(report.audit),
        shell_euler,
    })
}

/// Remove one recognized hole or closed pocket. A geometric refusal is data in
/// the report, not an exceptional batch-aborting error; `model` is unchanged.
pub fn defeature_hole_pocket(
    model: &Model,
    feature: &FeatureRef,
    budget: &Budget,
) -> Result<DefeatureOperationReport> {
    model.validate()?;
    let graph = graph(model, budget)?;
    run(model, &graph, feature, budget)
}

fn fingerprint(kind: DefeatureKind, faces: &[usize]) -> (DefeatureKind, Vec<usize>) {
    (kind, faces.to_vec())
}

/// Batch deletion with re-recognition after every successful edit. This keeps
/// all indices current and also suppresses duplicate hole/pocket templates.
pub fn defeature_holes_and_pockets(
    model: &Model,
    options: &DefeatureOptions,
    budget: &Budget,
) -> Result<DefeatureBatchReport> {
    model.validate()?;
    let mut current = model.clone();
    let mut operations = Vec::new();
    let mut skipped = BTreeSet::new();
    loop {
        let guard = budget.guard("defeature-batch");
        guard.check()?;
        let graph = graph(&current, budget)?;
        let small = options
            .small
            .as_ref()
            .map(|options| scan_small_features(&current, &graph, options, budget))
            .transpose()?;
        let mut candidates = Vec::<FeatureRef>::new();
        if options.holes {
            candidates.extend(find_holes(&current, &graph, budget)?.into_iter().map(FeatureRef::Hole));
        }
        if options.pockets {
            candidates.extend(find_pockets(&current, &graph, budget)?.into_iter().map(FeatureRef::Pocket));
        }
        let mut selected = None;
        for feature in candidates {
            let (kind, faces) = match &feature {
                FeatureRef::Hole(hole) => (DefeatureKind::Hole, hole_faces(hole).into_iter().collect()),
                FeatureRef::Pocket(pocket) => (DefeatureKind::Pocket, pocket.faces.clone()),
                FeatureRef::FilletChain(_) => continue,
            };
            if skipped.contains(&fingerprint(kind, &faces)) {
                continue;
            }
            if let Some(scan) = &small {
                let wanted = match kind {
                    DefeatureKind::Hole => SmallFeatureKind::Hole,
                    DefeatureKind::Pocket => SmallFeatureKind::Pocket,
                };
                let faces = faces.iter().copied().collect::<BTreeSet<_>>();
                if !scan.candidates.iter().any(|candidate| {
                    candidate.kind == wanted
                        && candidate.faces.iter().any(|face| faces.contains(face))
                }) {
                    skipped.insert(fingerprint(kind, &faces.into_iter().collect::<Vec<_>>()));
                    continue;
                }
            }
            selected = Some((feature, kind, faces));
            break;
        }
        let Some((feature, kind, faces)) = selected else { break };
        let report = run(&current, &graph, &feature, budget)?;
        if report.removed {
            current = report.model.clone();
            operations.push(report);
            skipped.clear();
        } else {
            skipped.insert(fingerprint(kind, &faces));
            operations.push(report);
        }
    }
    let removed_count = operations.iter().filter(|report| report.removed).count();
    let refused_count = operations.len() - removed_count;
    Ok(DefeatureBatchReport { model: current, operations, removed_count, refused_count })
}


#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> Budget {
        Budget::new(10_000_000, 8, 60_000).unwrap()
    }

    fn cup() -> Model {
        crate::analytic::revolve(&[
            [0., 0.], [5., 0.], [5., 6.], [2., 6.], [2., 2.], [0., 2.],
        ])
        .unwrap()
    }

    fn blended_cup() -> Model {
        let profile = [
            [0., 0.], [6., 0.], [6., 6.], [2., 6.], [2., 2.], [0., 2.],
        ];
        let source = crate::analytic::revolve(&profile).unwrap();
        let mut curves = crate::imprint_pipeline::rounded_profile_curves(
            &source,
            &profile,
            &[false, false, false, true, false, false],
            1.,
            0.,
        )
        .unwrap();
        for curve in &mut curves {
            for point in &mut curve.control_points {
                point.truncate(2);
            }
        }
        crate::revolve_wire(&curves, source.tolerance_mm).unwrap()
    }

    fn assert_success(model: &Model, expected: f64) -> DefeatureOperationReport {
        let graph = graph(model, &budget()).unwrap();
        let holes = find_holes(model, &graph, &budget()).unwrap();
        assert_eq!(holes.len(), 1, "{holes:?}");
        let report = defeature_hole_pocket(
            model,
            &FeatureRef::Hole(holes[0].clone()),
            &budget(),
        )
        .unwrap();
        assert!(report.removed, "expected {expected}, hole {:?}: {:?}: {:?}; estimate {:?}, actual {:?}", holes[0], report.failure, report.failure_code, report.delta_volume_estimate_mm3, report.delta_volume_actual_mm3);
        let actual = report.delta_volume_actual_mm3.unwrap();
        let estimate = report.delta_volume_estimate_mm3.unwrap();
        assert!((estimate - expected).abs() <= 0.01 * expected, "estimate {estimate} != {expected}");
        assert!((actual - expected).abs() <= 0.01 * expected, "actual {actual} != {expected}");
        assert!(report.audit.as_ref().unwrap().ok);
        assert_eq!(report.shell_euler.len(), report.model.shells.len());
        report.model.validate().unwrap();
        report
    }

    #[test]
    fn defeature_hole_pocket_corpus_volume_and_euler() {
        let block = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let drill = crate::transform::affine(
            &crate::cylinder(2., 14.).unwrap(),
            [
                [1., 0., 0., 5.],
                [0., 1., 0., 5.],
                [0., 0., 1., -2.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let through = crate::boolean(&block, &drill, "difference").unwrap();
        let countersink = crate::analytic::revolve(&[
            [0., 0.], [5., 0.], [5., 6.], [4., 6.], [2., 4.], [2., 2.], [0., 2.],
        ])
        .unwrap();
        let counterbore = crate::analytic::revolve(&[
            [0., 0.], [5., 0.], [5., 6.], [4., 6.], [4., 4.], [2., 4.], [2., 2.], [0., 2.],
        ])
        .unwrap();
        let pi = std::f64::consts::PI;
        let cases = [
            (through, 40. * pi),
            (cup(), 16. * pi),
            (countersink, 80. * pi / 3.),
            (counterbore, 40. * pi),
        ];
        let mut passed = 0;
        for (model, expected) in &cases {
            assert_success(model, *expected);
            passed += 1;
        }
        assert_eq!(100 * passed / cases.len(), 100);

        let pocket_model = crate::boolean(
            &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
            &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
            "difference",
        )
        .unwrap();
        let graph = graph(&pocket_model, &budget()).unwrap();
        let pockets = find_pockets(&pocket_model, &graph, &budget()).unwrap();
        let report = defeature_hole_pocket(
            &pocket_model,
            &FeatureRef::Pocket(pockets[0].clone()),
            &budget(),
        )
        .unwrap();
        assert!(report.removed, "{:?}", report.failure);
        let expected = 36. * 26. * 18.;
        assert!((report.delta_volume_actual_mm3.unwrap() - expected).abs() <= 0.01 * expected);
        assert_eq!(report.shell_euler, vec![2]);
    }

    #[test]
    fn defeature_hole_and_entry_fillet_is_atomic() {
        let model = blended_cup();
        let source_graph = graph(&model, &budget()).unwrap();
        let holes = find_holes(&model, &source_graph, &budget()).unwrap();
        assert_eq!(holes.len(), 1, "{holes:?}");
        let entry_faces = entry_fillet_faces(&source_graph, &holes[0], &budget()).unwrap();
        assert!(!entry_faces.is_empty(), "no entry fillet patches");
        assert!(entry_fillet_estimate(&model, &source_graph, &holes[0], &entry_faces).is_some());
        let before_hash = direct_edit_model_hash(&model);
        let report = defeature_hole_pocket(
            &model,
            &FeatureRef::Hole(holes[0].clone()),
            &budget(),
        )
        .unwrap();
        assert!(report.removed, "{:?}: {:?}; estimate {:?}, actual {:?}; hole {:?}", report.failure, report.failure_code, report.delta_volume_estimate_mm3, report.delta_volume_actual_mm3, holes[0]);
        assert!(!report.entry_fillet_faces.is_empty());
        let pi = std::f64::consts::PI;
        let expected = 16. * pi + 2. * pi * (2. * (1. - pi / 4.) + 5. / 6. - pi / 4.);
        assert!((report.delta_volume_estimate_mm3.unwrap() - expected).abs() <= 0.01 * expected);
        assert!((report.delta_volume_actual_mm3.unwrap() - expected).abs() <= 0.01 * expected);
        let after_graph = graph(&report.model, &budget()).unwrap();
        assert!(find_holes(&report.model, &after_graph, &budget()).unwrap().is_empty());
        assert!(crate::aag_fillets::find_fillet_chains(&report.model, &after_graph, &budget())
            .unwrap()
            .is_empty());
        assert_eq!(direct_edit_model_hash(&model), before_hash);
    }

    #[test]
    fn defeature_batch_small_filter_rerecognizes() {
        let model = cup();
        let options = DefeatureOptions {
            small: Some(SmallFeatureOptions {
                hole_diameter_max: Some(3.),
                ..SmallFeatureOptions::default()
            }),
            ..DefeatureOptions::default()
        };
        let rejected = defeature_holes_and_pockets(&model, &options, &budget()).unwrap();
        assert_eq!(rejected.removed_count, 0);
        assert_eq!(rejected.operations.len(), 0);
        assert_eq!(rejected.model, model);

        let options = DefeatureOptions {
            small: Some(SmallFeatureOptions {
                hole_diameter_max: Some(5.),
                ..SmallFeatureOptions::default()
            }),
            ..DefeatureOptions::default()
        };
        let report = defeature_holes_and_pockets(&model, &options, &budget()).unwrap();
        assert_eq!(report.removed_count, 1, "{:?}", report.operations);
        assert!(find_holes(&report.model, &graph(&report.model, &budget()).unwrap(), &budget())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn defeature_open_pocket_refusal_rolls_back_hash() {
        let model = crate::boolean(
            &crate::cuboid([0.; 3], [10., 10., 10.]).unwrap(),
            &crate::cuboid([0., 3., 6.], [7., 7., 12.]).unwrap(),
            "difference",
        )
        .unwrap();
        let graph = graph(&model, &budget()).unwrap();
        let pocket = find_pockets(&model, &graph, &budget())
            .unwrap()
            .into_iter()
            .find(|pocket| pocket.kind == PocketKind::Open)
            .unwrap();
        let hash = direct_edit_model_hash(&model);
        let report = defeature_hole_pocket(
            &model,
            &FeatureRef::Pocket(pocket),
            &budget(),
        )
        .unwrap();
        assert!(!report.removed);
        assert_eq!(report.failure, Some(DefeatureFailure::OpenPocket));
        assert_eq!(direct_edit_model_hash(&report.model), hash);
        assert_eq!(direct_edit_model_hash(&model), hash);
    }
}
