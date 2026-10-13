use super::*;

pub fn compile_text(nodes: Vec<J>, parameters: &[J], mut root: String) -> Result<J> {
    let params = parameters
        .iter()
        .map(|p| {
            let value = p["value"].as_f64().unwrap_or(f64::NAN);
            let numeric = if let Some(unit) = p["unit"].as_str() {
                units::quantity(value, unit, s(p, "id"))
                    .map_err(|error| text_error(s(p, "id"), &error.message))?
            } else {
                Numeric {
                    value,
                    dimension: SCALAR,
                }
            };
            Ok((s(p, "id").to_owned(), numeric))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let nodes = if nodes.iter().any(|n| s(n, "op") == "if") {
        let by_id: BTreeMap<String, J> = nodes
            .into_iter()
            .map(|n| (s(&n, "id").to_owned(), n))
            .collect();
        let mut selected = Vec::new();
        let mut found = BTreeSet::new();
        fn select(
            id: &str,
            depth: usize,
            by_id: &BTreeMap<String, J>,
            params: &BTreeMap<String, Numeric>,
            found: &mut BTreeSet<String>,
            selected: &mut Vec<J>,
        ) -> Result<String> {
            if depth > 64 {
                return Err(text_error(id, "Conditional geometry depth exceeds 64"));
            }
            let node = by_id
                .get(id)
                .ok_or_else(|| text_error(id, "Unknown geometry node"))?;
            if s(node, "op") == "if" {
                let condition = units::scalar(scalar(&node["condition"], params, id, 0)?, id)
                    .map_err(|error| text_error(id, &error.message))?;
                return select(
                    s(node, if condition != 0.0 { "then" } else { "else" }),
                    depth + 1,
                    by_id,
                    params,
                    found,
                    selected,
                );
            }
            if !found.insert(id.into()) {
                return Ok(id.into());
            }
            let slot = selected.len();
            selected.push(node.clone());
            let mut n = node.clone();
            if let Some(input) = node["input"].as_str() {
                n["input"] = json!(select(input, depth + 1, by_id, params, found, selected)?)
            }
            if s(node,"op")=="ribbon_surface" {n["width_law"]=json!(select(s(node,"width_law"),depth+1,by_id,params,found,selected)?);}
            if s(node,"op")=="variable_pipe_surface" {
                n["radius_law"]=json!(select(s(node,"radius_law"),depth+1,by_id,params,found,selected)?);
            }
            if ["progressive_sweep","brep_progressive_sweep","brep_progressive_miter_sweep"].contains(&s(node,"op")) {
                if let Some(id)=node["orientation_guide"].as_str(){n["orientation_guide"]=json!(select(id,depth+1,by_id,params,found,selected)?);}
            }
            for reference_key in ["inputs","u_curves","v_curves"] {
            if let Some(inputs) = node[reference_key].as_array() {
                n[reference_key] = J::Array(
                    inputs
                        .iter()
                        .map(|i| {
                            select(
                                i.as_str().unwrap_or(""),
                                depth + 1,
                                by_id,
                                params,
                                found,
                                selected,
                            )
                            .map(J::String)
                        })
                        .collect::<Result<_>>()?,
                )
            }
            }
            if ["brep_extrude_curves","brep_progressive_sweep","brep_miter_sweep","brep_progressive_miter_sweep"].contains(&s(node,"op")) {
                n["loops"] = J::Array(
                    node["loops"]
                        .as_array()
                        .ok_or_else(|| {
                            text_error(id, "Curve extrusion requires nested curve lists")
                        })?
                        .iter()
                        .map(|wire| {
                            wire.as_array()
                                .ok_or_else(|| {
                                    text_error(id, "Curve extrusion requires nested curve lists")
                                })?
                                .iter()
                                .map(|curve| {
                                    select(
                                        curve.as_str().unwrap_or(""),
                                        depth + 1,
                                        by_id,
                                        params,
                                        found,
                                        selected,
                                    )
                                    .map(J::String)
                                })
                                .collect::<Result<Vec<_>>>()
                                .map(J::Array)
                        })
                        .collect::<Result<_>>()?,
                );
            }
            selected[slot] = n;
            Ok(id.into())
        }
        root = select(&root, 0, &by_id, &params, &mut found, &mut selected)?;
        selected
    } else {
        nodes
    };
    fn field(
        v: &J,
        dimension: [i8; 2],
        params: &BTreeMap<String, Numeric>,
        path: &str,
    ) -> Result<J> {
        if let Some(a) = v.as_array() {
            return a
                .iter()
                .enumerate()
                .map(|(i, v)| field(v, dimension, params, &format!("{path}/{i}")))
                .collect::<Result<Vec<_>>>()
                .map(J::Array);
        }
        Ok(json!(
            units::field(scalar(v, params, path, 0)?, dimension, path, false)
                .map_err(|error| text_error(path, &error.message))?
        ))
    }
    let mut lowered = Vec::new();
    for mut node in nodes {
        if s(&node, "op") == "nurbs_surface" {
            node["op"] = json!("surface")
        }
        if s(&node, "op") == "nurbs_curve" {
            node["op"] = json!("curve")
        }
        let supported = [
            "sphere_surface", "cylinder_surface", "cone_surface",
            "plane_patch", "bilinear_patch", "bezier_surface",
            "bezier_curve", "curve_compose", "round_polyline_curve", "transition_polyline_curve", "brep_miter_sweep","brep_progressive_miter_sweep",
            "line_curve", "polyline_curve", "circle_curve", "circle_arc",
            "hyperboloid_one_sheet", "hyperboloid_two_sheet", "polynomial_graph",
            "polynomial_curve", "polynomial_surface", "rational_polynomial_curve", "rational_polynomial_surface",
            "parabola_curve",
            "hyperbola_curve",
            "elliptic_cylinder_surface",
            "cone_frustum_surface",
            "quadratic_patch",

            "ellipse_arc", "ellipsoid_surface", "torus_surface",
            "polygon_profile",
            "polygon_extrude",
            "polygon_sweep",
            "polygon_loft",
            "ribbon_surface", "variable_pipe_surface", "pipe_surface", "screw_surface", "clothoid_curve", "spherical_spiral_curve", "toroidal_spiral_curve", "torus_knot_curve", "helicoid_patches", "circle_rectangle_transition", "ellipse_transition_surface", "circle_transition_surface", "helicoid_surface", "catenoid_patches", "catenoid_surface", "catenary_curve", "archimedean_spiral_curve", "epicycloid_curve", "hypocycloid_curve", "trochoid_curve", "cycloid_curve", "lissajous_curve", "logarithmic_spiral_curve", "involute_curve", "elliptic_helix_curve", "variable_pitch_helix_curve", "conical_helix_curve", "helix_curve", "surface_sweep", "brep_progressive_sweep", "progressive_sweep", "profile_sweep", "framed_sweep", "scaled_sweep", "two_guide_sweep", "twist_sweep",
            "surface_loft",
            "ruled_surface",
            "coons_patch",
            "guided_loft_surface", "auto_guided_loft_surface", "loft_match_surface", "brep_natural_loft", "brep_capped_loft", "control_tangent_loft_surface", "clamped_loft_surface", "boundary_fill", "triangular_patch", "gordon_surface", "closed_loft_surface", "natural_loft_surface", "grid_spline_surface", "hermite_patch", "closed_spline_curve", "clamped_spline_curve", "natural_spline_curve", "hermite_curve", "formula_curve", "formula_surface",
            "extrude",
            "revolve",
            "triangle_mesh",
            "mesh_to_nurbs_brep",
            "mesh_to_sdf",
            "mesh_to_subdivision",
            "subdivision_tessellate",
            "mesh_to_nurbs",
            "mesh_fit_nurbs",
            "nurbs_patches_tessellate",
            "subdivision",
            "sdf_sphere",
            "sdf_box",
            "sdf_torus",
            "sdf_union",
            "sdf_intersection",
            "sdf_difference",
            "sdf_smooth_union",
            "sdf_offset",
            "sdf_translate",
            "sdf_tessellate",
            "brep_box",
            "brep_sphere",
            "brep_torus",
            "brep_cylinder",
            "brep_frustum",
            "brep_tube",
            "brep_gear",
            "brep_extrude",
            "brep_extrude_curves",
            "brep_revolve",
            "brep_boolean",
            "brep_chamfer",
            "brep_fillet",
            "brep_tessellate",
            "brep_smooth_miter_stations",
            "surface",
            "curve",
            "surface_extrude_patches", "surface_extrude",
            "surface_revolve",
            "tessellate",
            "thicken",
            "mesh_boolean",
            "transform",
        ];
        if !supported.contains(&s(&node, "op")) {
            return Err(text_error(
                s(&node, "id"),
                &format!(
                    "Operation {} cannot be mixed with own NURBS/mesh operations",
                    s(&node, "op")
                ),
            ));
        }
        let progressive_sweep=["progressive_sweep","brep_progressive_sweep","brep_progressive_miter_sweep"].contains(&s(&node,"op"));
        let line_coordinates=s(&node,"op")=="line_curve";
        let scaled_sweep=["scaled_sweep","profile_sweep","progressive_sweep","brep_progressive_sweep","brep_progressive_miter_sweep"].contains(&s(&node,"op"));
        let catenary=s(&node,"op")=="catenary_curve";
        let circle_rectangle=s(&node,"op")=="circle_rectangle_transition";
        let ellipse_transition=s(&node,"op")=="ellipse_transition_surface";
        let circle_transition=s(&node,"op")=="circle_transition_surface";
        let catenoid=["catenoid_surface","catenoid_patches"].contains(&s(&node,"op"));
        let clothoid=s(&node,"op")=="clothoid_curve";
        let pipe=["profile_sweep","framed_sweep","pipe_surface","variable_pipe_surface","ribbon_surface"].contains(&s(&node,"op"));
        let loft_refs=["loft_match_surface","brep_natural_loft","brep_capped_loft"].contains(&s(&node,"op"));
        for (key, value) in node.as_object_mut().unwrap() {
            if loft_refs && ["sections","guides","start_reference","end_reference","start","end","sides","start_boundary","end_boundary","start_reverse","end_reverse","cap_surfaces","cap_trims"].contains(&key.as_str()){continue;}
            if ["id", "op", "orientation", "spacing", "width_law", "radius_law", "orientation_guide", "input", "inputs", "u_curves", "v_curves", "guides", "operation", "loops", "lower", "closed", "periodic", "construction"].contains(&key.as_str()) {
                continue;
            }
            if key=="section_mappings" {
                fn maps(value:&J,params:&BTreeMap<String,Numeric>,path:&str)->Result<J> {
                    if value.is_null() {return Ok(J::Null);}
                    if let Some(items)=value.as_array() {return Ok(J::Array(items.iter().enumerate().map(|(i,v)|maps(v,params,&format!("{path}/{i}"))).collect::<Result<_>>()?));}
                    if let Some(object)=value.as_object() {
                        if object.contains_key("pieces") || object.contains_key("composition") || object.contains_key("controlValues") {
                            return Ok(J::Object(object.iter().map(|(k,v)|Ok((k.clone(),maps(v,params,&format!("{path}/{k}"))?))).collect::<Result<_>>()?));
                        }
                    }
                    field(value,SCALAR,params,path)
                }
                *value=maps(value,&params,key)?;
                continue;
            }
            if key=="embedding_limits" {
                for (name,limit) in value.as_object_mut().ok_or_else(||text_error(key,"Expected embedding limits"))? {
                    *limit=field(limit,SCALAR,&params,&format!("{key}/{name}"))?;
                }
                continue;
            }
            if key == "expressions" {
                let groups=value.as_array().ok_or_else(||text_error(key,"Expected three formula token lists"))?;
                *value=J::Array(groups.iter().enumerate().map(|(axis,group)| {
                    if let Some(text)=group.as_str().or_else(||group.get("text").and_then(J::as_str)){return Ok(J::String(text.to_owned()));}
                    let tokens=group.as_array().ok_or_else(||text_error(key,"Expected formula text or a token list"))?;
                    tokens.iter().enumerate().map(|(i,token)| {
                        if let Some(text)=token.as_str().or_else(||token.get("text").and_then(J::as_str)) {Ok(J::String(text.to_owned()))}
                        else {field(token,SCALAR,&params,&format!("{key}/{axis}/{i}"))}
                    }).collect::<Result<Vec<_>>>().map(J::Array)
                }).collect::<Result<Vec<_>>>()?);
            } else if key == "matrix" {
                let rows = value
                    .as_array()
                    .ok_or_else(|| text_error(key, "Expected a matrix"))?;
                let mut matrix = Vec::new();
                for (i, row) in rows.iter().enumerate() {
                    let row = row
                        .as_array()
                        .ok_or_else(|| text_error(key, "Expected a matrix"))?;
                    matrix.push(J::Array(
                        row.iter()
                            .enumerate()
                            .map(|(j, v)| {
                                field(
                                    v,
                                    if i < 3 && j == 3 { LENGTH } else { SCALAR },
                                    &params,
                                    &format!("{key}/{i}/{j}"),
                                )
                            })
                            .collect::<Result<_>>()?,
                    ))
                }
                *value = J::Array(matrix)
            } else if progressive_sweep && ["axis_scale","center_law","frame_axis","frame_normal"].contains(&key.as_str()) {
                let law=value.as_object_mut().ok_or_else(||text_error(key,"Sweep vector law must be an object"))?;
                for (component,entry) in law {
                    *entry=field(entry,if key=="center_law" && component=="values" {LENGTH} else {SCALAR},&params,&format!("{key}/{component}"))?;
                }
            } else if key == "twist" && progressive_sweep {
                let law=value.as_object_mut().ok_or_else(||text_error(key,"Sweep twist must be an object"))?;
                for (component,entry) in law {
                    *entry=field(entry,if component=="values" {ANGLE} else {SCALAR},&params,&format!("twist/{component}"))?;
                }
            } else if key == "scale" && scaled_sweep {
                let law=value.as_object_mut().ok_or_else(||text_error(key,"Sweep scale must be an object"))?;
                for (component,entry) in law {
                    *entry=field(entry,SCALAR,&params,&format!("scale/{component}"))?;
                }
            } else {
                let dimension = if progressive_sweep && key=="length_tolerance" {LENGTH} else if circle_rectangle {if ["circle_normal","circle_seam"].contains(&key.as_str()) {SCALAR} else {LENGTH}} else if ellipse_transition {LENGTH} else if circle_transition && ["start_center","end_center","start_radius","end_radius"].contains(&key.as_str()) {LENGTH} else if pipe && key=="sections" {SCALAR} else if clothoid && ["start_curvature","end_curvature"].contains(&key.as_str()) {[-1,0]} else if clothoid && key=="length" {LENGTH} else if catenoid && ["scale","start_z","end_z"].contains(&key.as_str()) || catenary && ["scale","start_x","end_x"].contains(&key.as_str()) || line_coordinates && ["start","end"].contains(&key.as_str()) || [
                    "height",
                    "width",
                    "z_min",
                    "z_max",
                    "outer",
                    "holes",
                    "sections",
                    "path",
                    "tangent_u", "tangent_v", "twist", "points", "tangents", "start_tangent", "end_tangent", "start_tangents", "end_tangents",
                    "corners",
                    "wall_tolerance", "quantum", "max_deviation", "budget", "error_budget", "cap_correction_tolerance", "cap_correction_quantum", "circle_correction_tolerance", "circle_correction_quantum",
                    "vertices",
                    "axis_u", "axis_v", "radii", "radial_radius", "axial_radius",
                    "radius_x", "radius_y", "bounds",
                    "fixed_radius", "rolling_radius", "tracing_radius", "amplitudes", "start_radius", "end_radius", "start_pitch", "end_pitch",
                    "center",
                    "half_size",
                    "radius",
                    "bottom_radius",
                    "top_radius",
                    "inner_radius",
                    "outer_radius",
                    "module",
                    "bore",
                    "rim_width",
                    "clearance",
                    "backlash",
                    "size",
                    "major_radius",
                    "minor_radius",
                    "distance", "setback",
                    "min",
                    "max",
                    "control_points",
                    "vector",
                    "origin",
                ]
                .contains(&key.as_str())
                {
                    LENGTH
                } else if ["angle", "pressure_angle", "helix_angle", "start_degrees", "sweep_degrees", "phase_degrees", "end_degrees", "phases_degrees", "major_phase_degrees", "minor_phase_degrees", "longitude_phase_degrees", "latitude_phase_degrees"].contains(&key.as_str()) {
                    ANGLE
                } else {
                    SCALAR
                };
                *value = field(value, dimension, &params, key)?
            }
        }
        lowered.push(node)
    }
    compile(
        json!({"language":"modelgraph/nurbs-1","units":"mm","parameters":[],"nodes":lowered,"root":root}),
    )
}
