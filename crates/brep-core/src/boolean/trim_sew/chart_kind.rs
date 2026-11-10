use super::*;

pub(super) fn refuse(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

pub(super) fn push_usize(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Byte-exact NURBS curve signature for identity derivation; no Debug
/// serialization of the whole curve into a String.
pub(super) fn curve_signature_bytes(curve: &Curve) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_usize(&mut bytes, curve.degree);
    push_usize(&mut bytes, curve.knots.len());
    for knot in &curve.knots {
        bytes.extend_from_slice(&knot.to_bits().to_le_bytes());
    }
    push_usize(&mut bytes, curve.control_points.len());
    for point in &curve.control_points {
        push_usize(&mut bytes, point.len());
        for coordinate in point {
            bytes.extend_from_slice(&coordinate.to_bits().to_le_bytes());
        }
    }
    push_usize(&mut bytes, curve.weights.len());
    for weight in &curve.weights {
        bytes.extend_from_slice(&weight.to_bits().to_le_bytes());
    }
    bytes.push(u8::from(curve.periodic));
    bytes
}

pub(super) fn point_signature_bytes(point: &[f64; 3]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(24);
    for value in point {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    bytes
}

/// Deterministic sew-key authority discriminator hashed from definition bytes,
/// not from a Debug dump of the rational definition.
pub(super) fn authority_key(authority: &CurveAuthority) -> String {
    match authority {
        CurveAuthority::Rational(definition) => {
            let mut bytes = Vec::new();
            push_usize(&mut bytes, definition.degree);
            push_usize(&mut bytes, definition.knots.len());
            for knot in &definition.knots {
                bytes.extend_from_slice(&knot.to_le_bytes());
            }
            push_usize(&mut bytes, definition.control_points.len());
            for point in &definition.control_points {
                push_usize(&mut bytes, point.len());
                for coordinate in point {
                    bytes.extend_from_slice(&coordinate.to_le_bytes());
                }
            }
            push_usize(&mut bytes, definition.weights.len());
            for weight in &definition.weights {
                bytes.extend_from_slice(&weight.to_le_bytes());
            }
            bytes.push(u8::from(definition.periodic));
            TopoId::derive(TopoKind::Edge, "sew-key", "authority", "rational", &bytes).to_string()
        }
        CurveAuthority::CanonicalConstruction(identity) => identity.clone(),
    }
}

/// O(E) vertex → incident edge indices; edge.vertices never mutate during a
/// heal plan, so one build serves the whole plan application.
pub(super) fn vertex_edge_incidence(model: &Model) -> Vec<Vec<usize>> {
    let mut incidence = vec![Vec::new(); model.vertices.len()];
    for (index, edge) in model.edges.iter().enumerate() {
        for vertex in edge.vertices {
            incidence[vertex].push(index);
        }
    }
    incidence
}

pub(super) type EdgeUse = (usize, usize, usize, bool, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChartKind {
    PlanePoly,
    AnalyticCircle,
    /// Finite tensor rectangle containing only its four exact boundaries and
    /// strict-interior constant-U/V Bezier graph traces.
    TensorBezierGraph,
    /// Admitted freeform chart touch for UV arrange walking slice (F3).
    /// Out-of-matrix freeform DCEL still refuses Complete without strata.
    Freeform,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartEvent {
    pub parameter: f64,
    pub kind: &'static str,
    pub edge: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellLabel {
    Outside,
    Inside,
    Boundary,
}

#[derive(Clone, Debug)]
pub struct ClassificationCertificate {
    pub chart: ChartKind,
    pub events: Vec<ChartEvent>,
    pub cells: Vec<(f64, f64, CellLabel)>,
    pub complete: bool,
}

/// Event order on a frozen chart: parameters must be strictly increasing except
/// for explicit coincident boundary strata which are labeled, never merged.
pub fn classify_chart_events(
    chart: ChartKind,
    mut events: Vec<ChartEvent>,
    sample_points: &[(f64, CellLabel)],
) -> Result<ClassificationCertificate> {
    if events.len() > 4096 || sample_points.len() > 4096 {
        return Err(refuse(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Chart event/sample budget exceeded",
        ));
    }
    events.sort_by(|a, b| {
        a.parameter
            .partial_cmp(&b.parameter)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.edge.cmp(&b.edge))
    });
    for window in events.windows(2) {
        let a = &window[0];
        let b = &window[1];
        if !a.parameter.is_finite() || !b.parameter.is_finite() {
            return Err(refuse(
                "BREP_TRIM_INVALID",
                "Chart event parameter must be finite",
            ));
        }
        if (a.parameter - b.parameter).abs() <= 1e-15 && a.edge == b.edge {
            return Err(refuse(
                "BREP_TRIM_AMBIGUOUS",
                "Duplicate chart event on the same edge without stratum label",
            ));
        }
    }
    let mut cells = Vec::new();
    for (i, point) in sample_points.iter().enumerate() {
        if !point.0.is_finite() {
            return Err(refuse(
                "BREP_TRIM_INVALID",
                "Classification sample must be finite",
            ));
        }
        // Root-isolated: sample must not sit on an event unless Boundary.
        let on_event = events
            .iter()
            .any(|e| (e.parameter - point.0).abs() <= 1e-12);
        if on_event && point.1 != CellLabel::Boundary {
            return Err(refuse(
                "BREP_TRIM_AMBIGUOUS",
                "Interior/exterior sample coincides with an event root",
            ));
        }
        if !on_event && point.1 == CellLabel::Boundary {
            return Err(refuse(
                "BREP_TRIM_AMBIGUOUS",
                "Boundary label without an isolating event",
            ));
        }
        let lo = if i == 0 {
            f64::NEG_INFINITY
        } else {
            sample_points[i - 1].0
        };
        cells.push((lo, point.0, point.1));
    }
    Ok(ClassificationCertificate {
        chart,
        events,
        cells,
        complete: true,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SewEdgeKey {
    pub a: [i64; 3],
    pub b: [i64; 3],
    /// Curve-shape discriminator. Closed-shell auditing fills this with the
    /// exact NURBS midpoint so distinct arcs sharing endpoints never collapse.
    pub mid: Option<[i64; 3]>,
    /// Complete rational-definition discriminator for proof-backed sewing.
    pub definition: Option<String>,
    /// Shell discriminator prevents touching but disconnected material lumps
    /// from being welded by coincident geometric coordinates.
    pub shell: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplacementOp {
    None,
    Refit,
    Split,
}

/// Optional displacement record for an authorized heal plan (never silent).
#[derive(Clone, Debug)]
pub struct DisplacementRecord {
    pub before: SewEdgeKey,
    pub after: SewEdgeKey,
    pub bound: i64,
    pub op: DisplacementOp,
}

#[derive(Clone, Debug)]
pub struct SewLedgerEntry {
    pub key: SewEdgeKey,
    pub face_a: usize,
    pub face_b: usize,
    pub orientation_agree: bool,
    pub displacement: Option<DisplacementRecord>,
}

#[derive(Clone, Debug, Default)]
pub struct SewSnapshot {
    pub edges: BTreeSet<SewEdgeKey>,
    pub oriented: BTreeMap<SewEdgeKey, bool>,
    pub displacements: Vec<DisplacementRecord>,
}

#[derive(Clone, Debug)]
pub struct SewCertificate {
    pub matched: usize,
    pub complete: bool,
    pub displacement_budget_ok: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvePcurveSupport {
    CurvedIsoU,
    CurvedIsoV,
    AffinePlanar,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CurvePcurveAuthority {
    pub(super) curve: RationalCurveDefinition,
    pub(super) pcurve: RationalCurveDefinition,
    pub(super) support: CurvePcurveSupport,
    pub(super) parameter_domain_bits: [u64; 2],
    pub(super) orientation: ParameterOrientation,
    pub(super) owner: BoundaryUse,
    pub(super) context: ToleranceSpecIdentity,
}

/// Exact 3D rational-curve/pcurve authority for a single owned support use.
/// No sampled residual or endpoint snap participates in authorization.
#[derive(Clone, Debug)]
pub struct CurvePcurveCorrespondence {
    pub curve: Curve,
    pub pcurve: Curve,
    pub support: CurvePcurveSupport,
    pub parameter_domain_bits: [u64; 2],
    pub orientation: ParameterOrientation,
    pub owner: BoundaryUse,
    pub context: ToleranceSpecIdentity,
    pub evidence: ComposedEvidence,
    pub no_snapping: bool,
    pub(super) authority: CurvePcurveAuthority,
}

impl CurvePcurveCorrespondence {
    pub fn permits_exact_correspondence(&self) -> bool {
        let Ok(curve) = RationalCurveDefinition::from_curve(&self.curve) else {
            return false;
        };
        let Ok(pcurve) = RationalCurveDefinition::from_curve(&self.pcurve) else {
            return false;
        };
        self.no_snapping
            && self.context == self.evidence.context
            && self
                .evidence
                .claims
                .iter()
                .any(|claim| matches!(claim, EvidenceClaim::Correspondence { .. }))
            && self.evidence.claims.iter().any(|claim| {
                matches!(
                    claim,
                    EvidenceClaim::TopologyPreservation { invariant }
                        if invariant == "curve_pcurve_parameter_orientation_context_owner"
                )
            })
            && self.authority
                == CurvePcurveAuthority {
                    curve,
                    pcurve,
                    support: self.support,
                    parameter_domain_bits: self.parameter_domain_bits,
                    orientation: self.orientation,
                    owner: self.owner.clone(),
                    context: self.context.clone(),
                }
    }
}

/// Exact curve authority. Canonical construction identities are reserved for
/// constructors which guarantee the same rational definition by contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CurveAuthority {
    Rational(RationalCurveDefinition),
    CanonicalConstruction(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterOrientation {
    Same,
    Reversed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundaryUse {
    pub face: usize,
    pub wire: usize,
    pub cyclic_index: usize,
    pub reversed: bool,
}

pub struct BoundaryCorrespondenceInput<'a> {
    pub curve_a: &'a Curve,
    pub curve_b: &'a Curve,
    pub endpoints_a: [[f64; 3]; 2],
    pub endpoints_b: [[f64; 3]; 2],
    pub orientation: ParameterOrientation,
    pub seam_shift: i32,
    pub shell: usize,
    pub uses: [BoundaryUse; 2],
}

/// Proof that two complete boundary uses correspond. Every field is checked by
/// exact sew; no endpoint/midpoint quantization is an authority.
#[derive(Clone, Debug)]
pub struct BoundaryCorrespondence {
    pub context: ToleranceSpecIdentity,
    pub authority: CurveAuthority,
    pub endpoints_bits: [[[u64; 3]; 2]; 2],
    pub parameter_domain_bits: [u64; 2],
    pub orientation: ParameterOrientation,
    /// Integer periodic shift in complete periods. Zero for non-periodic curves.
    pub seam_shift: i32,
    pub shell: usize,
    pub uses: [BoundaryUse; 2],
    pub evidence: ComposedEvidence,
    /// Native-only binding to the complete serialized heal recipe. Generic sew
    /// proofs leave this absent and therefore cannot authorize mutation.
    pub(super) heal_recipe: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CorrespondenceLedgerEntry {
    pub key: SewEdgeKey,
    pub correspondence: BoundaryCorrespondence,
}

/// One narrowly authorized geometric repair. Construction is only available
/// through `AuthorizedHealPlan::new`, which binds every recipe to one context.
#[derive(Clone, Debug)]
pub enum HealOperation {
    EndpointSnap {
        vertex: usize,
        expected_id: TopoId,
        to: [f64; 3],
        correspondence: BoundaryCorrespondence,
    },
    CurveRefit {
        edge: usize,
        expected_id: TopoId,
        replacement: Curve,
        correspondence: BoundaryCorrespondence,
    },
    /// Split is represented explicitly, but this kernel slice refuses it until
    /// a caller supplies topology-preserving child-loop authorship.
    EdgeSplit {
        edge: usize,
        expected_id: TopoId,
        parameter: f64,
        correspondence: BoundaryCorrespondence,
    },
}
