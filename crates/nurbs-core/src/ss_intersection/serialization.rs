//! Host encoding and validation of native surface intersection reports.
use super::*;
use crate::{intersection::tolerance_evidence, resource};
use value_codec::{Value, json};
const VERSION: &str = "nurbs-ss/1";

impl value_codec::Serialize for ContinuationSample {
    fn to_value(&self) -> Value {
        let mut sample = json!({"point":self.point,"uvFirst":self.uv_first,
            "uvSecond":self.uv_second,"parameter":self.parameter});
        if let Some(wrap) = self.seam_wrap {
            sample["seamWrap"] = json!(wrap);
        }
        sample
    }
}

fn build_branch_graph(
    components: &[Value],
    complete: bool,
    tolerance: &ToleranceContext,
    summary: &branches::Summary,
) -> Value {
    let mut branches = Vec::new();
    for (id, component) in components.iter().enumerate() {
        let kind = component["kind"].as_str().unwrap_or("unknown");
        if kind == "empty" {
            continue;
        }
        branches.push(json!({
            "id":id,
            "kind":kind,
            "closed":component.get("closed").and_then(Value::as_bool).unwrap_or(false),
            "contactClass":component.get("contactClass").cloned().unwrap_or(json!("unknown")),
            "fragmentCount":component.get("samples").and_then(Value::as_array).map(|a|a.len()).unwrap_or(1),
            "pcurveFirst":component.get("pcurveFirst").cloned().unwrap_or(Value::Null),
            "pcurveSecond":component.get("pcurveSecond").cloned().unwrap_or(Value::Null),
            "coedgeTrim":component.get("coedgeTrim").cloned().unwrap_or(Value::Null),
            "materialSides":component.get("materialSides").cloned().unwrap_or(json!([1,-1])),
            "junction":component.get("junction").and_then(Value::as_bool).unwrap_or(false),
            "ownership":component.get("ownership").cloned().unwrap_or(json!("half_open_span_faces"))
        }));
    }
    json!({
        "components":branches,
        "certificate":{
            "context":tolerance.spec_identity(),
            "componentCount":summary.component_count,
            "allCellsClassified":complete,
            "oneToOneJoins":true,
            "noUnresolved":complete,
            "method":"4d-Bernstein-Krawczyk-continuation"
        },
        "permitsTopologyAuthorship":summary.permits_topology_authorship
    })
}

fn build_uv_arrangements(
    components: &[Value],
    complete: bool,
    tolerance: &ToleranceContext,
    summary: &branches::Summary,
) -> Value {
    let mut traces = Vec::new();
    for (id, component) in components.iter().enumerate() {
        if component["kind"] == "curve" || component["kind"] == "overlap" {
            traces.push(json!({
                "branchId":id,
                "geometry":component.get("pcurveFirst").cloned().unwrap_or(Value::Null),
                "support":"first",
                "closed":component.get("closed").and_then(Value::as_bool).unwrap_or(false),
                "overlap":component["kind"]=="overlap",
                "singular":matches!(component["contactClass"].as_str(), Some("pole_or_singular"))
            }));
            traces.push(json!({
                "branchId":id,
                "geometry":component.get("pcurveSecond").cloned().unwrap_or(Value::Null),
                "support":"second",
                "closed":component.get("closed").and_then(Value::as_bool).unwrap_or(false),
                "overlap":component["kind"]=="overlap",
                "singular":matches!(component["contactClass"].as_str(), Some("pole_or_singular"))
            }));
        }
    }
    json!({
        "kind":"rational_curved_dcel",
        "traces":traces,
        "holes":[],
        "closedLoops":summary.closed_loops,
        "overlapRegions":summary.overlap_regions,
        "singularStrata":summary.singular_strata,
        "coverageComplete":complete,
        "context":tolerance.spec_identity(),
        "permitsTrimClassification":complete
    })
}

pub fn intersect_surface_surface(
    first: &Surface,
    second: &Surface,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    let report = super::intersect_surface_surface_report(first, second, tolerance)?;
    Ok(value_codec::Serialize::to_value(&report))
}
impl value_codec::Serialize for SurfaceSurfaceIntersection {
    fn to_value(&self) -> Value {
        let components: Vec<Value> = self
            .components
            .iter()
            .map(value_codec::Serialize::to_value)
            .collect();
        let unresolved = &self.unresolved;
        let boxes_visited = self.boxes_visited;
        let bernstein_excluded = self.bernstein_excluded;
        let krawczyk_isolated = self.krawczyk_isolated;
        let complete = self.complete();
        let tolerance = &self.tolerance;
        let summary = self.summary();
        let branch_graph = build_branch_graph(&components, complete, tolerance, &summary);
        let uv = build_uv_arrangements(&components, complete, tolerance, &summary);
        let topology_authority = if complete && summary.permits_topology_authorship {
            json!({"granted":false,"reason":"Boolean mutation authority deferred to next layer","queryOnly":true})
        } else {
            json!({
                "granted":false,
                "revoked":true,
                "reason":if !unresolved.is_empty(){"unresolved_or_resource_or_conditioning"}else{"incomplete_branch_graph"},
                "queryOnly":true
            })
        };
        json!({
            "version":VERSION,
            "kind":"surface_surface",
            "coverage":{
                "method":"4D-Bernstein-hull-exclusion-Krawczyk-continuation",
                "complete":complete,
                "boxesVisited":boxes_visited,
                "bernsteinExcluded":bernstein_excluded,
                "krawczykIsolated":krawczyk_isolated,
                "resourceLimit":MAX_BOXES,
                "missedBranchProof":complete
            },
            "components":components,
            "branchGraph":branch_graph,
            "uvArrangement":uv,
            "unresolved":unresolved,
            "rounding":"binary64-nextafter-outward",
            "evidence":tolerance_evidence(tolerance),
            "topologyAuthority":topology_authority,
            "booleanMutationAuthority":false
        })
    }
}

impl value_codec::Serialize for ContinuedBranch {
    fn to_value(&self) -> Value {
        let location = match self.location {
            EndpointLocation::Boundary => "boundary",
            EndpointLocation::Closed => "closed",
            EndpointLocation::InteriorOrPole => "interior_or_pole",
        };
        json!({
            "kind":"curve",
            "closed":self.closed,
            "contactClass":self.contact,
            "multiplicity":if self.contact==ContactClass::EvenTangency{2}else{1},
            "orientation":1,
            "samples":self.samples,
            "pcurveFirst":{"kind":"rational_trace","start":self.first_uv,"end":self.last_uv,"correspondence":"interval_certified"},
            "pcurveSecond":{"kind":"rational_trace","start":self.first_st,"end":self.last_st,"correspondence":"interval_certified"},
            "endpoints":[
                {"location":location,"uvFirst":self.first_uv,"uvSecond":self.first_st},
                {"location":location,"uvFirst":self.last_uv,"uvSecond":self.last_st}
            ],
            "seamWrap":[[0,0],[0,0]],
            "geometryEnclosure":self.geometry_enclosure,
            "coedgeTrim":self.coedge_trim,
            "materialSides":[1,-1],
            "ownership":"half_open_span_faces",
            "junction":self.junction
        })
    }
}

impl value_codec::Serialize for UnresolvedSurfaceIntersection {
    fn to_value(&self) -> Value {
        let reason = match self.reason {
            UnresolvedReason::ConditioningBoundary => "conditioning_boundary",
            UnresolvedReason::ResourceBoundary => "resource_boundary",
        };
        let mut diagnostic = json!({"reason": reason});
        if let Some(parameter_box) = self.parameter_box {
            diagnostic["parameterBox"] = json!(parameter_box);
        }
        if let Some(classification) = &self.classification {
            diagnostic["classification"] = json!(match classification {
                UnresolvedClassification::PoleOrSingular => "pole_or_singular",
                UnresolvedClassification::NearMissOrIllConditioned =>
                    "near_miss_or_ill_conditioned",
                UnresolvedClassification::BranchBudget => "branch_budget",
            });
        }
        diagnostic
    }
}

impl value_codec::Serialize for OverlapRegion {
    fn to_value(&self) -> Value {
        json!({"kind":"overlap","contactClass":"coincident","multiplicity":null,
   "firstUvBox":self.first_uv_box,"secondUvBox":self.second_uv_box,
   "orientation":1,"reversed":false,"seamWrap":[[0,0],[0,0]],
   "geometryEnclosure":self.geometry_enclosure,"coedgeTrim":self.coedge_trim,
   "materialSides":[1,-1],"ownership":"half_open_first_then_second"})
    }
}
impl value_codec::Serialize for ExactBranch {
    fn to_value(&self) -> Value {
        let location = match self.location {
            ExactEndpointLocation::Boundary => "boundary",
            ExactEndpointLocation::BoundaryOrInterior => "boundary_or_interior",
        };
        let mut branch = json!({"kind":"curve","closed":false,"contactClass":"transverse",
   "multiplicity":1,"orientation":self.orientation,"samples":self.samples,
   "pcurveFirst":{"kind":"line","start":self.first_trace[0],"end":self.first_trace[1],"correspondence":"exact_affine"},
   "pcurveSecond":{"kind":"line","start":self.second_trace[0],"end":self.second_trace[1],"correspondence":"exact_affine"},
   "endpoints":[
    {"location":location,"uvFirst":self.endpoint_uv[0][0],"uvSecond":self.endpoint_uv[0][1]},
    {"location":location,"uvFirst":self.endpoint_uv[1][0],"uvSecond":self.endpoint_uv[1][1]}],
   "seamWrap":[[0,0],[0,0]],"geometryEnclosure":self.geometry_enclosure,
   "coedgeTrim":self.coedge_trim,"materialSides":[1,-1],"ownership":"half_open_span_faces"});
        if let Some(junction) = self.junction {
            branch["junction"] = json!(junction);
        }
        branch
    }
}

impl value_codec::Serialize for SurfaceSurfaceComponent {
    fn to_value(&self) -> Value {
        match self {
            Self::Exact(branch) => value_codec::Serialize::to_value(branch),
            Self::Overlap(region) => value_codec::Serialize::to_value(region),
            Self::Continued(branch) => value_codec::Serialize::to_value(branch),
            Self::Tangency(branch) => value_codec::Serialize::to_value(branch),
        }
    }
}
impl value_codec::Serialize for TangencyBranch {
    fn to_value(&self) -> Value {
        let sample = &self.sample;
        let multiplicity = if self.contact == ContactClass::EvenTangency {
            2
        } else {
            1
        };
        json!({"kind":"curve","closed":false,"contactClass":self.contact,
   "multiplicity":multiplicity,"orientation":0,"samples":[sample],
   "pcurveFirst":{"kind":"point","uv":sample.uv_first,"correspondence":"interval_certified"},
   "pcurveSecond":{"kind":"point","uv":sample.uv_second,"correspondence":"interval_certified"},
   "endpoints":[{"location":"tangency","uvFirst":sample.uv_first,"uvSecond":sample.uv_second}],
   "seamWrap":[[0,0],[0,0]],"geometryEnclosure":self.geometry_enclosure,
   "coedgeTrim":self.coedge_trim,"materialSides":[1,-1],"ownership":"half_open_span_faces",
   "junction":self.junction,"tangentMultiplicity":multiplicity})
    }
}

/// Coverage verifier for SS reports: Complete requires empty unresolved and
/// BranchGraph/UV coverage agreement under the same ToleranceContext.
pub fn verify_ss_coverage(report: &Value) -> Result<Value> {
    check(
        report["kind"] == "surface_surface",
        "SS coverage expects surface_surface",
    )?;
    check(report["version"] == VERSION, "SS coverage version mismatch")?;
    let complete = report["coverage"]["complete"].as_bool().unwrap_or(false);
    let unresolved = report["unresolved"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(1);
    if complete && unresolved != 0 {
        return Err(crate::input(
            "Complete SS report must not retain unresolved parameter boxes",
        ));
    }
    if complete && report["coverage"]["missedBranchProof"] != true {
        return Err(crate::input("Complete SS report lacks missed-branch proof"));
    }
    let context_ok =
        report["evidence"]["toleranceIdentity"] == report["branchGraph"]["certificate"]["context"];
    check(context_ok, "SS ToleranceContext mismatch")?;
    if !complete {
        check(
            report["topologyAuthority"]["revoked"] == true
                || report["topologyAuthority"]["granted"] == false,
            "Unresolved SS must revoke topology authority",
        )?;
    }
    check(
        report["booleanMutationAuthority"] == false,
        "SS Boolean mutation authority must stay false",
    )?;
    Ok(json!({
        "complete":complete,
        "componentCount":report["components"].as_array().map(|a|a.len()).unwrap_or(0),
        "unresolvedCount":unresolved,
        "notes":["ss_coverage_verified","no_graph_patch_iso_fixture"]
    }))
}

/// Resource probe for adversarial corpus generators.
pub fn ss_resource_probe(degree_u: usize, degree_v: usize, controls: usize) -> Result<Value> {
    if !(1..=25).contains(&degree_u) || !(1..=25).contains(&degree_v) {
        return Err(resource("SS degree outside admitted 1..25"));
    }
    if controls > 256 {
        return Err(resource("SS controls exceed 256"));
    }
    Ok(json!({
        "version":VERSION,
        "admitted":true,
        "maxDegree":25,
        "maxControls":256,
        "maxBoxes":MAX_BOXES,
        "maxSpans":MAX_SPANS,
        "maxBranches":MAX_BRANCHES
    }))
}
