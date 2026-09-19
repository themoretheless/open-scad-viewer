//! General certified NURBS surface/surface BranchGraph adapter.
//!
//! Consumes `nurbs-ss/1` reports from `nurbs_core::ss_intersection` and builds
//! topology-usable BranchGraph-shaped evidence without the graph-patch iso
//! fixture. Boolean mutation authority stays revoked until the next layer.
use crate::coverage_verifier::CoverageAudit;
use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::{Error, Result};
use value_codec::Value;

fn refuse(message: &str) -> Error {
    Error::new("BREP_NURBS_SS_GENERAL_REFUSED", message)
}

pub const GENERAL_SS_CAPABILITY: &str = "nurbs-ss/1";

#[derive(Clone, Debug)]
pub struct GeneralSsBranch {
    pub id: usize,
    pub kind: String,
    pub closed: bool,
    pub contact_class: String,
    pub junction: bool,
    pub material_sides: [i8; 2],
    pub coedge_trim: Option<Value>,
    pub pcurve_first: Value,
    pub pcurve_second: Value,
}

#[derive(Clone, Debug)]
pub struct GeneralBranchGraph {
    pub components: Vec<GeneralSsBranch>,
    pub context: ToleranceSpecIdentity,
    pub complete: bool,
    pub permits_topology_authorship: bool,
    pub boolean_mutation_authority: bool,
    pub missed_branch_proof: bool,
}

impl GeneralBranchGraph {
    pub fn permits_topology_authorship(&self) -> bool {
        self.permits_topology_authorship
            && self.complete
            && self.missed_branch_proof
            && !self.boolean_mutation_authority
            && self.components.iter().all(|c| {
                matches!(
                    c.contact_class.as_str(),
                    "transverse" | "coincident" | "odd_tangency" | "even_tangency" | "boundary"
                )
            })
    }
}

/// Build a general BranchGraph from a certified nurbs-ss/1 report value.
pub fn branch_graph_from_ss_report(
    report: &Value,
    context: &ToleranceContext,
) -> Result<GeneralBranchGraph> {
    if report["version"] != "nurbs-ss/1" || report["kind"] != "surface_surface" {
        return Err(refuse("Expected nurbs-ss/1 surface_surface report"));
    }
    let complete = report["coverage"]["complete"].as_bool().unwrap_or(false);
    let missed = report["coverage"]["missedBranchProof"]
        .as_bool()
        .unwrap_or(false);
    let mut components = Vec::new();
    for (id, component) in report["components"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
    {
        let kind = component["kind"].as_str().unwrap_or("unknown").to_string();
        if kind == "empty" {
            continue;
        }
        let sides = component
            .get("materialSides")
            .and_then(Value::as_array)
            .map(|a| {
                [
                    a.first().and_then(Value::as_i64).unwrap_or(1) as i8,
                    a.get(1).and_then(Value::as_i64).unwrap_or(-1) as i8,
                ]
            })
            .unwrap_or([1, -1]);
        components.push(GeneralSsBranch {
            id,
            kind,
            closed: component
                .get("closed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            contact_class: component
                .get("contactClass")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            junction: component
                .get("junction")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            material_sides: sides,
            coedge_trim: component.get("coedgeTrim").cloned(),
            pcurve_first: component.get("pcurveFirst").cloned().unwrap_or(Value::Null),
            pcurve_second: component
                .get("pcurveSecond")
                .cloned()
                .unwrap_or(Value::Null),
        });
    }
    let unresolved_empty = report["unresolved"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(false);
    let permits = complete
        && missed
        && unresolved_empty
        && report["booleanMutationAuthority"] == false
        && report["topologyAuthority"]["granted"] == false;
    Ok(GeneralBranchGraph {
        components,
        context: context.spec_identity(),
        complete: complete && unresolved_empty,
        permits_topology_authorship: permits,
        boolean_mutation_authority: false,
        missed_branch_proof: missed,
    })
}

/// Independent coverage audit for general SS BranchGraph consumers.
pub fn verify_general_ss_branch_graph(
    graph: &GeneralBranchGraph,
    context: &ToleranceContext,
) -> Result<CoverageAudit> {
    if graph.context != context.spec_identity() {
        return Err(refuse("General SS BranchGraph ToleranceContext mismatch"));
    }
    if graph.boolean_mutation_authority {
        return Err(refuse(
            "General SS BranchGraph must not carry Boolean mutation authority",
        ));
    }
    if graph.complete && !graph.missed_branch_proof {
        return Err(refuse(
            "Complete general SS BranchGraph requires missed-branch proof",
        ));
    }
    if !graph.complete && graph.permits_topology_authorship {
        return Err(refuse(
            "Incomplete general SS BranchGraph cannot permit topology authorship",
        ));
    }
    Ok(CoverageAudit {
        complete: graph.complete,
        component_count: graph.components.len(),
        unresolved_count: if graph.complete { 0 } else { 1 },
        notes: vec![
            "general_ss_branch_graph_verified",
            "no_graph_patch_iso_fixture",
            "boolean_mutation_deferred",
        ],
    })
}

/// Arrange rational curved traces from an SS report into LiftedUv primitives.
pub fn rational_traces_from_ss_report(
    report: &Value,
) -> Result<Vec<crate::uv_arrangement::LiftedUvPrimitive>> {
    use crate::uv_arrangement::{LiftedUvGeometry, LiftedUvPrimitive};
    let mut out = Vec::new();
    for (id, component) in report["components"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
    {
        for (support, key) in [(0, "pcurveFirst"), (1, "pcurveSecond")] {
            let pcurve = component.get(key).cloned().unwrap_or(Value::Null);
            let samples = if let Some(arr) = component.get("samples").and_then(Value::as_array) {
                arr.iter()
                    .filter_map(|sample| {
                        let uv = sample
                            .get(if support == 0 { "uvFirst" } else { "uvSecond" })?
                            .as_array()?;
                        Some([uv.first()?.as_f64()?, uv.get(1)?.as_f64()?])
                    })
                    .collect::<Vec<_>>()
            } else if pcurve["kind"] == "line" {
                let start = pcurve.get("start").and_then(Value::as_array);
                let end = pcurve.get("end").and_then(Value::as_array);
                match (start, end) {
                    (Some(s), Some(e)) => vec![
                        [s[0].as_f64().unwrap_or(0.), s[1].as_f64().unwrap_or(0.)],
                        [e[0].as_f64().unwrap_or(0.), e[1].as_f64().unwrap_or(0.)],
                    ],
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };
            if samples.len() < 2 {
                continue;
            }
            out.push(LiftedUvPrimitive {
                edge_id: id * 2 + support,
                geometry: LiftedUvGeometry::RationalCurvedTrace {
                    samples,
                    closed: component
                        .get("closed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    correspondence: if pcurve["correspondence"] == "exact_affine" {
                        "exact_affine"
                    } else {
                        "interval_certified"
                    },
                    overlap: component["kind"] == "overlap",
                    singular: component["contactClass"] == "pole_or_singular",
                },
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::surface::Surface;

    fn plane(z: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., z], vec![0., 1., z]],
                vec![vec![1., 0., z], vec![1., 1., z]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn general_ss_branch_graph_without_graph_patch() {
        let xy = plane(0.);
        let xz = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 1.]],
                vec![vec![1., 0., 0.], vec![1., 0., 1.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let report =
            nurbs_core::ss_intersection::intersect_surface_surface(&xy, &xz, None).unwrap();
        let context = ToleranceContext::default_valid();
        let graph = branch_graph_from_ss_report(&report, &context).unwrap();
        assert!(graph.complete);
        assert!(!graph.boolean_mutation_authority);
        // Query-grade topology flag may be true only when complete; Boolean stays false.
        let audit = verify_general_ss_branch_graph(&graph, &context).unwrap();
        assert!(audit.notes.contains(&"no_graph_patch_iso_fixture"));
        let traces = rational_traces_from_ss_report(&report).unwrap();
        assert!(!traces.is_empty());
    }
}
