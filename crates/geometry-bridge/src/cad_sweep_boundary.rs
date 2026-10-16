//! Original request -> native law/boundary proof -> model, without TS geometry.
use crate::{Result, Value, field, input};
use brep_core::{LoftCap, sweep_boundary_owned as owned};
use nurbs_core::{
    curve::Curve,
    progressive_sweep::{MultiSweep, Options, Orientation, Spacing, constant_vector_law},
};
use value_codec::json;
fn optional<T: for<'a> value_codec::Deserialize<'a>>(v: &Value, key: &str) -> Result<Option<T>> {
    v.get(key)
        .filter(|x| !x.is_null())
        .map(|_| field(v, key))
        .transpose()
}
fn caps(v: &Value) -> Result<Vec<LoftCap>> {
    let values = v
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or_else(|| input("Exactly two authored caps required"))?;
    values
        .iter()
        .map(|c| {
            Ok(LoftCap {
                surface: field(c, "surface")?,
                trims: field(c, "trims")?,
            })
        })
        .collect()
}
mod closed;
mod smooth_closed;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Open,
    Closed,
    SmoothClosed,
    SmoothClosedAudit,
}
pub fn construct(v: Value) -> Result<Value> {
    construct_impl(v, Kind::Open)
}
pub fn construct_closed(v: Value) -> Result<Value> {
    construct_impl(v, Kind::Closed)
}
pub fn construct_smooth_closed(v: Value) -> Result<Value> {
    construct_impl(v, Kind::SmoothClosed)
}
pub fn verify_smooth_closed(v: Value) -> Result<Value> {
    construct_impl(v, Kind::SmoothClosedAudit)
}
fn construct_impl(v: Value, kind: Kind) -> Result<Value> {
    let closed = kind != Kind::Open;
    let s = field::<Value>(&v, "source")?;
    let profiles: Vec<Curve> = field(&s, "profiles")?;
    let path = field(&s, "path")?;
    let scale = field(&s, "scale")?;
    let twist = field(&s, "twist")?;
    let orientation_name = field::<String>(&s, "orientation")?;
    let orientation = match orientation_name.as_str() {
        "rmf" => Orientation::RotationMinimizing,
        "fixed" | "authored" => Orientation::Fixed,
        "fixed_normal" => Orientation::FixedNormal,
        "frenet" => Orientation::Frenet,
        "corrected_frenet" => Orientation::CorrectedFrenet,
        _ => return Err(input("Unknown original frame mode")),
    };
    let spacing = match field::<String>(&s, "spacing")?.as_str() {
        "parameter" => Spacing::Parameter,
        "arc_length" => Spacing::ArcLength {
            tolerance: field(&s, "length_tolerance")?,
            max_cells: field(&s, "length_max_cells")?,
        },
        _ => return Err(input("Unknown original station policy")),
    };
    let options = Options {
        normal: field(&s, "normal")?,
        orientation,
        spacing,
        initial_sections: field(&s, "initial_sections")?,
        max_sections: field(&s, "max_sections")?,
        max_deviation: field(&s, "max_deviation")?,
    };
    let axis = optional::<Curve>(&s, "frame_axis")?;
    let normal = optional::<Curve>(&s, "frame_normal")?;
    if orientation_name == "authored" && axis.is_none() {
        return Err(input("Authored mode requires original frame laws"));
    }
    if axis.is_some() != normal.is_some() {
        return Err(input("Both original authored frame laws required"));
    }
    let axes = optional::<Curve>(&s, "axis_scale")?;
    let center = optional::<Curve>(&s, "center_law")?;
    let default_axes = constant_vector_law([1.; 3])?;
    let default_center = constant_vector_law([0.; 3])?;
    let guide = optional::<Curve>(&s, "orientation_guide")?;
    let contact = optional::<f64>(&s, "contact_parameter")?;
    if contact.is_none() && s.get("contact_profile").is_some_and(|v| !v.is_null()) {
        return Err(input("Contact profile requires original contact parameter"));
    }
    if s.get("rmf_transport_steps").is_none_or(|v| v.is_null())
        && ["error_max_cells", "error_max_products"]
            .iter()
            .any(|k| s.get(k).is_some_and(|v| !v.is_null()))
    {
        return Err(input(
            "Spatial RMF proof budgets require original transport steps",
        ));
    }

    let mut sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options)?;
    if let (Some(axis), Some(normal)) = (&axis, &normal) {
        sweep = sweep.with_frame_laws(axis, normal)?;
    }
    if axes.is_some() || center.is_some() {
        sweep = sweep.with_affine_laws(
            axes.as_ref().unwrap_or(&default_axes),
            center.as_ref().unwrap_or(&default_center),
        )?;
    }
    if let Some(guide) = &guide {
        sweep = if let Some(parameter) = contact {
            sweep.with_contact_guide(guide, field(&s, "contact_profile")?, parameter)?
        } else {
            sweep.with_orientation_guide(guide)?
        };
    } else if contact.is_some() {
        return Err(input("Original contact anchor requires guide"));
    }
    if let Some(steps) = optional::<usize>(&s, "rmf_transport_steps")? {
        sweep = sweep.with_spatial_rmf_error_limits(
            steps,
            field(&s, "error_max_cells")?,
            field(&s, "error_max_products")?,
        )?;
    }
    let limits = field::<Value>(&v, "limits")?;
    let ideal = optional::<Value>(&v, "idealBody")?;
    let ideal_method = ideal
        .as_ref()
        .map(|i| optional::<String>(i, "method"))
        .transpose()?
        .flatten()
        .unwrap_or_else(|| if closed { "atlas" } else { "monotone" }.to_owned());
    if !matches!(ideal_method.as_str(), "monotone" | "atlas") {
        return Err(input("Unknown original ideal-body proof method"));
    }
    let ideal_limits = ideal
        .as_ref()
        .map(|i| -> Result<owned::IdealLimits> {
            Ok(owned::IdealLimits {
                projection: field(i, "projection")?,
                cells: field(i, "maxCells")?,
                exact_work: field(i, "maxExactWork")?,
            })
        })
        .transpose()?;
    let loop_sizes = field::<Vec<usize>>(&v, "loopSizes")?;
    let sections = field(&v, "sections")?;
    let tolerance_uv = field(&v, "toleranceUv")?;
    let error_budget = field(&v, "maxDeviation")?;
    if matches!(kind, Kind::SmoothClosed | Kind::SmoothClosedAudit) {
        if ideal_method != "atlas" {
            return Err(input(
                "Smooth closed original material requires the quotient atlas",
            ));
        }
        return smooth_closed::construct(
            &sweep,
            &loop_sizes,
            sections,
            tolerance_uv,
            error_budget,
            ideal_limits.ok_or_else(|| input("Smooth closed ideal-body limits required"))?,
            &v,
            kind == Kind::SmoothClosedAudit,
        );
    }
    let limits = owned::Limits {
        source_cells: field(&limits, "sourceCells")?,
        source_products: field(&limits, "sourceProducts")?,
        source_exact_work: field(&limits, "sourceExactWork")?,
        regularity_cells: field(&limits, "regularityCells")?,
        linear_cells: field(&limits, "linearCells")?,
        volume: crate::cad_boundary_certificate::volume_limits(&limits["volume"])?,
    };
    if closed {
        if ideal_method != "atlas" {
            return Err(input(
                "Closed original material requires the quotient atlas",
            ));
        }
        return self::closed::construct(
            &sweep,
            &loop_sizes,
            sections,
            tolerance_uv,
            error_budget,
            limits,
            ideal_limits.ok_or_else(|| input("Closed original ideal-body limits required"))?,
            &v,
        );
    }
    let source_caps = caps(&v["sourceCaps"])?;
    let retained_caps = caps(&v["retainedCaps"])?;
    let source_caps = [&source_caps[0], &source_caps[1]];
    let retained_caps = [&retained_caps[0], &retained_caps[1]];
    let attempt = if let Some(ideal) = ideal_limits {
        let construct = if ideal_method == "atlas" {
            owned::construct_with_ideal_atlas
        } else {
            owned::construct_with_ideal
        };
        construct(
            &sweep,
            &loop_sizes,
            sections,
            source_caps,
            retained_caps,
            tolerance_uv,
            error_budget,
            limits,
            ideal,
        )?
    } else {
        owned::construct(
            &sweep,
            &loop_sizes,
            sections,
            source_caps,
            retained_caps,
            tolerance_uv,
            error_budget,
            limits,
        )?
    };
    let (model, r) = match &attempt {
        owned::Attempt::Certified(body) => (Some(body.model()), body.report()),
        owned::Attempt::Refused(r) => (None, r.as_ref()),
    };
    Ok(
        json!({"model":model,"certified":r.certified,"continuousBound":r.continuous_bound,"withinBudget":r.within_budget,
        "boundaryErrorUpper":r.boundary_error_upper,"filledCapErrorUpper":r.cap_errors,"wall":r.wall,
        "sourceCells":r.source_cells,"sourceProducts":r.source_products,"sourceExactWork":r.source_exact_work,
        "reason":r.reason,"scope":"original-sweep-law-and-authored-filled-caps",
        "idealBodyEmbeddingCertified":r.ideal_volume.as_ref().is_some_and(|p|p.embedding_certified),
        "idealBody":r.ideal_volume.as_ref().map(|p|json!({"proofMethod":ideal_method,"embeddingCertified":p.embedding_certified,"volumeRegularityCertified":p.regularity_certified,"parameterTopologyPreserved":p.parameter_topology_preserved,"cells":p.cells,"exactWork":p.exact_work,"boxes":p.boxes,"monotonicityLower":p.monotonicity_lower,"projection":p.projection,"reason":p.reason})),"smoothnessCertified":false,"seamContinuity":"C0",
        "regularity":r.regularity.as_ref().map(|p|json!({"proven":p.all_faces_regular_modulo_poles,"cells":p.cells,"unresolvedFaces":p.unresolved_faces})),
        "solidGeometryCertified":r.volume.as_ref().is_some_and(|p|p.proven)}),
    )
}
