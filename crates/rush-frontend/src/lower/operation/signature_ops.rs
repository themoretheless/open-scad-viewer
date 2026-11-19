use super::*;

const REQUIRED_ALL_SIGNATURE_OPS: &[&str] = &[
    "iso_curve",
    "curve_remove_knot", "curve_reduce_degree", "curve_rebuild", "surface_remove_knot", "surface_reduce_degree", "surface_rebuild",
    "surface_split",
    "surface_transpose",
    "surface_bezier_patch",
    "curve_edit_weights", "curve_relocate_seam", "surface_edit_weights", "surface_relocate_seam", "curve_exact_insert_knot", "curve_exact_remove_knot", "curve_exact_reduce_degree", "surface_exact_insert_knot", "surface_exact_remove_knot", "surface_exact_reduce_degree", "curve_normalize_weights", "curve_cancel_factor", "surface_cancel_factor", "curve_exact_split", "curve_exact_restrict", "curve_exact_partition", "curve_exact_elevate", "rotate_periodic_surface_at_exact_knot", "insert_and_rotate_periodic_surface_seam", "open_exact_periodic_surface_axis", "restrict_exact_periodic_surface_axis", "restrict_exact_periodic_surface_over_turns", "restrict_exact_doubly_periodic_surface", "restrict_exact_doubly_periodic_surface_over_turns", "partition_exact_periodic_surface_axis", "partition_exact_periodic_surface_over_turns", "partition_exact_doubly_periodic_surface", "partition_exact_doubly_periodic_surface_over_turns", "remove_exact_periodic_surface_knot", "simplify_exact_periodic_surface_axis", "remove_exact_periodic_surface_seam_knot", "simplify_exact_doubly_periodic_surface", "reduce_exact_periodic_surface_period", "repeat_exact_periodic_surface_period", "curve_reparameterize",
    "curve_split",
    "curve_bezier_span",
    "rounded_rectangle_profile", "capsule_profile", "annular_sector_surface", "keyway_profile", "t_slot_profile", "dovetail_profile", "poly_v_belt_profile", "toothed_belt_profile", "o_ring_groove_profile", "retaining_ring_profile", "hex_fastener_profile", "square_fastener_profile",
    "exact_ellipse", "exact_ellipsoid", "exact_ring_torus", "exact_frustum_side", "exact_planar_region",
    "helix_curve",
    "ribbon_surface",
    "variable_pipe_surface",
    "pipe_surface",
    "screw_surface",
    "clothoid_curve",
    "spherical_spiral_curve",
    "toroidal_spiral_curve",
    "torus_knot_curve",
    "helicoid_patches",
    "circle_rectangle_transition",
    "ellipse_transition_surface",
    "circle_transition_surface",
    "helicoid_surface",
    "catenoid_patches",
    "catenoid_surface",
    "catenary_curve",
    "archimedean_spiral_curve",
    "epicycloid_curve",
    "hypocycloid_curve",
    "trochoid_curve",
    "cycloid_curve",
    "lissajous_curve",
    "logarithmic_spiral_curve",
    "involute_curve",
    "elliptic_helix_curve",
    "conical_helix_curve",
    "variable_pitch_helix_curve",
];

const MODIFIER_OPS: &[&str] = &[
    "curve_axis_motion_partition_speed", "curve_axis_motion_partition_motion", "curve_axis_motion_partition_axes", "curve_axis_motion_limits", "curve_axis_motion_slowdown", "curve_axis_motion_multi_slowdown", "curve_axis_motion_duration", "curve_axis_motion_deadline", "curve_directional_travel_full", "curve_directional_travel_signed", "curve_check_backtracking", "surface_measure_trimmed_mass", "curve_axial_derivative_peak", "curve_measure_directional_travel", "curve_measure_directional_travel_batch",
    "polygon_extrude",
    "polygon_sweep",
    "mesh_to_nurbs_brep",
    "mesh_to_sdf",
    "mesh_to_subdivision",
    "subdivision_tessellate",
    "mesh_to_nurbs",
    "mesh_fit_nurbs",
    "nurbs_patches_tessellate",
    "sdf_tessellate",
    "sdf_offset",
    "sdf_translate",
    "brep_tessellate",
    "brep_smooth_miter_stations",
    "brep_smooth_sweep_stations",
    "brep_extrude",
    "brep_revolve",
    "brep_chamfer",
    "brep_fillet",
    "brep_full_round_exact",
    "ribbon_surface",
    "variable_pipe_surface",
    "pipe_surface",
    "screw_surface",
    "surface_extrude_patches",
    "iso_curve",
    "curve_remove_knot", "curve_reduce_degree", "curve_rebuild", "surface_remove_knot", "surface_reduce_degree", "surface_rebuild",
    "surface_split",
    "surface_transpose",
    "surface_bezier_patch",
    "curve_edit_weights", "curve_relocate_seam", "surface_edit_weights", "surface_relocate_seam", "curve_exact_insert_knot", "curve_exact_remove_knot", "curve_exact_reduce_degree", "surface_exact_insert_knot", "surface_exact_remove_knot", "surface_exact_reduce_degree", "curve_normalize_weights", "curve_cancel_factor", "surface_cancel_factor", "curve_exact_split", "curve_exact_restrict", "curve_exact_partition", "curve_exact_elevate", "rotate_periodic_surface_at_exact_knot", "insert_and_rotate_periodic_surface_seam", "open_exact_periodic_surface_axis", "restrict_exact_periodic_surface_axis", "restrict_exact_periodic_surface_over_turns", "restrict_exact_doubly_periodic_surface", "restrict_exact_doubly_periodic_surface_over_turns", "partition_exact_periodic_surface_axis", "partition_exact_periodic_surface_over_turns", "partition_exact_doubly_periodic_surface", "partition_exact_doubly_periodic_surface_over_turns", "remove_exact_periodic_surface_knot", "simplify_exact_periodic_surface_axis", "remove_exact_periodic_surface_seam_knot", "simplify_exact_doubly_periodic_surface", "reduce_exact_periodic_surface_period", "repeat_exact_periodic_surface_period", "curve_reparameterize",
    "curve_split",
    "curve_bezier_span",
    "curve_edit",
    "surface_edit",
    "surface_extrude",
    "surface_revolve",
    "tessellate",
    "thicken",
    "transform",
    "extrude",
    "revolve",
    "translate",
    "rotate",
    "scale",
    "offset",
    "mirror",
];

impl Compiler {
    pub(super) fn signature_operation(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        normalize_shorthand_args(name, &mut args)?;
        if name == "offset" && args.iter().any(|(n, _)| n.as_deref() == Some("delta")) {
            if input.is_none() || args.len() != 1 || args[0].0.as_deref() != Some("delta") {
                return Err(crate::error(
                    "offset(delta: distance) requires one argument and piped geometry",
                ));
            }
            return self.add(
                json!({"op":"offset","input":input,"distance":raw(args.remove(0).1)?,"mode":"delta"}),
            );
        }
        let sig =
            signature(name).ok_or_else(|| crate::error(format!("Unknown operation {name}")))?;
        let mut node = json!({"op":name});
        for (i, (n, v)) in args.into_iter().enumerate() {
            let key = n
                .as_deref()
                .or_else(|| sig.get(i).copied())
                .ok_or_else(|| crate::error("Unknown or duplicate argument"))?;
            if !sig.contains(&key) || node.get(key).is_some() {
                return Err(crate::error("Unknown or duplicate argument"));
            }
            node[key] = self.lower_signature_argument(name, key, v)?;
        }
        if REQUIRED_ALL_SIGNATURE_OPS.contains(&name) {
            for key in sig {
                if node.get(*key).is_none() {
                    return Err(crate::error(format!("{name} requires {key}")));
                }
            }
        }
        self.finish_piped_or_standalone_node(name, input, node)
    }

    fn lower_signature_argument(&mut self, name: &str, key: &str, v: V) -> R<J> {
        if (name == "ribbon_surface" && key == "width_law")
            || (name == "variable_pipe_surface" && key == "radius_law")
        {
            return Ok(J::String(self.geometry(v)?));
        }
        if name == "brep_extrude_curves" && key == "loops" {
            return self.lower_extrude_curve_loops(v);
        }
        if [
            "herringbone",
            "internal",
            "radius_of_gyration",
            "left_handed",
            "lower",
            "closed",
            "periodic",
            "periodic_u",
            "periodic_v",
            "reverse",
            "reverse_u",
            "reverse_v",
        ]
        .contains(&key)
        {
            // Text has no boolean literal: true/false lower to 1/0, and the
            // canonical schema wants a JSON boolean for these flags.
            return match raw(v)? {
                J::Bool(b) => Ok(J::Bool(b)),
                J::Number(n) => Ok(J::Bool(n.as_f64().is_some_and(|x| x != 0.))),
                _ => Err(crate::error(format!("{key} expects true or false"))),
            };
        }
        raw(v)
    }

    fn lower_extrude_curve_loops(&mut self, v: V) -> R<J> {
        let V::Array(loops) = v else {
            return Err(crate::error("Curve extrusion requires nested curve lists"));
        };
        if loops.len() > 64 {
            return Err(crate::error("Curve extrusion accepts at most 64 loops"));
        }
        let mut count = 0;
        let lowered = loops
            .into_iter()
            .map(|wire| {
                let V::Array(curves) = wire else {
                    return Err(crate::error("Curve extrusion requires nested curve lists"));
                };
                count += curves.len();
                if curves.is_empty() || count > 254 {
                    return Err(crate::error(
                        "Curve extrusion requires nonempty loops and at most 254 curve references",
                    ));
                }
                curves
                    .into_iter()
                    .map(|curve| {
                        if matches!(curve, V::Array(_)) {
                            return Err(crate::error("Each loop item must be one curve"));
                        }
                        self.geometry(curve).map(J::String)
                    })
                    .collect::<R<Vec<_>>>()
                    .map(J::Array)
            })
            .collect::<R<_>>()?;
        Ok(J::Array(lowered))
    }

    fn finish_piped_or_standalone_node(
        &mut self,
        name: &str,
        input: Option<String>,
        mut node: J,
    ) -> R<V> {
        let modifier = MODIFIER_OPS.contains(&name);
        if modifier && input.is_none() {
            return Err(crate::error(format!("{name} requires piped geometry")));
        }
        if !modifier && input.is_some() {
            return Err(crate::error(format!(
                "{name} does not accept piped geometry"
            )));
        }
        if let Some(i) = input {
            let collection = self.collections.contains(&i);
            if collection && !["translate", "rotate", "scale", "mirror"].contains(&name) {
                return Err(crate::error(
                    "Transform each generated part inside the generator, or explicitly union the collection first",
                ));
            }
            node["input"] = json!(i);
            let result = self.add(node)?;
            if collection {
                let id = self.geometry(result.clone())?;
                self.collections.insert(id);
            }
            return Ok(result);
        }
        self.add(node)
    }
}

fn normalize_shorthand_args(name: &str, args: &mut Vec<(Option<String>, V)>) -> R<()> {
    if ["box", "translate"].contains(&name)
        && args.len() == 3
        && args.iter().all(|(arg_name, _)| arg_name.is_none())
    {
        let values = std::mem::take(args)
            .into_iter()
            .map(|(_, value)| scalar(value).map(V::Json))
            .collect::<R<Vec<_>>>()?;
        *args = vec![(
            Some(if name == "box" { "size" } else { "vector" }.into()),
            V::Array(values),
        )];
    }
    if ["translate", "rotate", "scale"].contains(&name)
        && args
            .iter()
            .any(|(n, _)| matches!(n.as_deref(), Some("x" | "y" | "z")))
    {
        let mut names = Set::new();
        if args.iter().any(|(n, _)| {
            !matches!(n.as_deref(), Some("x" | "y" | "z")) || !names.insert(n.clone())
        }) {
            return Err(crate::error("Use unique x/y/z arguments or one vector"));
        }
        let vec = ["x", "y", "z"]
            .iter()
            .map(|axis| {
                args.iter()
                    .find(|(n, _)| n.as_deref() == Some(axis))
                    .map_or(
                        V::Json(json!(if name == "scale" { 1 } else { 0 })),
                        |(_, v)| v.clone(),
                    )
            })
            .collect();
        *args = vec![(Some("vector".into()), V::Array(vec))];
    }
    Ok(())
}
