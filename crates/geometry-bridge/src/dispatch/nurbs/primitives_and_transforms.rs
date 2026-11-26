//! Primitive solids, extrusions, revolutions, faceted constructors, and transforms.
use crate::*;

#[path="radial_cam.rs"]
mod radial_cam;
#[path="worm_gear.rs"]
mod worm_gear;
#[path="finite_thread.rs"]
mod finite_thread;
#[path="rack_bevel.rs"]
mod rack_bevel;
#[path="spur_inspection.rs"]
mod spur_inspection;
#[path="certified_gear_edits.rs"]
mod certified_gear_edits;
#[path="bore_walls.rs"]
mod bore_walls;

pub(super) const OPS: &[&str] = &[
    "piecewise_axis_radius","bore_wall_faces","cap_edge_pcurve","transfer_cap_loop","replace_bore_wall","cut_bore_cylinder",
    "brep_nurbs_sketch_extrude",
    "brep_nurbs_affine_lattice",
    "brep_nurbs_transform",
    "brep_nurbs_workplane",
    "brep_nurbs_box",
    "brep_nurbs_freeform_cuboid",
    "brep_nurbs_freeform_cuboid_bump",
    "brep_nurbs_revolve",
    "brep_nurbs_sphere",
    "brep_nurbs_sphere_exact",
    "brep_fibered_source_construct",
    "brep_fibered_source_audit",
    "brep_implicit_sphere_source_construct",
    "brep_implicit_sphere_source_audit",
    "brep_implicit_affine_sphere_source_audit",
    "radial_cam_profile",
    "rack_profile",
    "brep_rack",
    "finite_za_worm",
    "za_wheel_start", "za_wheel_step", "za_wheel_cancel",
    "finite_za_wheel",
    "finite_external_thread",
    "finite_internal_thread",
    "verified_bevel_gear",
    "brep_nurbs_gear",
    "certified_profile_edit","verified_gear_edit","edited_gear_step","planet_gear_occurrences","planet_occurrences_step","planetary_gear_set_step",
    "eccentric_bore_stock","ring_bolt_stock","ring_bolt_window","external_spur_ratio","internal_spur_ratio","mounted_spur_ratio","spur_tip_clearance","spur_tooth_chord","spur_tooth_thinning","external_spur_backlash","internal_spur_backlash","spur_backlash_range","spur_overlap_range","spur_mounting_window","planet_tip_separation","spinner_layout_counts",
    "brep_nurbs_gear_span",
    "brep_nurbs_verified_gear",
    "brep_nurbs_torus",
    "brep_nurbs_cylinder",
    "brep_nurbs_frustum",
    "brep_nurbs_tube",
    "brep_nurbs_extrude_curves",
    "brep_nurbs_extrude_polygon",
    "brep_nurbs_faceted_loft",
    "brep_nurbs_faceted_sweep",
    "brep_nurbs_faceted_revolve",
    "brep_nurbs_faceted_cylinder",
    "brep_nurbs_faceted_sphere",
    "brep_nurbs_canonical_graph_solid_v3",
    "brep_nurbs_canonical_rational_graph_solid_v5",
    "brep_nurbs_canonical_multispan_graph_solid",
];

pub(super) fn handle(op: &str, v: Value) -> Result<Value> {
    if bore_walls::OPS.contains(&op){return bore_walls::apply(&v,op);}
    match op {
        "brep_nurbs_sketch_extrude" => handle_sketch_extrude(&v),
        "brep_nurbs_affine_lattice" => {
            let report = brep_core::affine_lattice::place(
                &field(&v, "model")?,
                field(&v, "matrix")?,
                field(&v, "quantum")?,
                field(&v, "maxWork")?,
            )?;
            Ok(json!({
                "model": report.model,
                "operatorNormUpper": report.operator_norm_upper,
                "arithmeticErrorUpper": report.arithmetic_error_upper,
                "work": report.work,
                "reason": report.reason
            }))
        }
        "brep_nurbs_transform" => encode(brep_core::transform::affine(
            &field(&v, "model")?,
            field(&v, "matrix")?,
        )?),
        "brep_nurbs_workplane" => encode(brep_core::transform::workplane(
            &field(&v, "model")?,
            field(&v, "origin")?,
            field(&v, "u")?,
            field(&v, "v")?,
            field(&v, "offset")?,
        )?),
        "brep_nurbs_box" => encode(brep_core::cuboid(field(&v, "min")?, field(&v, "max")?)?),
        "brep_nurbs_freeform_cuboid" => encode(cad_step::freeform_cuboid_solid(
            field(&v, "min")?,
            field(&v, "max")?,
        )?),
        "brep_nurbs_freeform_cuboid_bump" => encode(cad_step::freeform_cuboid_with_bump_face(
            field(&v, "min")?,
            field(&v, "max")?,
        )?),
        "brep_nurbs_revolve" => encode(brep_core::revolve_angle(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            v.get("angleDegrees")
                .and_then(Value::as_f64)
                .unwrap_or(360.),
        )?),
        "brep_nurbs_sphere" | "brep_nurbs_sphere_exact" => cad_face_contacts::sphere(v),
        "brep_fibered_source_construct" => cad_fibered_body::run(v, false),
        "brep_fibered_source_audit" => cad_fibered_body::run(v, true),
        "brep_implicit_sphere_source_construct" => cad_implicit_body::sphere(v, false),
        "brep_implicit_sphere_source_audit" => cad_implicit_body::sphere(v, true),
        "brep_implicit_affine_sphere_source_audit" => cad_implicit_body::affine_sphere(v),
        "radial_cam_profile" => radial_cam::profile(&v),
        "rack_profile" => rack_bevel::rack(&v,false),
        "brep_rack" => rack_bevel::rack(&v,true),
        "finite_za_worm" => worm_gear::worm(&v),
        "za_wheel_start" => worm_gear::wheel_start(&v),
        "za_wheel_step" => worm_gear::wheel_step(&v),
        "za_wheel_cancel" => worm_gear::wheel_cancel(&v),
        "finite_za_wheel" => worm_gear::wheel(&v),
        "finite_external_thread" => finite_thread::build(&v, false),
        "finite_internal_thread" => finite_thread::build(&v, true),
        "verified_bevel_gear" => rack_bevel::bevel(&v),
        "eccentric_bore_stock" | "ring_bolt_stock" | "ring_bolt_window" | "external_spur_ratio" | "internal_spur_ratio" | "mounted_spur_ratio" | "spur_tip_clearance" | "spur_tooth_chord" | "spur_tooth_thinning" | "external_spur_backlash" | "internal_spur_backlash" | "spur_backlash_range" | "spur_overlap_range" | "spur_mounting_window" | "planet_tip_separation" | "spinner_layout_counts" => spur_inspection::measure(&v),
        "certified_profile_edit" => certified_gear_edits::profile(&v),
        "verified_gear_edit" | "edited_gear_step" | "planet_gear_occurrences" | "planet_occurrences_step" | "planetary_gear_set_step" | "brep_nurbs_gear" | "brep_nurbs_gear_span" | "brep_nurbs_verified_gear" => handle_gear(&v),
        "brep_nurbs_torus" => encode(brep_core::torus(
            field(&v, "majorRadius")?,
            field(&v, "minorRadius")?,
        )?),
        "brep_nurbs_cylinder" => encode(brep_core::cylinder(
            field(&v, "radius")?,
            field(&v, "height")?,
        )?),
        "brep_nurbs_frustum" => encode(brep_core::frustum(
            field(&v, "bottomRadius")?,
            field(&v, "topRadius")?,
            field(&v, "height")?,
        )?),
        "brep_nurbs_tube" => encode(brep_core::tube(
            field(&v, "outerRadius")?,
            field(&v, "innerRadius")?,
            field(&v, "height")?,
        )?),
        "brep_nurbs_extrude_curves" => encode(brep_core::prism::extrude(
            &field::<Vec<Vec<Curve>>>(&v, "loops")?,
            field(&v, "zMin")?,
            field(&v, "zMax")?,
        )?),
        "brep_nurbs_extrude_polygon" => {
            let holes = v
                .get("holes")
                .map(|_| field::<Vec<Vec<[f64; 2]>>>(&v, "holes"))
                .transpose()?
                .unwrap_or_default();
            encode(brep_core::extrude_polygon_with_holes(
                &field::<Vec<[f64; 2]>>(&v, "profile")?,
                &holes,
                field(&v, "zMin")?,
                field(&v, "zMax")?,
            )?)
        }
        "brep_nurbs_faceted_loft" => encode(brep_core::faceted_loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
        )?),
        "brep_nurbs_faceted_sweep" => encode(brep_core::faceted_sweep(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            &field::<Vec<[f64; 3]>>(&v, "path")?,
            field(&v, "up")?,
        )?),
        "brep_nurbs_faceted_revolve" => encode(brep_core::faceted_revolve(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_faceted_cylinder" => encode(brep_core::faceted_cylinder(
            field(&v, "radius")?,
            field(&v, "height")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_faceted_sphere" => encode(brep_core::faceted_sphere(
            field(&v, "radius")?,
            field(&v, "radialSegments")?,
            field(&v, "latitudeSegments")?,
        )?),
        "brep_nurbs_canonical_graph_solid_v3" => encode(brep_core::canonical_bezier_graph_solid(
            field(&v, "degreeU")?,
            field(&v, "degreeV")?,
        )?),
        "brep_nurbs_canonical_rational_graph_solid_v5" => {
            encode(brep_core::canonical_rational_graph_solid(
                field(&v, "degreeU")?,
                field(&v, "degreeV")?,
            )?)
        }
        "brep_nurbs_canonical_multispan_graph_solid" => {
            require_exact_fields(&v, &["op", "spansU", "spansV"], "multispan graph request")?;
            encode(brep_core::canonical_multispan_graph_solid(
                field(&v, "spansU")?,
                field(&v, "spansV")?,
            )?)
        }
        _ => unreachable!("primitives_and_transforms admits only its OPS"),
    }
}

fn handle_sketch_extrude(v: &Value) -> Result<Value> {
    let sketch = v.get("sketch").ok_or_else(|| input("Missing sketch"))?;
    let profile = if let Some(path) = sketch.get("editablePath").filter(|p| !p.is_null()) {
        brep_core::sketch::Profile::Bezier(path2d::authored_path(path)?)
    } else {
        match sketch.get("analytic") {
            Some(analytic) if analytic.get("kind").and_then(Value::as_str) == Some("circle") => {
                brep_core::sketch::Profile::Circle {
                    center: field(analytic, "center")?,
                    radius: field(analytic, "radius")?,
                }
            }
            _ => brep_core::sketch::Profile::Polygon(field(sketch, "points")?),
        }
    };
    let plane = match sketch.get("plane") {
        None | Some(Value::Null) => None,
        Some(plane) => Some([
            field(plane, "origin")?,
            field(plane, "u")?,
            field(plane, "v")?,
        ]),
    };
    encode(brep_core::sketch::extrude(
        profile,
        field(sketch, "closed")?,
        field(v, "height")?,
        field(v, "baseZ")?,
        plane,
    )?)
}

fn handle_gear(v: &Value) -> Result<Value> {
    let number =
        |name: &str, default: f64| -> f64 { v.get(name).and_then(Value::as_f64).unwrap_or(default) };
    let flag = |name: &str| v.get(name).and_then(Value::as_bool).unwrap_or(false);
    let teeth: f64 = field(v, "teeth")?;
    let spec = brep_core::GearSpec {
        module: field(v, "module")?,
        teeth: teeth.round().max(0.) as usize,
        pressure_angle_deg: number("pressureAngle", 20.),
        height: field(v, "height")?,
        helix_angle_deg: number("helixAngle", 0.),
        herringbone: flag("herringbone"),
        bore: number("bore", 0.),
        internal: flag("internal"),
        rim_width: number("rimWidth", 2.),
        clearance: number("clearance", 0.25),
        backlash: number("backlash", 0.),
    };
    if matches!(v["op"].as_str(),Some("verified_gear_edit"|"edited_gear_step")) {return certified_gear_edits::gear(v,&spec);}
    if matches!(v["op"].as_str(),Some("planet_gear_occurrences" | "planet_occurrences_step" | "planetary_gear_set_step")) {
        let sun_teeth = field(v,"sun_teeth")?;
        let angles:Vec<f64> = field(v,"orbit_angles")?;
        let index = field(v,"index")?;
        let assembly = brep_core::gear::planetary_gear_occurrences(&spec,sun_teeth,&angles)?;
        let model = assembly.materialize(index)?;
        let full = v["op"].as_str() == Some("planetary_gear_set_step");
        let step = if full {
            let sun = brep_core::gear(&brep_core::GearSpec { teeth:sun_teeth, ..spec.clone() })?;
            let ring = brep_core::gear(&brep_core::GearSpec { teeth:sun_teeth+2*spec.teeth, internal:true, ..spec.clone() })?;
            Some(cad_step::export_planetary_gear_set_step(&sun,&ring,&assembly)?)
        } else if v["op"].as_str() == Some("planet_occurrences_step") {
            Some(cad_step::export_planet_gear_occurrences_step(&assembly)?)
        } else { None };
        return Ok(value_codec::json!({"model":model,"placements":assembly.placements(),"selectedPlanet":index,
            "prototypeFaces":assembly.prototype().faces.len(),"definitionCount":if full {3} else {1},
            "occurrenceCount":angles.len()+if full {2} else {0},"step":step}));
    }
    if v["op"].as_str() == Some("brep_nurbs_gear_span") {
        return gear_span_report(&spec, v);
    }
    if v["op"].as_str() == Some("brep_nurbs_verified_gear") {
        let gear = brep_core::gear::gear_with_verified_profile(&spec, Default::default())?;
        return Ok(value_codec::json!({
            "model":gear.model(), "profileErrorUpperMm":gear.geometry().profile_error_upper,
            "idealWallErrorUpperMm":gear.geometry().ideal_wall_error_upper,
            "profileWinding":gear.profile().winding(), "solidGeometryCertified":gear.solid_geometry_certified(),
        }));
    }
    encode(brep_core::gear(&spec)?)
}

fn gear_span_report(spec: &brep_core::GearSpec, v: &Value) -> Result<Value> {
    use nurbs_core::involute::{external_spur_span, measurable_spur_spans, tooth_thinning_from_span};
    if spec.internal || spec.helix_angle_deg != 0. || spec.herringbone {
        return Err(crate::input("Base tangent measurement requires an external spur gear"));
    }
    let counts = measurable_spur_spans(spec.module, spec.teeth, spec.pressure_angle_deg, spec.clearance, spec.backlash)?;
    let count = v.get("spanCount").map(|_| field::<usize>(v, "spanCount")).transpose()?;
    let width = count.map(|k| external_spur_span(spec.module, spec.teeth, spec.pressure_angle_deg, spec.clearance, spec.backlash, k)).transpose()?;
    let thinning = if v.get("measuredSpanMm").is_some() {
        let k = count.ok_or_else(|| crate::input("Measured span requires its tooth count"))?;
        Some(tooth_thinning_from_span(spec.module, spec.teeth, spec.pressure_angle_deg, spec.clearance, k, field(v, "measuredSpanMm")?)?)
    } else {
        None
    };
    Ok(value_codec::json!({"counts":counts,"nominalWidthMm":width,"inferredThinningMm":thinning}))
}
