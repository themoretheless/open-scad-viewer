//! Complete support-product coverage for aligned chords. Same-face products
//! are included; unresolved diagonal neighborhoods never disappear.
use super::*;
/// Owner-bound lower bound for endpoint-normal-aligned chords only.
/// This is not an unconstrained minimum thickness or an attained witness.
pub struct ExpressionAlignedChordMinimum<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    geometry: &'v EmbeddedExpressionGeometry<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>,
    minimum_mm: f64,
    max_sine_squared: f64,
}
impl ExpressionAlignedChordMinimum<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn geometry(&self) -> &EmbeddedExpressionGeometry<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
        self.geometry
    }
    pub fn minimum_mm(&self) -> f64 {
        self.minimum_mm
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
}
pub struct ExpressionWallQualification<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub report: ExpressionWallReport,
    pub certificate: Option<ExpressionAlignedChordMinimum<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>>,
}
impl<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>
    EmbeddedExpressionGeometry<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>
{
    /// Fresh coverage is mandatory; mutable caller reports never admit bounds.
    pub fn qualify_minimum_aligned_chords<'v>(
        &'v self,
        minimum_mm: f64,
        max_sine_squared: f64,
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ExpressionWallQualification<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>> {
        self.qualify_trimmed_minimum_aligned_chords(
            minimum_mm,
            max_sine_squared,
            tolerance_uv,
            controls,
            max_cells,
            max_spans,
            0,
        )
    }
    /// Fresh whole-face coverage including proven outside-trim exclusions.
    pub fn qualify_trimmed_minimum_aligned_chords<'v>(
        &'v self,
        minimum_mm: f64,
        max_sine_squared: f64,
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
        max_domain_cells: usize,
    ) -> Result<ExpressionWallQualification<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>> {
        let report = self.inspect_trimmed_minimum_aligned_chords(
            minimum_mm,
            max_sine_squared,
            tolerance_uv,
            controls,
            max_cells,
            max_spans,
            max_domain_cells,
        )?;
        let certificate = if report.complete && report.pending.is_empty() {
            Some(ExpressionAlignedChordMinimum {
                geometry: self,
                minimum_mm,
                max_sine_squared,
            })
        } else {
            None
        };
        Ok(ExpressionWallQualification {
            report,
            certificate,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpressionWallPendingReason {
    Budget,
    Enclosure,
    Resolution,
}
impl ExpressionWallPendingReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Budget => "budget",
            Self::Enclosure => "enclosure",
            Self::Resolution => "resolution",
        }
    }
}
pub struct ExpressionWallCell {
    pub faces: [usize; 2],
    pub domains: [[[f64; 2]; 2]; 2],
    pub pending_reason: Option<ExpressionWallPendingReason>,
}
#[cfg(feature = "codec")]
impl value_codec::Serialize for ExpressionWallPendingReason {
    fn to_value(&self) -> value_codec::Value {
        value_codec::Serialize::to_value(self.as_str())
    }
}
#[cfg(feature = "codec")]
impl value_codec::Serialize for ExpressionWallCell {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"faces":self.faces,"domains":self.domains,"pending_reason":self.pending_reason})
    }
}
fn product_key(cell: &ExpressionWallCell) -> ([usize; 2], [[[u64; 2]; 2]; 2]) {
    let mut domains = cell.domains.map(|uv| uv.map(|d| d.map(f64::to_bits)));
    if cell.faces[0] == cell.faces[1] && domains[1] < domains[0] {
        domains.swap(0, 1);
    }
    (cell.faces, domains)
}
// A whole-rectangle exclusion covers every pair of endpoints in any
// contained rectangle. Both certificates belong to this immutable face and
// this inspection's angular policy; incomplete/negative results never enter.
fn rectangle_contains(outer: [[f64; 2]; 2], inner: [[f64; 2]; 2]) -> bool {
    (0..2).all(|axis| {
        outer[axis]
            .iter()
            .chain(inner[axis].iter())
            .all(|v| v.is_finite())
            && outer[axis][0] <= inner[axis][0]
            && inner[axis][0] <= inner[axis][1]
            && inner[axis][1] <= outer[axis][1]
    })
}
#[cfg(test)]
mod product_tests {
    use super::*;
    #[test]
    fn intrinsic_exclusion_reuse_requires_exact_containment() {
        let outer = [[0., 1.], [0.25, 0.75]];
        assert!(rectangle_contains(outer, [[0.1, 0.9], [0.25, 0.5]]));
        assert!(rectangle_contains(outer, outer));
        assert!(!rectangle_contains(outer, [[0., 1.], [0.25, f64::from_bits(0.75f64.to_bits()+1)]]));
        assert!(!rectangle_contains(outer, [[0.9, 0.1], [0.25, 0.5]]));
        assert!(!rectangle_contains(outer, [[f64::NAN, 1.], [0.25, 0.5]]));
        assert!(!rectangle_contains(outer, [[0., f64::INFINITY], [0.25, 0.5]]));
    }
    #[test]
    fn diagnostic_wire_preserves_domains_reasons_and_support_scope() {
        let report=ExpressionWallReport {
            minimum_mm:1.,max_sine_squared:1e-4,tolerance_uv:0.05,
            complete:false,
            pending:vec![ExpressionWallCell {faces:[2,3],domains:[[[0.,0.5],[0.2,0.4]];2],
                pending_reason:Some(ExpressionWallPendingReason::Budget)}],
            cells:0,spans:122,span_stages:ExpressionWallSpanStages {position:2,normal:3,shared_corner:5,original_pair:7,retained_pair:11,corrected_pair:13},distance_excluded:0,angle_excluded:0,planar_excluded:0,
            profile_transition_checked:2,profile_transition_excluded:1,profile_transition_spans:64,
            profile_transition_inconclusive:1,profile_transition_unresolved:0,
            intrinsic_checked:3,intrinsic_spans:17,intrinsic_unresolved:2,
            intrinsic_excluded:0,plane_controls:0,cache_hits:0,total_pairs:1,
            initial_pairs_checked:0,symmetry_reused:0,trim_excluded:0,domain_cells:0,
            budget_pending:1,enclosure_pending:0,resolution_pending:0,
        };
        let value=value_codec::Serialize::to_value(&report);
        let encoded=value_codec::to_string(&report).unwrap();
        let decoded:value_codec::Value=value_codec::from_str(&encoded).unwrap();
        assert_eq!(decoded,value);
        assert_eq!(value["spans"],value_codec::json!(122));
        for (stage,spans) in [("position",2),("normal",3),("shared_corner",5),
            ("original_pair",7),("retained_pair",11),("corrected_pair",13)] {
            assert_eq!(value["span_stages"][stage],value_codec::json!(spans));
        }
        assert_eq!(value["intrinsic_checked"],value_codec::json!(3));
        assert_eq!(value["intrinsic_spans"],value_codec::json!(17));
        assert_eq!(value["intrinsic_unresolved"],value_codec::json!(2));
        assert_eq!(value["profile_transition_checked"],value_codec::json!(2));
        assert_eq!(value["profile_transition_excluded"],value_codec::json!(1));
        assert_eq!(value["profile_transition_spans"],value_codec::json!(64));
        assert_eq!(value["profile_transition_inconclusive"],value_codec::json!(1));
        assert_eq!(value["profile_transition_unresolved"],value_codec::json!(0));
        assert_eq!(value["scope"],value_codec::json!("original-retained-transition-support-products"));
        assert_eq!(value["minimum_mm"],value_codec::json!(1.0));
        assert_eq!(value["max_sine_squared"],value_codec::json!(0.0001));
        assert_eq!(value["tolerance_uv"],value_codec::json!(0.05));
        assert_eq!(value["pending"][0]["pending_reason"],value_codec::json!("budget"));
        assert_eq!(value["pending"][0]["domains"],value_codec::json!(report.pending[0].domains));
        assert_eq!(value["pending"][0]["faces"],value_codec::json!([2,3]));
        assert!(!value.as_object().unwrap().contains_key("certificate"));
    }
    #[test]
    fn exchange_reuses_only_exact_same_face_products() {
        let a = [[0., 0.5], [0.2, 0.4]];
        let b = [[0.5, 1.], [0.6, 0.8]];
        let cell = |faces, domains| ExpressionWallCell { faces, domains, pending_reason: None };
        assert_eq!(
            product_key(&cell([2, 2], [a, b])),
            product_key(&cell([2, 2], [b, a]))
        );
        assert_ne!(
            product_key(&cell([2, 3], [a, b])),
            product_key(&cell([2, 3], [b, a]))
        );
        let mut other = b;
        other[0][1] = 0.9;
        assert_ne!(
            product_key(&cell([2, 2], [a, b])),
            product_key(&cell([2, 2], [a, other]))
        );
    }
}
#[derive(Default)]
pub struct ExpressionWallSpanStages {
    pub position: usize,
    pub normal: usize,
    pub shared_corner: usize,
    pub original_pair: usize,
    pub retained_pair: usize,
    pub corrected_pair: usize,
}
pub struct ExpressionWallReport {
    pub minimum_mm: f64,
    pub max_sine_squared: f64,
    pub tolerance_uv: f64,
    pub complete: bool,
    pub pending: Vec<ExpressionWallCell>,
    pub cells: usize,
    pub spans: usize,
    pub span_stages: ExpressionWallSpanStages,
    pub distance_excluded: usize,
    pub angle_excluded: usize,
    pub profile_transition_checked: usize,
    pub profile_transition_excluded: usize,
    pub profile_transition_inconclusive: usize,
    pub profile_transition_unresolved: usize,
    pub profile_transition_spans: usize,
    pub planar_excluded: usize,
    pub intrinsic_excluded: usize,
    pub intrinsic_checked: usize,
    pub intrinsic_spans: usize,
    pub intrinsic_unresolved: usize,
    pub plane_controls: usize,
    pub cache_hits: usize,
    pub total_pairs: usize,
    pub initial_pairs_checked: usize,
    pub symmetry_reused: usize,
    pub trim_excluded: usize,
    pub domain_cells: usize,
    /// Pending products left by global cell/span exhaustion.
    pub budget_pending: usize,
    /// Pending products without complete position enclosures.
    pub enclosure_pending: usize,
    /// Pending products at the requested or representable UV resolution.
    pub resolution_pending: usize,
}
/// Diagnostic transport only. The geometry-owned qualification certificate
/// deliberately has no wire decoder or serialization-based admission path.
#[cfg(feature = "codec")]
impl value_codec::Serialize for ExpressionWallReport {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "method":"expression-support-aligned-chord-coverage",
            "scope":"original-retained-transition-support-products",
            "minimum_mm":self.minimum_mm,"max_sine_squared":self.max_sine_squared,
            "tolerance_uv":self.tolerance_uv,
            "complete":self.complete,"pending":self.pending,
            "cells":self.cells,"spans":self.spans,
            "span_stages":{
                "position":self.span_stages.position,"normal":self.span_stages.normal,
                "shared_corner":self.span_stages.shared_corner,"original_pair":self.span_stages.original_pair,
                "retained_pair":self.span_stages.retained_pair,"corrected_pair":self.span_stages.corrected_pair,
            },
            "distance_excluded":self.distance_excluded,"angle_excluded":self.angle_excluded,
            "planar_excluded":self.planar_excluded,"intrinsic_excluded":self.intrinsic_excluded,
            "intrinsic_checked":self.intrinsic_checked,"intrinsic_spans":self.intrinsic_spans,
            "intrinsic_unresolved":self.intrinsic_unresolved,
            "profile_transition_checked":self.profile_transition_checked,
            "profile_transition_excluded":self.profile_transition_excluded,
            "profile_transition_inconclusive":self.profile_transition_inconclusive,
            "profile_transition_unresolved":self.profile_transition_unresolved,
            "profile_transition_spans":self.profile_transition_spans,
            "plane_controls":self.plane_controls,"cache_hits":self.cache_hits,
            "total_pairs":self.total_pairs,"initial_pairs_checked":self.initial_pairs_checked,
            "symmetry_reused":self.symmetry_reused,"trim_excluded":self.trim_excluded,
            "domain_cells":self.domain_cells,"budget_pending":self.budget_pending,
            "enclosure_pending":self.enclosure_pending,"resolution_pending":self.resolution_pending
        })
    }
}
impl EmbeddedExpressionGeometry<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    /// Exclude all geometrical endpoint-aligned chords shorter than minimum.
    /// Whole-support supersets include chords outside material. Complete
    /// exclusion therefore bounds material chords, but never supplies a local
    /// witness or proves that this requested minimum is attained.
    pub fn inspect_minimum_aligned_chords(
        &self,
        minimum_mm: f64,
        max_sine_squared: f64,
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ExpressionWallReport> {
        self.inspect_trimmed_minimum_aligned_chords(
            minimum_mm,
            max_sine_squared,
            tolerance_uv,
            controls,
            max_cells,
            max_spans,
            0,
        )
    }
    /// Additionally exclude only rectangles proven outside owned trim loops.
    pub fn inspect_trimmed_minimum_aligned_chords(
        &self,
        minimum_mm: f64,
        max_sine_squared: f64,
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
        max_domain_cells: usize,
    ) -> Result<ExpressionWallReport> {
        if !minimum_mm.is_finite()
            || minimum_mm <= 0.
            || !max_sine_squared.is_finite()
            || !(0. ..1.).contains(&max_sine_squared)
            || !tolerance_uv.is_finite()
            || tolerance_uv <= 0.
            || tolerance_uv > 1.
            || controls > 1000000
            || max_cells > 100000
            || max_spans > 1000000
            || max_domain_cells > 1000000
        {
            return Err(Error::new(
                "BREP_EXPRESSION_WALL",
                "Choose positive minimum, angular tolerance and bounded coverage work",
            ));
        }
        let shell = self.shell();
        let n = shell.faces().len();
        let total_pairs = n * (n + 1) / 2;
        let domains = (0..n)
            .map(|i| shell.support_domain(i))
            .collect::<Result<Vec<_>>>()?;
        let mut stack = std::collections::VecDeque::with_capacity(total_pairs);
        for a in 0..n {
            for b in a..n {
                stack.push_back(ExpressionWallCell {
                    pending_reason: None,
                    faces: [a, b],
                    domains: [domains[a], domains[b]],
                });
            }
        }
        let mut out = ExpressionWallReport {
            minimum_mm,max_sine_squared,tolerance_uv,
            complete: false,
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            span_stages: ExpressionWallSpanStages::default(),
            distance_excluded: 0,
            angle_excluded: 0,
            profile_transition_checked:0,profile_transition_excluded:0,profile_transition_spans:0,
            profile_transition_inconclusive:0,profile_transition_unresolved:0,
            planar_excluded: 0,
            intrinsic_excluded: 0,
            intrinsic_checked: 0, intrinsic_spans: 0, intrinsic_unresolved: 0,
            plane_controls: 0,
            cache_hits: 0,
            total_pairs,
            initial_pairs_checked: 0,
            symmetry_reused: 0,
            trim_excluded: 0,
            domain_cells: 0,
            budget_pending: 0,
            enclosure_pending: 0,
            resolution_pending: 0,
        };
        let root = shell.assembly.patch.rails.face.boundary.rooted[0];
        let source = root.restricted.proposal.body.geometry().shell();
        // One immutable root recipe owns every transition query in this
        // invocation. Preparation refusal stays None for all its products.
        let transition_bridge=if max_cells>0 && max_spans>0
            && shell.faces.iter().any(|f|matches!(f.kind,ShellFaceKind::Transition)) {
            let profile=root.restricted.proposal.profile(root.face).unwrap();
            let first=profile.section(root.axis,root.parameters[0])?;
            let last=profile.section(root.axis,root.parameters[1])?;
            let width=root.parameters[1]-root.parameters[0];
            Some(nurbs_core::normal_alignment::ProfileSectionBridge::new(first,last,[width;2])?)
        } else {None};
        let corrected_transition=match &transition_bridge {
            Some(bridge)=>bridge.constrain_original_rails(controls)?.0,
            None=>None,
        };
        let mut planes = vec![None; n];
        if max_cells > 0 {
            for (face, entry) in shell.faces.iter().enumerate() {
                let original = match entry.kind {
                    ShellFaceKind::Original(i) => Some(i),
                    ShellFaceKind::Retained(_) => Some(root.face),
                    ShellFaceKind::Transition => None,
                };
                if let Some(original) = original {
                    let surface = source.faces()[original][0].edges()[0].surface();
                    let (plane, used) = crate::material_wall_coverage::plane(surface, controls);
                    out.plane_controls += used;
                    planes[face] = plane;
                }
            }
        }
        let mut profile_chord_workspace=corrected_transition.as_ref().map(|bridge|bridge.profile_chord_workspace());
        type Cache = std::collections::HashMap<(usize, [[u64; 2]; 2]), [[f64; 2]; 3]>;
        let mut original_chord_workspaces=std::collections::HashMap::new();
        let mut corner_chord_workspaces=std::collections::HashMap::new();
        let retained_profile=root.restricted.proposal.profile(root.face).unwrap();
        let mut retained_chord_workspace=retained_profile.chord_workspace();
        let mut intrinsic = std::collections::HashMap::new();
        let mut intrinsic_attempts = std::collections::HashMap::new();
        let mut intrinsic_exclusions: std::collections::HashMap<usize, Vec<[[f64; 2]; 2]>> = std::collections::HashMap::new();
        let mut positions = Cache::new();
        let mut normal_cache = Cache::new();
        let mut products = std::collections::HashSet::new();
        let mut trim_rectangles = std::collections::HashMap::new();
        let mut trim_exclusions: std::collections::HashMap<usize, Vec<[[f64; 2]; 2]>> = std::collections::HashMap::new();
        let key = |face: usize, uv: [[f64; 2]; 2]| (face, uv.map(|d| d.map(f64::to_bits)));
        while let Some(mut cell) = stack.pop_front() {
            let product = product_key(&cell);
            // Endpoint alignment and distance are invariant under exchange on
            // the same immutable face. The representative may remain pending;
            // reusing it never turns uncertainty into an exclusion.
            if products.contains(&product) {
                out.symmetry_reused += 1;
                continue;
            }
            if out.cells == max_cells || out.spans == max_spans {
                out.budget_pending += 1 + stack.len();
                cell.pending_reason=Some(ExpressionWallPendingReason::Budget);
                out.pending.push(cell);
                out.pending.extend(stack.into_iter().map(|mut c| {
                    c.pending_reason=Some(ExpressionWallPendingReason::Budget);c
                }));
                return Ok(out);
            }
            products.insert(product);
            out.cells += 1;
            if cell.domains == [domains[cell.faces[0]], domains[cell.faces[1]]] {
                out.initial_pairs_checked += 1;
            }
            let mut outside = false;
            for side in 0..2 {
                let rectangle_key = key(cell.faces[side], cell.domains[side]);
                // A complete outside-trim proof excludes every contained UV
                // rectangle on this same immutable face. Never reuse an
                // unresolved classification or an inside classification here.
                if trim_exclusions
                    .get(&cell.faces[side])
                    .is_some_and(|regions| {
                        regions
                            .iter()
                            .any(|region| rectangle_contains(*region, cell.domains[side]))
                    })
                {
                    out.cache_hits += 1;
                    outside = true;
                    break;
                }
                if let Some(&excluded) = trim_rectangles.get(&rectangle_key) {
                    out.cache_hits += 1;
                    if excluded {
                        outside = true;
                        break;
                    }
                    continue;
                }
                if out.domain_cells < max_domain_cells {
                    let (excluded, spent) = shell.inspect_support_trim_outside(
                        cell.faces[side],
                        cell.domains[side],
                        tolerance_uv,
                        (max_domain_cells - out.domain_cells).min(100000),
                    )?;
                    out.domain_cells += spent;
                    if let Some(excluded) = excluded {
                        trim_rectangles.insert(rectangle_key, excluded);
                        if excluded {
                            let regions = trim_exclusions.entry(cell.faces[side]).or_default();
                            regions.retain(|region| !rectangle_contains(cell.domains[side], *region));
                            regions.push(cell.domains[side]);
                            outside = true;
                            break;
                        }
                    }
                }
            }
            if outside {
                out.trim_excluded += 1;
                continue;
            }
            if matches!((planes[cell.faces[0]],planes[cell.faces[1]]),(Some(a),Some(b))
                if crate::material_wall_coverage::same_plane(a,b))
            {
                out.planar_excluded += 1;
                continue;
            }
            // Both retained pieces are restrictions of the same immutable
            // selected profile, expressed in its unchanged source coordinates.
            // A self-chord proof therefore also applies across the split pieces.
            let same_retained_profile=matches!(
                (shell.faces[cell.faces[0]].kind,shell.faces[cell.faces[1]].kind),
                (ShellFaceKind::Retained(_),ShellFaceKind::Retained(_)));
            if cell.faces[0] == cell.faces[1] {
                let union = std::array::from_fn(|k| {
                    [
                        cell.domains[0][k][0].min(cell.domains[1][k][0]),
                        cell.domains[0][k][1].max(cell.domains[1][k][1]),
                    ]
                });
                let union_key = key(cell.faces[0], union);
                let budget = (max_spans - out.spans).min(100000);
                let decision = if intrinsic_exclusions
                    .get(&cell.faces[0])
                    .is_some_and(|regions| {
                        regions
                            .iter()
                            .any(|region| rectangle_contains(*region, union))
                    }) {
                    out.cache_hits += 1;
                    Some(true)
                } else if let Some(&decision) = intrinsic.get(&union_key) {
                    out.cache_hits += 1;
                    Some(decision)
                } else if intrinsic_attempts
                    .get(&union_key)
                    .is_some_and(|&previous| budget <= previous)
                {
                    // Exact same immutable query already refused with at least
                    // this much work. Keep it unresolved; no exclusion follows.
                    // A larger budget is allowed to retry the calculation.
                    out.cache_hits += 1;
                    None
                } else {
                    out.intrinsic_checked += 1;
                    let (decision, spent) = match shell.faces[cell.faces[0]].kind {
                        ShellFaceKind::Original(index) => {
                            let surface = source.faces()[index][0].edges()[0].surface();
                            let workspace = match original_chord_workspaces.entry(index) {
                                std::collections::hash_map::Entry::Occupied(entry) => {
                                    entry.into_mut()
                                }
                                std::collections::hash_map::Entry::Vacant(entry) => entry.insert(
                                    nurbs_core::surface_self_chord::OriginalChordWorkspace::new(
                                        surface,
                                        max_sine_squared,
                                    )?,
                                ),
                            };
                            let hits = workspace.cache_hits();
                            let report = workspace.qualify_rectangle(union, budget)?;
                            out.cache_hits += workspace.cache_hits() - hits;
                            (
                                report.complete.then_some(report.certificate.is_some()),
                                report.spans,
                            )
                        }
                        ShellFaceKind::Retained(_) => {
                            let hits = retained_chord_workspace.cache_hits();
                            let result = retained_chord_workspace.inspect_self_chords(
                                union,
                                max_sine_squared,
                                budget,
                            )?;
                            out.cache_hits += retained_chord_workspace.cache_hits() - hits;
                            result
                        }
                        ShellFaceKind::Transition => match &mut profile_chord_workspace {
                            Some(workspace) => {
                                let hits = workspace.cache_hits();
                                let result = workspace.inspect_self_chords(
                                    union,
                                    max_sine_squared,
                                    budget,
                                )?;
                                out.cache_hits += workspace.cache_hits() - hits;
                                #[cfg(test)]
                                {
                                    let direct = corrected_transition
                                        .as_ref()
                                        .unwrap()
                                        .inspect_self_chords(union, max_sine_squared, budget)?;
                                    if direct.0.is_some() {
                                        assert_eq!(
                                            result.0, direct.0,
                                            "Shared intrinsic exclusion must preserve the native decision"
                                        );
                                    }
                                    assert!(
                                        result.1 <= direct.1,
                                        "Sharing must not increase intrinsic span work"
                                    );
                                }
                                result
                            }
                            None => (None, 0),
                        },
                    };
                    out.spans += spent;
                    out.intrinsic_spans += spent;
                    if decision.is_none() {
                        out.intrinsic_unresolved += 1;
                        intrinsic_attempts.insert(union_key, budget);
                    }
                    // Cache only complete calculations. False means this
                    // exclusion was insufficient, never a collision witness.
                    if let Some(decision) = decision {
                        intrinsic.insert(union_key, decision);
                        if decision {
                            let regions = intrinsic_exclusions.entry(cell.faces[0]).or_default();
                            regions.retain(|region| !rectangle_contains(union, *region));
                            regions.push(union);
                        }
                    }
                    decision
                };
                if decision == Some(true) {
                    out.intrinsic_excluded += 1;
                    continue;
                }
            }
            let mut points = [None, None];
            let mut normals = [None, None];
            for side in 0..2 {
                let key = key(cell.faces[side], cell.domains[side]);
                if let Some(point) = positions.get(&key) {
                    points[side] = Some(*point);
                    out.cache_hits += 1;
                    continue;
                }
                let budget = (max_spans - out.spans).min(100000);
                // The immutable corrected recipe is shared with the intrinsic
                // queries; do not rebuild its constrained rails per cache miss.
                let (point, used) = if matches!(shell.faces[cell.faces[side]].kind, ShellFaceKind::Transition) {
                    match &mut profile_chord_workspace {
                        Some(workspace) => {
                            let hits=workspace.cache_hits();
                            let result=workspace.enclose_position(cell.domains[side],budget)?;
                            out.cache_hits+=workspace.cache_hits()-hits;
                            result
                        }
                        None => (None, 0),
                    }
                } else {
                    shell.enclose_support(cell.faces[side], cell.domains[side], controls, budget)?
                };
                #[cfg(test)]
                if matches!(shell.faces[cell.faces[side]].kind, ShellFaceKind::Transition) {
                    let direct=shell.enclose_support(cell.faces[side], cell.domains[side], controls, budget)?;
                    if direct.0.is_some() {
                        assert_eq!(point,direct.0,"Shared transition position must preserve complete interval bounds");
                    }
                    assert!(used<=direct.1,"Sharing must not increase position span work");
                }
                out.spans += used;
                out.span_stages.position += used;
                points[side] = point;
                if let Some(point) = point {
                    positions.insert(key, point);
                }
            }
            let [Some(a), Some(b)] = points else {
                if out.spans==max_spans {
                    out.budget_pending+=1;
                    cell.pending_reason=Some(ExpressionWallPendingReason::Budget);
                } else {
                    out.enclosure_pending += 1;
                    cell.pending_reason=Some(ExpressionWallPendingReason::Enclosure);
                }
                out.pending.push(cell);
                continue;
            };
            if nurbs_core::surface_distance::enclosure_distance(&a, &b)?.0 >= minimum_mm {
                out.distance_excluded += 1;
                continue;
            }
            for side in 0..2 {
                let key = key(cell.faces[side], cell.domains[side]);
                if let Some(normal) = normal_cache.get(&key) {
                    normals[side] = Some(*normal);
                    out.cache_hits += 1;
                    continue;
                }
                let budget = (max_spans - out.spans).min(100000);
                let (normal, used) = if matches!(shell.faces[cell.faces[side]].kind, ShellFaceKind::Transition) {
                    // Keep the public path's owner/domain validation and exact
                    // interval cross product, including exhausted-budget refusal.
                    shell.enclose_support_derivatives(cell.faces[side], cell.domains[side], controls, 0)?;
                    if budget == 0 { (None, 0) } else {
                        match &mut profile_chord_workspace {
                            Some(workspace) => {
                                let hits=workspace.cache_hits();
                                let (jet, spent) = workspace.enclose_derivatives(cell.domains[side],budget)?;
                                out.cache_hits+=workspace.cache_hits()-hits;
                                (jet.map(segment::normal_from_derivatives).transpose()?, spent)
                            }
                            None => (None, 0),
                        }
                    }
                } else {
                    shell.enclose_support_normal_directions(cell.faces[side], cell.domains[side], controls, budget)?
                };
                #[cfg(test)]
                if matches!(shell.faces[cell.faces[side]].kind, ShellFaceKind::Transition) {
                    let direct=shell.enclose_support_normal_directions(cell.faces[side], cell.domains[side], controls, budget)?;
                    if direct.0.is_some() {
                        assert_eq!(normal,direct.0,"Shared transition normals must preserve complete interval bounds");
                    }
                    assert!(used<=direct.1,"Sharing must not increase normal span work");
                }
                out.spans += used;
                out.span_stages.normal += used;
                if let Some(normal) = normal {
                    normals[side] = Some(normal);
                    normal_cache.insert(key, normal);
                }
            }
            if nurbs_core::normal_alignment::endpoint_chord_excluded(
                [a, b],
                normals,
                max_sine_squared,
            )? {
                out.angle_excluded += 1;
                continue;
            }
            if cell.faces[0] != cell.faces[1] && out.spans < max_spans {
                if let (ShellFaceKind::Original(first), ShellFaceKind::Original(last)) = (
                    shell.faces[cell.faces[0]].kind,
                    shell.faces[cell.faces[1]].kind,
                ) {
                    // A midpoint direction is only a proposal. Native proof
                    // requires an exactly shared authored corner and fresh
                    // continuous paths and endpoint normal enclosures.
                    let projection = std::array::from_fn(|k| {
                        b[k][0] * 0.5 + b[k][1] * 0.5 - a[k][0] * 0.5 - a[k][1] * 0.5
                    });
                    if projection.iter().all(|x| x.is_finite()) {
                        let surfaces = [
                            source.faces()[first][0].edges()[0].surface(),
                            source.faces()[last][0].edges()[0].surface(),
                        ];
                        let workspace = match corner_chord_workspaces.entry((first, last)) {
                            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                            std::collections::hash_map::Entry::Vacant(entry) => entry.insert(
                                nurbs_core::surface_self_chord::SharedCornerWorkspace::new(
                                    surfaces,
                                    max_sine_squared,
                                )?,
                            ),
                        };
                        let budget = (max_spans - out.spans).min(64);
                        let hits = workspace.cache_hits();
                        let (excluded, spent) =
                            workspace.inspect(cell.domains, projection, budget)?;
                        out.cache_hits += workspace.cache_hits() - hits;
                        #[cfg(test)]
                        {
                            let direct =
                                nurbs_core::surface_self_chord::inspect_shared_corner_chords(
                                    surfaces,
                                    cell.domains,
                                    projection,
                                    max_sine_squared,
                                    budget,
                                )?;
                            if direct.0 {
                                assert!(
                                    excluded,
                                    "Cached corner path must preserve direct admission"
                                );
                            }
                            assert!(
                                spent <= direct.1,
                                "Cached corner evidence must not increase span work"
                            );
                        }
                        out.spans += spent;
                        out.span_stages.shared_corner += spent;
                        if excluded {
                            out.angle_excluded += 1;
                            continue;
                        }
                    }
                }
            }
            if cell.faces[0] == cell.faces[1]
                && matches!(shell.faces[cell.faces[0]].kind, ShellFaceKind::Original(_))
                && out.spans < max_spans
            {
                let ShellFaceKind::Original(index) = shell.faces[cell.faces[0]].kind else {
                    unreachable!()
                };
                let surface = source.faces()[index][0].edges()[0].surface();
                let workspace = match original_chord_workspaces.entry(index) {
                    std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(nurbs_core::surface_self_chord::OriginalChordWorkspace::new(
                            surface,
                            max_sine_squared,
                        )?)
                    }
                };
                let hits = workspace.cache_hits();
                let (excluded, spent) =
                    workspace.inspect_pair(cell.domains, (max_spans - out.spans).min(100000))?;
                out.cache_hits += workspace.cache_hits() - hits;
                out.spans += spent;
                out.span_stages.original_pair += spent;
                if excluded {
                    out.angle_excluded += 1;
                    continue;
                }
            }
            if (cell.faces[0] == cell.faces[1] || same_retained_profile)
                && matches!(shell.faces[cell.faces[0]].kind, ShellFaceKind::Retained(_))
                && out.spans < max_spans
            {
                let hits = retained_chord_workspace.cache_hits();
                let (excluded, spent) = retained_chord_workspace.inspect_pair(
                    cell.domains,
                    max_sine_squared,
                    (max_spans - out.spans).min(100000),
                )?;
                out.cache_hits += retained_chord_workspace.cache_hits() - hits;
                out.spans += spent;
                out.span_stages.retained_pair += spent;
                if excluded == Some(true) {
                    out.angle_excluded += 1;
                    continue;
                }
            }
            let mut coordinate = 0;
            if cell.faces[0] == cell.faces[1]
                && matches!(shell.faces[cell.faces[0]].kind, ShellFaceKind::Transition)
                && out.spans < max_spans
            {
                if let Some(workspace) = &mut profile_chord_workspace {
                    let hits=workspace.cache_hits();
                    let (excluded, spent) = workspace.inspect_corrected_pair(
                        cell.domains,
                        max_sine_squared,
                        (max_spans - out.spans).min(100000),
                    )?;
                    out.spans += spent;
                    out.span_stages.corrected_pair += spent;
                    out.cache_hits += workspace.cache_hits() - hits;
                    if excluded == Some(true) {
                        out.angle_excluded += 1;
                        continue;
                    }
                }
            }
            if out.spans < max_spans {
                let sides = match (
                    shell.faces[cell.faces[0]].kind,
                    shell.faces[cell.faces[1]].kind,
                ) {
                    (ShellFaceKind::Retained(end), ShellFaceKind::Transition) => Some((0, 1, end)),
                    (ShellFaceKind::Transition, ShellFaceKind::Retained(end)) => Some((1, 0, end)),
                    _ => None,
                };
                if let (Some((profile_side, transition_side, end)), Some(workspace)) =
                    (sides, &mut profile_chord_workspace)
                {
                    out.profile_transition_checked += 1;
                    let hits = workspace.cache_hits();
                    let (excluded, spent) = workspace.inspect(
                        cell.domains[profile_side],
                        cell.domains[transition_side],
                        end == 1,
                        max_sine_squared,
                        (max_spans - out.spans).min(256),
                    )?;
                    out.spans += spent;
                    out.profile_transition_spans += spent;
                    out.cache_hits += workspace.cache_hits() - hits;
                    match excluded {
                        Some(true) => {
                            out.angle_excluded += 1;
                            out.profile_transition_excluded += 1;
                            continue;
                        }
                        Some(false) => out.profile_transition_inconclusive += 1,
                        None => out.profile_transition_unresolved += 1,
                    }
                }
            }
            let mut largest = 0.;
            for k in 0..4 {
                let side = k / 2;
                let axis = k % 2;
                let natural = domains[cell.faces[side]][axis];
                let width = (cell.domains[side][axis][1] - cell.domains[side][axis][0])
                    / (natural[1] - natural[0]);
                if width > largest {
                    largest = width;
                    coordinate = k;
                }
            }
            if largest <= tolerance_uv {
                out.resolution_pending += 1;
                cell.pending_reason=Some(ExpressionWallPendingReason::Resolution);
                out.pending.push(cell);
                continue;
            }
            let side = coordinate / 2;
            let axis = coordinate % 2;
            let range = cell.domains[side][axis];
            let midpoint = range[0] * 0.5 + range[1] * 0.5;
            if midpoint <= range[0] || midpoint >= range[1] {
                out.resolution_pending += 1;
                cell.pending_reason=Some(ExpressionWallPendingReason::Resolution);
                out.pending.push(cell);
                continue;
            }
            let mut left = cell.domains;
            let mut right = cell.domains;
            left[side][axis][1] = midpoint;
            right[side][axis][0] = midpoint;
            stack.push_back(ExpressionWallCell {
                pending_reason: None,
                faces: cell.faces,
                domains: right,
            });
            stack.push_back(ExpressionWallCell {
                pending_reason: None,
                faces: cell.faces,
                domains: left,
            });
        }
        out.complete = out.pending.is_empty();
        Ok(out)
    }
}
