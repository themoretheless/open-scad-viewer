use super::*;

impl Compiler {
    pub(super) fn sweep_operation(
        &mut self,
        name: &str,
        input: Option<String>,
        args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        if matches!(name, "brep_miter_sweep" | "brep_progressive_miter_sweep") {
            self.lower_miter_sweep(name, input, args)
        } else {
            self.lower_curve_or_body_sweep(name, input, args)
        }
    }

    fn lower_miter_sweep(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        if input.is_some() {
            return Err(crate::error("Miter sweep takes nested profile loops"));
        }
        let mut node = json!({"op":name});
        for key in ["points", "normal"] {
            node[key] = raw(take_named(
                &mut args,
                key,
                &format!("Miter sweep requires {key}"),
            )?)?;
        }
        if name == "brep_progressive_miter_sweep" {
            self.populate_progressive_miter_options(&mut node, &mut args)?;
        }
        for key in [
            "cap_correction_tolerance",
            "cap_correction_quantum",
            "cap_correction_max_work",
        ] {
            if let Some(val) = take_optional(&mut args, key) {
                node[key] = raw(val)?;
            }
        }
        if name == "brep_miter_sweep" {
            for key in [
                "max_deviation",
                "circle_correction_tolerance",
                "circle_correction_quantum",
                "circle_correction_max_work",
            ] {
                if let Some(val) = take_optional(&mut args, key) {
                    node[key] = raw(val)?;
                }
            }
            if node.get("max_deviation").is_none()
                && [
                    "circle_correction_tolerance",
                    "circle_correction_quantum",
                    "circle_correction_max_work",
                ]
                .iter()
                .any(|key| node.get(*key).is_some())
            {
                return Err(crate::error(
                    "Direct miter circle correction requires max_deviation",
                ));
            }
        }
        if let Some(val) = take_optional(&mut args, "miter_limit") {
            node["miter_limit"] = raw(val)?;
        }
        if let Some(val) = take_optional(&mut args, "closed") {
            node["closed"] = match raw(val)? {
                J::Bool(b) => J::Bool(b),
                J::Number(n) => J::Bool(n.as_f64().is_some_and(|x| x != 0.)),
                _ => return Err(crate::error("closed expects true or false")),
            };
        }
        if args.len() != 1 || args[0].0.is_some() {
            return Err(crate::error(
                "Miter sweep needs one positional nested loop list",
            ));
        }
        node["loops"] = self.lower_nested_curve_loops(
            args.remove(0).1,
            "Miter sweep requires nested loops",
            "Miter sweep needs 1..16 loops",
            "Miter sweep requires nested curves",
            "Miter sweep needs nonempty loops and at most64 curves",
        )?;
        self.add(node)
    }

    fn populate_progressive_miter_options(
        &mut self,
        node: &mut J,
        args: &mut Vec<(Option<String>, V)>,
    ) -> R<()> {
        if let Some(val) = take_optional(args, "orientation_guide") {
            node["orientation_guide"] = json!(self.geometry(val)?);
        }
        populate_vector_laws(node, args)?;
        for key in ["scale", "twist", "max_deviation"] {
            let value = take_named(args, key, &format!("Miter sweep requires {key}"))?;
            node[key] = if matches!(key, "scale" | "twist") {
                lower_record_law(value, "Miter law must be a record")?
            } else {
                raw(value)?
            };
        }
        if let Some(val) = take_optional(args, "cap_correction_authored_frame") {
            let value = raw(val)?;
            node["cap_correction_authored_frame"] = match value {
                J::Bool(b) => J::Bool(b),
                J::Number(n) if n.as_f64() == Some(0.) || n.as_f64() == Some(1.) => {
                    J::Bool(n.as_f64() == Some(1.))
                }
                _ => {
                    return Err(crate::error(
                        "cap_correction_authored_frame expects true or false",
                    ))
                }
            };
        }
        for key in [
            "initial_steps",
            "max_steps",
            "retained_wall_max_injectivity_cells",
            "circle_correction_tolerance",
            "circle_correction_quantum",
            "circle_correction_max_work",
        ] {
            if let Some(val) = take_optional(args, key) {
                node[key] = raw(val)?;
            }
        }
        Ok(())
    }

    fn lower_curve_or_body_sweep(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let mut node = json!({"op":name});
        if name == "brep_progressive_sweep" {
            for key in [
                "cap_correction_tolerance",
                "cap_correction_quantum",
                "cap_correction_max_work",
                "circle_correction_tolerance",
                "circle_correction_quantum",
                "circle_correction_max_work",
            ] {
                if let Some(val) = take_optional(&mut args, key) {
                    node[key] = raw(val)?;
                }
            }
        }
        let is_progressive = matches!(name, "progressive_sweep" | "brep_progressive_sweep");
        if is_progressive {
            if let Some(val) = take_optional(&mut args, "orientation_guide") {
                node["orientation_guide"] = json!(self.geometry(val)?);
            }
        }
        let keys: &[&str] = match name {
            "progressive_sweep" | "brep_progressive_sweep" => {
                &["scale", "twist", "normal", "max_deviation"]
            }
            "profile_sweep" => &["scale", "normal", "sections", "max_deviation"],
            "twist_sweep" => &["origin", "axis", "start_degrees", "sweep_degrees"],
            "two_guide_sweep" => &["width", "axis_y", "axis_z"],
            "scaled_sweep" => &["origin", "scale"],
            _ => &["normal", "sections", "max_deviation"],
        };
        for &key in keys {
            let value = take_named(&mut args, key, &format!("{name} requires named {key}"))?;
            let is_law = (matches!(
                name,
                "scaled_sweep" | "profile_sweep" | "progressive_sweep" | "brep_progressive_sweep"
            ) && key == "scale")
                || (is_progressive && key == "twist");
            node[key] = if is_law {
                lower_record_law(value, &format!("{name} {key} must be a record"))?
            } else {
                raw(value)?
            };
        }
        if is_progressive {
            populate_vector_laws(&mut node, &mut args)?;
            for key in [
                "orientation",
                "spacing",
                "initial_sections",
                "max_sections",
                "length_tolerance",
                "length_max_cells",
                "contact_parameter",
                "contact_profile",
                "rmf_transport_steps",
                "error_max_cells",
                "error_max_products",
            ] {
                if let Some(val) = take_optional(&mut args, key) {
                    let value = raw(val)?;
                    node[key] = if matches!(key, "orientation" | "spacing") {
                        let text = value
                            .as_str()
                            .or_else(|| value.get("text").and_then(J::as_str))
                            .ok_or_else(|| crate::error(format!("{key} must be a text choice")))?;
                        J::String(text.to_owned())
                    } else {
                        value
                    };
                }
            }
        }
        if args.iter().any(|(n, _)| n.is_some()) {
            return Err(crate::error(format!(
                "{name} profile/path are positional; unexpected named argument"
            )));
        }
        if name == "brep_progressive_sweep" {
            if input.is_some() || args.len() != 2 {
                return Err(crate::error(
                    "brep_progressive_sweep requires nested loops followed by a path",
                ));
            }
            node["loops"] = self.lower_nested_curve_loops(
                args.remove(0).1,
                "Sweep body requires nested curve lists",
                "Sweep body needs 1..16 loops",
                "Sweep body requires nested curve lists",
                "Sweep body needs nonempty loops and at most64 curves",
            )?;
            node["inputs"] = json!([self.geometry(args.remove(0).1)?]);
            return self.add(node);
        }
        let mut inputs = Vec::with_capacity(args.len() + usize::from(input.is_some()));
        if let Some(id) = input {
            inputs.push(id);
        }
        for (_, value) in args {
            inputs.push(self.geometry(value)?);
        }
        let invalid_arity = if name == "progressive_sweep" {
            !(2..=65).contains(&inputs.len())
        } else {
            let expected = if name == "two_guide_sweep" { 3 } else { 2 };
            inputs.len() != expected
        };
        if invalid_arity {
            return Err(crate::error(format!(
                "{name} requires profile curves followed by its guide curve"
            )));
        }
        node["inputs"] = json!(inputs);
        self.add(node)
    }

    fn lower_nested_curve_loops(
        &mut self,
        loops_val: V,
        outer_err: &str,
        count_err: &str,
        wire_err: &str,
        curve_limit_err: &str,
    ) -> R<J> {
        let V::Array(loops) = loops_val else {
            return Err(crate::error(outer_err));
        };
        if loops.is_empty() || loops.len() > 16 {
            return Err(crate::error(count_err));
        }
        let mut count = 0;
        let lowered = loops
            .into_iter()
            .map(|wire| {
                let V::Array(curves) = wire else {
                    return Err(crate::error(wire_err));
                };
                count += curves.len();
                if curves.is_empty() || count > 64 {
                    return Err(crate::error(curve_limit_err));
                }
                curves
                    .into_iter()
                    .map(|curve| self.geometry(curve).map(J::String))
                    .collect::<R<Vec<_>>>()
                    .map(J::Array)
            })
            .collect::<R<Vec<_>>>()?;
        Ok(J::Array(lowered))
    }
}

fn lower_record_law(value: V, err: &str) -> R<J> {
    let V::Record(fields, _, _, _) = value else {
        return Err(crate::error(err));
    };
    let mut law = J::Object(value_codec::Map::new());
    for (component, item) in fields {
        law[&component] = raw(item)?;
    }
    Ok(law)
}

fn populate_vector_laws(node: &mut J, args: &mut Vec<(Option<String>, V)>) -> R<()> {
    for key in ["axis_scale", "center_law", "frame_axis", "frame_normal"] {
        if let Some(val) = take_optional(args, key) {
            node[key] = lower_record_law(val, &format!("{key} must be a vector-law record"))?;
        }
    }
    Ok(())
}
