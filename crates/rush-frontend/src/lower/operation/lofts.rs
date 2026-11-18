use super::*;

impl Compiler {
    pub(super) fn loft_operation(
        &mut self,
        name: &str,
        input: Option<String>,
        args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        match name {
            "boundary_fill" => self.lower_boundary_fill(name, input, args),
            "gordon_surface" => self.lower_gordon_surface(name, input, args),
            "loft_match_surface" | "brep_natural_loft" | "brep_capped_loft" => {
                self.lower_brep_loft_family(name, input, args)
            }
            "loft_embedding_limits" => Self::lower_loft_embedding_limits(input, args),
            "loft_parameter_map" => Self::lower_loft_parameter_map(input, args),
            "guided_loft_surface"
            | "auto_guided_loft_surface"
            | "control_tangent_loft_surface"
            | "natural_loft_surface" => self.lower_section_loft_surface(name, input, args),
            "clamped_loft_surface" => self.lower_clamped_loft_surface(name, input, args),
            "closed_loft_surface" => self.lower_closed_loft_surface(name, input, args),
            _ => unreachable!("unhandled loft operation {name}"),
        }
    }

    fn lower_boundary_fill(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let center = raw(take_named(&mut args, "center", "boundary_fill requires named center")?)?;
        let inputs = self.collect_positional_geometry(
            input,
            args,
            "Boundary fill edges must be positional",
        )?;
        if !(3..=32).contains(&inputs.len()) {
            return Err(crate::error("Boundary fill needs 3..32 edges"));
        }
        self.add(json!({"op":name,"inputs":inputs,"center":center}))
    }

    fn lower_gordon_surface(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        if input.is_some() {
            return Err(crate::error("Gordon families must be named"));
        }
        let mut node = json!({"op":name});
        for key in ["u_curves", "v_curves"] {
            let value = take_named(
                &mut args,
                key,
                &format!("gordon_surface requires named {key}"),
            )?;
            let V::Array(values) = value else {
                return Err(crate::error("Gordon families must be curve arrays"));
            };
            let mut ids = Vec::with_capacity(values.len());
            for item in values {
                ids.push(self.geometry(item)?);
            }
            node[key] = json!(ids);
        }
        for key in ["parameters_u", "parameters_v"] {
            node[key] = raw(take_named(
                &mut args,
                key,
                &format!("gordon_surface requires named {key}"),
            )?)?;
        }
        if !args.is_empty() {
            return Err(crate::error("Unexpected Gordon arguments"));
        }
        self.add(node)
    }

    fn lower_brep_loft_family(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let mut node = json!({"op":name});
        if name == "loft_match_surface" {
            let source = if let Some(id) = input {
                id
            } else {
                if args.is_empty() || args[0].0.is_some() {
                    return Err(crate::error("Loft match needs a positional surface"));
                }
                self.geometry(args.remove(0).1)?
            };
            node["input"] = json!(source);
        } else if input.is_some() {
            return Err(crate::error("Capped loft uses named loop arrays"));
        }
        for (key, value) in args {
            let key = key.ok_or_else(|| crate::error("Loft options must be named"))?;
            let depth = match (name, key.as_str()) {
                ("brep_natural_loft", "sections") => Some(3),
                ("brep_capped_loft", "start" | "end" | "sides") => Some(2),
                ("brep_capped_loft", "cap_surfaces") => Some(1),
                ("brep_capped_loft", "cap_trims") => Some(3),
                ("loft_match_surface", "sections" | "guides") => Some(1),
                ("loft_match_surface", "start_reference" | "end_reference") => Some(0),
                _ => None,
            };
            node[&key] = if let Some(depth) = depth {
                self.loft_references(value, depth)?
            } else {
                raw(value)?
            };
        }
        self.add(node)
    }

    fn lower_loft_embedding_limits(
        input: Option<String>,
        args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        if input.is_some() {
            return Err(crate::error("Embedding limits cannot take a geometry receiver"));
        }
        let mut limits = json!({
            "exactWork": 1000000,
            "trimPairs": 10000,
            "trimCells": 100000,
            "trimDomainCells": 1000000,
            "spans": 10000,
            "facePairs": 10000,
            "faceCells": 1000000,
            "faceDomainCells": 1000000,
            "faceCellsPerPair": 10000,
            "faceDomainCellsPerPair": 10000
        });
        let mut seen = Set::new();
        for (key, value) in args {
            let key = key.ok_or_else(|| crate::error("Embedding limits must be named"))?;
            if limits.get(&key).is_none() || !seen.insert(key.clone()) {
                return Err(crate::error("Invalid or repeated embedding limit"));
            }
            limits[&key] = raw(value)?;
        }
        Ok(V::Json(limits))
    }

    fn lower_loft_parameter_map(
        input: Option<String>,
        args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        if input.is_some() {
            return Err(crate::error("Parameter maps cannot take a geometry receiver"));
        }
        let mut fields = json!({});
        for (key, value) in args {
            let key = key.ok_or_else(|| crate::error("Parameter map options must be named"))?;
            let key = if key == "control_values" {
                "controlValues".to_string()
            } else {
                key
            };
            if !["domain", "range", "controlValues", "weights", "composition", "pieces"]
                .contains(&key.as_str())
                || fields.get(&key).is_some()
            {
                return Err(crate::error("Invalid or repeated parameter map option"));
            }
            fields[&key] = raw(value)?;
        }
        if fields.get("composition").is_some() || fields.get("pieces").is_some() {
            if fields.as_object().unwrap().len() != 1 {
                return Err(crate::error(
                    "Map composition/pieces cannot be mixed with scalar controls",
                ));
            }
            if let Some(items) = fields.get("pieces") {
                let mut pieces = Vec::new();
                for item in items
                    .as_array()
                    .ok_or_else(|| crate::error("Map pieces must be an array"))?
                {
                    let values = item["pieces"]
                        .as_array()
                        .ok_or_else(|| crate::error("Piece entries must be leaf parameter maps"))?;
                    pieces.extend(values.iter().cloned());
                }
                fields["pieces"] = json!(pieces);
            }
            return Ok(V::Json(fields));
        }
        if fields.as_object().unwrap().is_empty() {
            fields = json!({
                "domain": [0., 1.],
                "range": [0., 1.],
                "controlValues": [0., 1.],
                "weights": [1., 1.]
            });
        }
        for key in ["domain", "range", "controlValues", "weights"] {
            if fields.get(key).is_none() {
                return Err(crate::error(format!("Parameter map requires {key}")));
            }
        }
        Ok(V::Json(json!({"pieces":[fields]})))
    }

    fn lower_section_loft_surface(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let mut node = json!({"op":name});
        node["parameters"] = raw(take_named(
            &mut args,
            "parameters",
            "Loft requires named parameters",
        )?)?;
        if matches!(name, "guided_loft_surface" | "auto_guided_loft_surface") {
            let guides_val =
                take_named(&mut args, "guides", "Guided loft requires named guides")?;
            let V::Array(values) = guides_val else {
                return Err(crate::error("Guides must be curve references"));
            };
            let mut ids = Vec::with_capacity(values.len());
            for value in values {
                ids.push(self.geometry(value)?);
            }
            node["guides"] = json!(ids);
            let field = if name == "auto_guided_loft_surface" {
                "budget"
            } else {
                "guide_parameters"
            };
            node[field] = raw(take_named(
                &mut args,
                field,
                &format!("Guided loft requires named {field}"),
            )?)?;
        }
        if name == "auto_guided_loft_surface" {
            if let Some(val) = take_optional(&mut args, "parameter_tolerance") {
                node["parameter_tolerance"] = raw(val)?;
            }
        }
        for key in [
            "construction",
            "error_budget",
            "max_cells",
            "max_map_evaluations",
            "section_mappings",
        ] {
            if let Some(val) = take_optional(&mut args, key) {
                let value = raw(val)?;
                node[key] = if key == "construction" {
                    let mode = value["text"]
                        .as_str()
                        .ok_or_else(|| crate::error("Loft construction must be a string"))?;
                    if !["cartesian", "homogeneous"].contains(&mode) {
                        return Err(crate::error("Invalid loft construction"));
                    }
                    json!(mode)
                } else {
                    value
                };
            }
        }
        for key in ["start_tangents", "end_tangents"] {
            if let Some(val) = take_optional(&mut args, key) {
                node[key] = raw(val)?;
            } else if name == "control_tangent_loft_surface" {
                return Err(crate::error(format!("Loft requires named {key}")));
            }
        }
        if node.get("start_tangents").is_some() != node.get("end_tangents").is_some() {
            return Err(crate::error("Loft requires both tangent fields"));
        }
        let inputs =
            self.collect_positional_geometry(input, args, "Loft sections must be positional")?;
        if !(2..=86).contains(&inputs.len()) {
            return Err(crate::error("Loft requires 2..86 sections"));
        }
        node["inputs"] = json!(inputs);
        self.add(node)
    }

    fn lower_clamped_loft_surface(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let mut node = json!({"op":name});
        for key in ["parameters", "start_tangent", "end_tangent"] {
            node[key] = raw(take_named(
                &mut args,
                key,
                &format!("clamped_loft_surface requires named {key}"),
            )?)?;
        }
        let inputs = self.collect_positional_geometry(
            input,
            args,
            "Clamped loft sections must be positional",
        )?;
        if !(2..=11).contains(&inputs.len()) {
            return Err(crate::error("Clamped loft requires 2..11 sections"));
        }
        node["inputs"] = json!(inputs);
        self.add(node)
    }

    fn lower_closed_loft_surface(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let parameters = raw(take_named(
            &mut args,
            "parameters",
            "natural_loft_surface requires named parameters",
        )?)?;
        let inputs = self.collect_positional_geometry(
            input,
            args,
            "natural loft sections must be positional",
        )?;
        if !(4..=11).contains(&inputs.len()) {
            return Err(crate::error("Cubic loft section count is outside its budget"));
        }
        self.add(json!({"op":name,"inputs":inputs,"parameters":parameters}))
    }
}
