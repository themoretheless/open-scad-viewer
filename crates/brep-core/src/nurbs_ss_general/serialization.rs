//! Compatibility decoding for the existing general SS report input.
use super::*;
use value_codec::Value;
fn contact(value: &Value) -> Option<ContactClass> {
    Some(match value.as_str()? {
        "transverse" => ContactClass::Transverse,
        "coincident" => ContactClass::Coincident,
        "odd_tangency" => ContactClass::OddTangency,
        "even_tangency" => ContactClass::EvenTangency,
        "boundary" => ContactClass::Boundary,
        "pole_or_singular" => ContactClass::PoleOrSingular,
        "higher_order_contact" => ContactClass::HigherOrderContact,
        "unresolved_conditioning" => ContactClass::UnresolvedConditioning,
        "near_coincidence" => ContactClass::NearCoincidence,
        _ => return None,
    })
}
fn array<T: for<'a> value_codec::Deserialize<'a>>(value: &Value) -> Result<T> {
    value_codec::from_value(value.clone()).map_err(|_| refuse("Invalid general SS geometry"))
}
fn pcurve(value: &Value) -> Result<GeneralPcurve> {
    Ok(match value["kind"].as_str() {
        Some("line") => GeneralPcurve::Line {
            endpoints: [array(&value["start"])?, array(&value["end"])?],
            correspondence: if value["correspondence"] == "exact_affine" {
                TraceCorrespondence::ExactAffine
            } else {
                TraceCorrespondence::IntervalCertified
            },
        },
        Some("rational_trace") => GeneralPcurve::RationalTrace {
            endpoints: [array(&value["start"])?, array(&value["end"])?],
        },
        Some("point") => GeneralPcurve::Point(array(&value["uv"])?),
        _ => GeneralPcurve::Absent,
    })
}
fn trim(value: &Value) -> Result<Option<brep_topology::CoedgeTrim>> {
    if value.is_null() {
        return Ok(None);
    }
    let trim = brep_topology::CoedgeTrim {
        curve_parameter: array(&value["curveParameter"])?,
        pcurve_parameter: array(&value["pcurveParameter"])?,
        periodic_lift: array(&value["periodicLift"])?,
    };
    trim.validate()?;
    Ok(Some(trim))
}
pub fn branch_graph_from_ss_report(
    report: &Value,
    context: &ToleranceContext,
) -> Result<GeneralBranchGraph> {
    if report["version"] != "nurbs-ss/1" || report["kind"] != "surface_surface" {
        return Err(refuse("Expected nurbs-ss/1 surface_surface report"));
    }
    let components = decode_branches(report)?;
    let complete = report["coverage"]["complete"].as_bool().unwrap_or(false);
    let missed = report["coverage"]["missedBranchProof"]
        .as_bool()
        .unwrap_or(false);
    let unresolved_empty = report["unresolved"]
        .as_array()
        .map(|a| a.is_empty())
        .unwrap_or(false);
    Ok(GeneralBranchGraph {
        components,
        context: context.spec_identity(),
        complete: complete && unresolved_empty,
        permits_topology_authorship: complete
            && missed
            && unresolved_empty
            && report["booleanMutationAuthority"] == false
            && report["topologyAuthority"]["granted"] == false,
        boolean_mutation_authority: false,
        missed_branch_proof: missed,
    })
}
pub fn rational_traces_from_ss_report(
    report: &Value,
) -> Result<Vec<crate::uv_arrangement::LiftedUvPrimitive>> {
    Ok(rational_traces_from_branches(&decode_branches(report)?))
}

fn decode_branches(report: &Value) -> Result<Vec<GeneralSsBranch>> {
    let mut components = Vec::new();
    for (id, component) in report["components"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let kind = match component["kind"].as_str().unwrap_or("unknown") {
            "empty" => continue,
            "curve" => GeneralBranchKind::Curve,
            "overlap" => GeneralBranchKind::Overlap,
            v => GeneralBranchKind::Unknown(v.into()),
        };
        let uv_samples = std::array::from_fn(|support| {
            component["samples"].as_array().map(|samples| {
                samples
                    .iter()
                    .filter_map(|sample| {
                        let uv =
                            sample[if support == 0 { "uvFirst" } else { "uvSecond" }].as_array()?;
                        Some([uv.first()?.as_f64()?, uv.get(1)?.as_f64()?])
                    })
                    .collect()
            })
        });
        let material_sides = component["materialSides"]
            .as_array()
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
            closed: component["closed"].as_bool().unwrap_or(false),
            contact_class: contact(&component["contactClass"]),
            junction: component["junction"].as_bool().unwrap_or(false),
            material_sides,
            coedge_trim: trim(&component["coedgeTrim"])?,
            pcurve_first: pcurve(&component["pcurveFirst"])?,
            pcurve_second: pcurve(&component["pcurveSecond"])?,
            uv_samples,
        });
    }
    Ok(components)
}
