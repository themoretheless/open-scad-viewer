use super::*;

pub(super) fn has_assertion(value: &J) -> bool {
    match value {
        J::Object(fields) => {
            ["assert_value", "geometry_effects"].contains(&s(value, "op"))
                || fields.values().any(has_assertion)
        }
        J::Array(items) => items.iter().any(has_assertion),
        _ => false,
    }
}
pub(super) fn gather(v: &V, out: &mut Vec<J>) {
    match v {
        V::Void(checks) => out.extend(checks.iter().cloned()),
        V::Array(a) => a.iter().for_each(|v| gather(v, out)),
        V::Record(r, _, _, _) => r.values().for_each(|v| gather(v, out)),
        V::Json(j) if ["typed", "typed_value", "checked", "memo"].contains(&s(j, "op")) => {
            out.push(j.clone())
        }
        V::Json(j) if j.get("sequence").is_some() => gather(&V::Json(j["sequence"].clone()), out),
        _ => {}
    }
}
pub(super) fn infer(expected: &J, actual: &J, g: &Set<String>, bs: &mut Types, explicit: bool) -> R<()> {
    let name = s(expected, "name");
    if g.contains(name) {
        if let Some(old) = bs.get(name) {
            if !explicit && typename(old) != typename(actual) {
                return Err(crate::error(format!(
                    "Conflicting inference for {name}: {} and {}",
                    typename(old),
                    typename(actual)
                )));
            }
        } else {
            bs.insert(name.into(), actual.clone());
        }
    } else if name == s(actual, "name") {
        for (e, a) in arr(expected, "args").iter().zip(arr(actual, "args")) {
            infer(e, a, g, bs, explicit)?
        }
    }
    Ok(())
}
pub(super) fn binding_names(pattern: &J) -> Vec<&str> {
    if s(pattern, "kind") == "name" {
        vec![s(pattern, "value")]
    } else {
        arr(pattern, "items")
            .iter()
            .map(|item| s(item, "value"))
            .collect()
    }
}
impl Compiler {
    pub(super) fn bind(&mut self, p: &J, v: V, e: &mut Env) -> R<Vec<J>> {
        if matches!(&v, V::Void(_)) {
            return Err(crate::error("Function has no return value"));
        }
        if s(p, "kind") == "name" {
            let name = s(p, "value");
            if e.contains_key(name) {
                return Err(crate::error(format!("Duplicate name {name}")));
            }
            e.insert(name.into(), v);
            return Ok(Vec::new());
        }
        if s(p, "kind") == "list_pattern" {
            let items = arr(p, "items");
            for item in items {
                if e.contains_key(s(item, "value")) {
                    return Err(crate::error(format!("Duplicate name {}", s(item, "value"))));
                }
            }
            if let V::Array(values) = v {
                if values.len() != items.len() {
                    return Err(crate::error(format!(
                        "List destructuring requires exactly {} elements",
                        items.len()
                    )));
                }
                for (item, value) in items.iter().zip(values) {
                    e.insert(s(item, "value").into(), value);
                }
                return Ok(Vec::new());
            }
            let input = json!({"op":"memo","id":self.check_id("m"),"value":list(v)?});
            let constraint = json!({"id":self.check_id("v"),"left":{"op":"length","input":&input},"relation":"eq","right":items.len(),"message":format!("List destructuring requires exactly {} elements",items.len())});
            let check = json!({"op":"memo","id":self.check_id("m"),"value":{"op":"assert_value","constraints":[constraint],"value":1}});
            for (index, item) in items.iter().enumerate() {
                e.insert(s(item,"value").into(),V::Json(json!({"op":"at","input":{"op":"checked","checks":[&check],"value":&input},"index":index})));
            }
            return Ok(vec![check]);
        }
        let V::Record(r, _, _, _) = v else {
            return Err(crate::error("Destructuring requires a record"));
        };
        for item in arr(p, "items") {
            let name = s(item, "value");
            if !r.contains_key(name) {
                return Err(crate::error(format!("Unknown result field {name}")));
            }
            if e.contains_key(name) {
                return Err(crate::error(format!("Duplicate name {name}")));
            }
        }
        for item in arr(p, "items") {
            let name = s(item, "value");
            e.insert(name.into(), r[name].clone());
        }
        Ok(Vec::new())
    }
}
pub fn compile(statements: Vec<Statement>) -> R<J> { compile_with_ids(statements, false) }
pub fn compile_with_ids(statements: Vec<Statement>, stable_ids: bool) -> R<J> {
    let mut c = Compiler {
        nodes: vec![],
        parameters: vec![],
        controls: vec![],
        constraints: vec![],
        checks: vec![],
        collections: Set::new(),
        structs: Map::new(),
        traits: Map::new(),
        trait_envs: Map::new(),
        implementations: Map::new(),

        active: Types::new(),
        serial: 0,
        stable_ids,
        owner: String::new(),
        owner_serial: 0,
        show_serial: 0,
        author_ids: Set::new(),
        check_serial: 0,
        expansions: 0,
    };
    let mut e = Env::new();
    let mut root = String::new();
    let mut segments = None;
    for statement in statements {
        let a = statement.node;
        c.owner.clear();
        c.owner_serial = 0;
        let result: R<()> = try {
            let name = s(&a, "name");
            match s(&a, "kind") {
                "bind" => {
                    if e.contains_key(name) {
                        do yeet crate::error(format!("Duplicate name {name}"));
                    }
                    c.owner = if let Some(id) = a.get("author_id").and_then(J::as_str) {
                        if !c.author_ids.insert(id.to_owned()) { do yeet crate::error(format!("Duplicate node ID {id}")); }
                        format!("ri_{id}")
                    } else { format!("rb_{name}") };
                    c.owner_serial = 0;
                    let v = c.eval(&a["value"], &e, 0)?;
                    if matches!(&v, V::Void(_)) {
                        do yeet crate::error("Function has no return value");
                    }
                    if c.needs_assertion(&v) {
                        c.statement_checks(&v);
                    }
                    if let V::Json(j) = &v
                        && j.get("geometry").is_some()
                    {
                        root = s(j, "geometry").into()
                    }
                    e.insert(name.into(), v);
                }
                "trait" => {
                    if c.traits.contains_key(name)
                        || c.structs.contains_key(name)
                        || [
                            "int", "f32", "f64", "str", "length", "angle", "Geometry", "Vec",
                        ]
                        .contains(&name)
                    {
                        do yeet crate::error(format!("Duplicate or reserved type {name}"));
                    }
                    c.validate_trait(&a)?;
                    c.traits.insert(name.into(), a.clone());
                    c.trait_envs.insert(name.into(), e.clone());
                }
                "impl" => c.register_impl(&a, &e)?,
                "struct" => {
                    if c.structs.contains_key(name)
                        || c.traits.contains_key(name)
                        || [
                            "int", "f32", "f64", "str", "length", "angle", "Geometry", "Vec",
                        ]
                        .contains(&name)
                    {
                        do yeet crate::error(format!("Duplicate or reserved type {name}"));
                    }
                    c.structs.insert(name.into(), a.clone());
                    let g = arr(&a, "generics")
                        .iter()
                        .map(|g| g.as_str().unwrap().into())
                        .collect();
                    for f in arr(&a, "fields") {
                        c.validate(&f["type"], &g, &Set::from([name.into()]))?
                    }
                }
                "destructure" => {
                    let v = c.eval(&a["value"], &e, 0)?;
                    if c.needs_assertion(&v) {
                        c.statement_checks(&v);
                    }
                    let checks = c.bind(&a["pattern"], v, &mut e)?;
                    if !checks.is_empty() {
                        c.statement_checks(&V::Void(checks));
                    }
                }
                "validate" | "assert" => c.check(s(&a, "kind"), &a["value"], &e)?,
                "segments" => {
                    if segments.is_some() {
                        do yeet crate::error("Duplicate segments declaration");
                    }
                    segments = Some(a["value"].clone())
                }
                "param" => {
                    if e.contains_key(name) {
                        do yeet crate::error(format!("Duplicate name {name}"));
                    }
                    c.parameters.push(a["parameter"].clone());
                    c.controls.push(a["control"].clone());
                    e.insert(name.into(), V::Json(json!({"param":name})));
                }
                "show" => {
                    c.show_serial += 1;
                    c.owner = format!("rs_{}", c.show_serial);
                    c.owner_serial = 0;
                    let v = c.eval(&a["value"], &e, 0)?;
                    root = c.geometry(v)?
                }
                "statement" => {
                    let value = c.eval(&a["value"], &e, 0)?;
                    c.statement_checks(&value);
                }
                _ => do yeet crate::error("Unknown statement"),
            }
        };
        result
            .map_err(|e| crate::error(format!("ModelGraph Text line {}: {}", statement.line, e)))?;
    }
    if root.is_empty() {
        return Err(crate::error("No geometry to show"));
    }
    let mut result = json!({"nodes":c.nodes,"parameters":c.parameters,"customizer":c.controls,"root":root,"constraints":c.constraints,"checks":c.checks});
    if let Some(s) = segments {
        result["segments"] = s
    }
    Ok(result)
}
pub(super) fn signature(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "polygon" => &["points"],
        "polygon_profile" => &["outer", "holes"],
        "polygon_extrude" => &["vector"],
        "polygon_sweep" => &["path", "up"],
        "polygon_loft" => &["sections"],
        "triangle_mesh" => &["vertices", "triangles"],
        "mesh_to_nurbs_brep" => &[],
        "mesh_to_sdf" => &[],
        "mesh_to_subdivision" => &["iterations"],
        "subdivision_tessellate" => &["levels"],
        "mesh_to_nurbs" => &[],
        "mesh_fit_nurbs" => &["max_deviation"],
        "nurbs_patches_tessellate" => &["segments"],
        "subdivision" => &["vertices", "faces", "levels"],
        "sdf_sphere" => &["center", "radius"],
        "sdf_box" => &["center", "half_size"],
        "sdf_torus" => &["center", "major_radius", "minor_radius"],
        "sdf_tessellate" => &["min", "max", "cells"],
        "sdf_offset" => &["distance"],
        "sdf_translate" => &["vector"],
        "brep_box" => &["min", "max"],
        "brep_sphere" => &["radius"],
        "brep_torus" => &["major_radius", "minor_radius"],
        "brep_cylinder" => &["radius", "height"],
        "brep_frustum" => &["bottom_radius", "top_radius", "height"],
        "brep_tube" => &["outer_radius", "inner_radius", "height"],
        "brep_gear" => &[
            "module",
            "teeth",
            "height",
            "pressure_angle",
            "helix_angle",
            "herringbone",
            "bore",
            "internal",
            "rim_width",
            "clearance",
            "backlash",
        ],
        "brep_revolve" => &["angle"],
        "brep_extrude" => &["height"],
        "brep_extrude_curves" => &["loops", "z_min", "z_max"],
        "brep_chamfer" => &["edges", "size"],
        "brep_fillet" => &["edges", "radius", "segments"],
        "brep_tessellate" => &["segments"],
        "brep_smooth_miter_stations" => &["wall_tolerance","quantum","max_work","max_deviation"],
        "nurbs_surface" => &[
            "degree_u",
            "degree_v",
            "knots_u",
            "knots_v",
            "control_points",
            "weights",
        ],
        "nurbs_curve" => &["degree", "knots", "control_points", "weights", "periodic"],
        "parabola_curve" | "hyperbola_curve" => &["center", "axis_u", "axis_v", "start", "end"],
        "elliptic_cylinder_surface" => &["center", "radius_x", "radius_y", "height"],
        "cone_frustum_surface" => &["center", "bottom_radius", "top_radius", "height"],
        "quadratic_patch" => &["bounds", "coefficients"],
        "line_curve" => &["start", "end"],
        "bezier_curve" => &["points", "weights"],
        "round_polyline_curve" => &["points", "radius", "closed"],
        "transition_polyline_curve" => &["points", "setback", "closed"],
        "closed_spline_curve" => &["points", "parameters"],
        "clamped_spline_curve" => &["points", "parameters", "start_tangent", "end_tangent"],
        "natural_spline_curve" => &["points", "parameters"],
        "hermite_curve" => &["points", "tangents", "parameters"],
        "clothoid_curve" => &["center", "length", "start_curvature", "end_curvature", "phase_degrees", "max_deviation"],
        "spherical_spiral_curve" => &["center", "radius", "longitude_turns", "latitude_turns", "longitude_phase_degrees", "latitude_phase_degrees", "max_deviation"],
        "toroidal_spiral_curve" => &["center", "major_radius", "minor_radius", "major_turns", "minor_turns", "major_phase_degrees", "minor_phase_degrees", "max_deviation"],
        "torus_knot_curve" => &["center", "major_radius", "minor_radius", "p", "q", "major_phase_degrees", "minor_phase_degrees", "max_deviation"],
        "helicoid_patches" => &["center", "inner_radius", "outer_radius", "height", "turns", "phase_degrees", "max_deviation"],
        "circle_rectangle_transition" => &["circle_center","circle_normal","circle_seam","circle_radius","rectangle_center","rectangle_axis_u","rectangle_axis_v"],
        "ellipse_transition_surface" => &["start_center","start_axis_u","start_axis_v","end_center","end_axis_u","end_axis_v"],
        "circle_transition_surface" => &["start_center","start_normal","start_seam","start_radius","end_center","end_normal","end_seam","end_radius"],
        "helicoid_surface" => &["center", "inner_radius", "outer_radius", "height", "turns", "phase_degrees", "max_deviation"],
        "catenoid_patches" => &["center", "scale", "start_z", "end_z", "max_deviation"],
        "catenoid_surface" => &["center", "scale", "start_z", "end_z", "max_deviation"],
        "catenary_curve" => &["center", "scale", "start_x", "end_x", "max_deviation"],
        "archimedean_spiral_curve" => &["center", "start_radius", "end_radius", "start_degrees", "end_degrees", "max_deviation"],
        "epicycloid_curve" => &["center", "fixed_radius", "rolling_radius", "start_degrees", "end_degrees", "max_deviation"],
        "hypocycloid_curve" => &["center", "fixed_radius", "rolling_radius", "start_degrees", "end_degrees", "max_deviation"],
        "trochoid_curve" => &["center", "rolling_radius", "tracing_radius", "start_degrees", "end_degrees", "max_deviation"],
        "cycloid_curve" => &["center", "radius", "start_degrees", "end_degrees", "max_deviation"],
        "lissajous_curve" => &["center", "amplitudes", "frequencies", "phases_degrees", "max_deviation"],
        "logarithmic_spiral_curve" => &["center", "radius", "growth", "start_degrees", "end_degrees", "max_deviation"],
        "involute_curve" => &["center", "radius", "start_degrees", "end_degrees", "max_deviation"],
        "helix_curve" => &["center", "radius", "height", "turns", "phase_degrees", "max_deviation"],
        "elliptic_helix_curve" => &["center", "radius_x", "radius_y", "height", "turns", "phase_degrees", "max_deviation"],
        "variable_pitch_helix_curve" => &["center", "radius", "height", "turns", "start_pitch", "end_pitch", "phase_degrees", "max_deviation"],
        "conical_helix_curve" => &["center", "start_radius", "end_radius", "height", "turns", "phase_degrees", "max_deviation"],
        "formula_curve" | "formula_surface" => &["domain", "expressions"],
        "sphere_surface" => &["center", "radius"],
        "cylinder_surface" | "cone_surface" => &["center", "radius", "height"],
        "plane_patch" => &["origin", "axis_u", "axis_v"],
        "grid_spline_surface" => &["points", "parameters_u", "parameters_v"],
        "hermite_patch" => &["corners", "tangent_u", "tangent_v", "twist"],
        "bilinear_patch" => &["corners"],
        "bezier_surface" => &["points", "weights"],
        "polyline_curve" => &["points", "closed"],
        "circle_curve" => &["center", "normal", "radius"],
        "circle_arc" => &["center", "normal", "radius", "start_degrees", "sweep_degrees"],
        "hyperboloid_one_sheet" => &["center", "radii", "start", "end"],
        "hyperboloid_two_sheet" => &["center", "radii", "start", "end", "lower"],
        "polynomial_graph" => &["bounds", "coefficients"],
        "polynomial_curve" | "polynomial_surface" | "rational_polynomial_curve" | "rational_polynomial_surface" => &["domain", "coefficients"],
        "ellipse_arc" => &["center", "axis_u", "axis_v", "start_degrees", "sweep_degrees"],
        "ellipsoid_surface" => &["center", "radii"],
        "torus_surface" => &["center", "major_radius", "radial_radius", "axial_radius"],
        "ribbon_surface" => &["width_law", "normal", "sections", "max_deviation"],
        "variable_pipe_surface" => &["radius_law", "normal", "sections", "max_deviation"],
        "pipe_surface" => &["radius", "normal", "sections", "max_deviation"],
        "screw_surface" => &["origin", "axis", "height", "turns", "phase_degrees", "max_deviation"],
        "surface_extrude_patches" => &["vector"],
        "surface_extrude" => &["vector"],
        "surface_revolve" => &["origin", "axis", "angle"],
        "tessellate" => &["segments_u", "segments_v"],
        "thicken" => &["vector"],
        "transform" => &["matrix"],
        "gear" => &[
            "teeth",
            "module",
            "thickness",
            "pressure_angle",
            "bore",
            "backlash",
            "clearance",
            "internal",
            "rim_width",
            "flank_segments",
            "helix_angle",
            "herringbone",
        ],
        "planetary_gears" => &[
            "sun_teeth",
            "planet_teeth",
            "planet_count",
            "module",
            "thickness",
            "pressure_angle",
            "bore",
            "backlash",
            "clearance",
            "rim_width",
            "flank_segments",
            "carrier_angle",
            "helix_angle",
            "herringbone",
        ],
        "planetary_spinner" => &[
            "inner_radius",
            "outer_radius",
            "bore",
            "gap",
            "height",
            "helix_angle",
        ],
        "circle" => &["radius"],
        "rectangle" => &["size"],
        "box" => &["size"],
        "sphere" => &["radius"],
        "cylinder" => &["radius", "height"],
        "extrude" => &["height"],
        "revolve" => &["angle"],
        "translate" => &["vector"],
        "rotate" => &["vector"],
        "scale" => &["vector"],
        "offset" => &["distance"],
        "mirror" => &["normal"],
        _ => return None,
    })
}
