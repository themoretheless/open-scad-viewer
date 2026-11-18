//! Primitive solids, extrusions, revolutions, faceted constructors, and transforms.
use crate::*;

pub(super) const OPS: &[&str] = &[
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
    "brep_nurbs_gear",
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
        "brep_nurbs_gear" => handle_gear(&v),
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
    encode(brep_core::gear(&spec)?)
}
