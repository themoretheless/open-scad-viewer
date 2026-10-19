use super::*;
impl<'a> Evaluator<'a> {
    pub(super) fn eval_node_cases_3(&self, node: &'a CallNode, ctx: Ctx<'a>) -> EvalResult<Vec<ShapeDescriptor>> {
        match node.name.as_str() {
            "linear_extrude" => {
                let evaluated = if self.is_stable() {
                    Some(self.bind_stable_module(
                        node,
                        &ctx,
                        &["height", "center", "convexity", "twist", "slices", "scale"],
                        &["$fn", "$fa", "$fs"],
                    )?)
                } else {
                    None
                };
                let sections = self.eval_nodes(&node.children, &ctx, true)?;
                if self.record_geometry && sections.is_empty() {
                    return self.emit_geometry(
                        "linear_extrude",
                        geometry_ops::solid_program::Node::Empty,
                        node.p,
                    );
                }
                let sections = self.boolean_shapes(sections, "union", node.p, "union")?;
                if sections.is_empty() {
                    return Ok(Vec::new());
                }
                if sections[0].dimension != 2 {
                    return Err(self.error(node.p, "linear_extrude() requires 2D children"));
                }
                if evaluated.is_none() {
                    let height = self.finite_number(
                        &self.arg(node, "height", 0, Value::Number(1.0), &ctx)?,
                        node.p,
                        "extrusion height",
                    )?;
                    if height <= 0.0 {
                        return Err(self.error(node.p, "linear_extrude() height must be positive"));
                    }
                    self.finite_number(
                        &self.arg(node, "twist", -1, Value::Number(0.0), &ctx)?,
                        node.p,
                        "extrusion twist",
                    )?;
                    let slices = self.finite_number(
                        &self.arg(node, "slices", -1, Value::Number(0.0), &ctx)?,
                        node.p,
                        "extrusion slices",
                    )?;
                    let _ = slices.trunc().clamp(0.0, 512.0);
                    let raw_scale = self.arg(
                        node,
                        "scale",
                        -1,
                        Value::vector(vec![Value::Number(1.0), Value::Number(1.0)]),
                        &ctx,
                    )?;
                    match &raw_scale {
                        Value::Vector(_) => {
                            self.vector_value(&raw_scale, node.p, "extrusion scale")?;
                        }
                        _ => {
                            self.finite_number(&raw_scale, node.p, "extrusion scale")?;
                        }
                    }
                    self.arg(node, "center", -1, Value::Bool(false), &ctx)?;
                }
                if self.record_geometry {
                    if let Some(values) = evaluated {
                        let fragments = self.stable_fragment_specials(&values, &ctx)?;
                        return self.record_stable_extrusion(sections, &values, fragments, node.p);
                    }
                }
                Ok(vec![descriptor("linear_extrude", 3)])
            }
            "rotate_extrude" => {
                let evaluated = if self.is_stable() {
                    Some(self.bind_stable_module(
                        node,
                        &ctx,
                        &["angle", "convexity"],
                        &["$fn", "$fa", "$fs"],
                    )?)
                } else {
                    None
                };
                let sections = self.eval_nodes(&node.children, &ctx, true)?;
                let sections = self.boolean_shapes(sections, "union", node.p, "union")?;
                if sections.is_empty() {
                    return Ok(Vec::new());
                }
                if sections[0].dimension != 2 {
                    return Err(self.error(node.p, "rotate_extrude() requires 2D children"));
                }
                if evaluated.is_none() {
                    let _angle = self.finite_number(
                        &self.arg(node, "angle", -1, Value::Number(360.0), &ctx)?,
                        node.p,
                        "revolve angle",
                    )?;
                }
                if self.record_geometry {
                    if let Some(values) = evaluated {
                        let angle = values.get("angle").and_then(Value::as_number);
                        let plan = crate::extrusion_plan::revolution_parameters(
                            angle,
                            values.contains_key("angle"),
                            1.,
                            1.,
                        );
                        if plan.angle_defaulted {
                            self.warn(
                                "Invalid rotate_extrude angle was replaced with 360".to_string(),
                            );
                        }
                        if plan.empty {
                            return self.emit_geometry(
                                "rotate_extrude",
                                geometry_ops::solid_program::Node::Empty,
                                node.p,
                            );
                        }
                        let root = sections[0].profile.ok_or_else(|| {
                            self.error(node.p, "Unsupported native revolution profile")
                        })?;
                        let profile = geometry_ops::profile_program::Program::from_roots(
                            &self.profiles.borrow(),
                            &[root],
                        )
                        .map_err(|error| self.error(node.p, &error.to_string()))?;
                        let fragments = self.stable_fragment_specials(&values, &ctx)?;
                        return self.emit_geometry(
                            "rotate_extrude",
                            geometry_ops::solid_program::Node::RevolveProfile {
                                profile,
                                angle: plan.angle,
                                segments: 0,
                                fragment_policy: Some(geometry_ops::fragment_resolution::Policy {
                                    fragments,
                                    maximum: if self.quality == Quality::Preview {
                                        48
                                    } else {
                                        256
                                    },
                                }),
                            },
                            node.p,
                        );
                    }
                }
                Ok(vec![descriptor("rotate_extrude", 3)])
            }
            "dxf_linear_extrude" => {
                self.require_stable(node)?;
                self.compatibility_deprecation(node, "linear_extrude()");
                let values = self.bind_stable_module(
                    node,
                    &ctx,
                    &[
                        "file", "layer", "height", "origin", "scale", "center", "twist", "slices",
                    ],
                    &["convexity", "$fn", "$fa", "$fs"],
                )?;
                let file = values.get("file").cloned().unwrap_or(Value::Undef);
                if let Value::Str(file) = &file
                    && !file.is_empty()
                {
                    return Err(self.import_project_required(node, "dxf_linear_extrude"));
                }
                let sections = self.eval_nodes(&node.children, &ctx, true)?;
                let sections =
                    self.boolean_shapes(sections, "union", node.p, "dxf_linear_extrude")?;
                if sections.is_empty() {
                    return Ok(Vec::new());
                }
                if sections[0].dimension != 2 {
                    return Err(self.error(node.p, "dxf_linear_extrude() requires 2D children"));
                }
                Ok(vec![descriptor("dxf_linear_extrude", 3)])
            }
            "dxf_rotate_extrude" => {
                self.require_stable(node)?;
                self.compatibility_deprecation(node, "rotate_extrude()");
                let values = self.bind_stable_module(
                    node,
                    &ctx,
                    &["file", "layer", "origin", "scale"],
                    &["convexity", "angle", "$fn", "$fa", "$fs"],
                )?;
                let file = values.get("file").cloned().unwrap_or(Value::Undef);
                let file_text = self.compatibility_string(&file);
                if !file_text.is_empty() {
                    return Err(self.import_project_required(node, "dxf_rotate_extrude"));
                }
                let sections = self.eval_nodes(&node.children, &ctx, true)?;
                let sections =
                    self.boolean_shapes(sections, "union", node.p, "dxf_rotate_extrude")?;
                if sections.is_empty() {
                    return Ok(Vec::new());
                }
                if sections[0].dimension != 2 {
                    return Err(self.error(node.p, "dxf_rotate_extrude() requires 2D children"));
                }
                Ok(vec![descriptor("dxf_rotate_extrude", 3)])
            }
            "projection" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(node, &ctx, &["cut", "convexity"], &[])?;
                    let children = self.eval_nodes(&node.children, &ctx, true)?;
                    let solids: Vec<_> = children.iter().filter(|s| s.dimension == 3).collect();
                    if solids.len() != children.len() {
                        self.warn("projection() ignored non-3D child geometry");
                    }
                    if solids.is_empty() {
                        return Ok(Vec::new());
                    }
                    if self.record_geometry {
                        let shapes = solids.into_iter().cloned().collect();
                        let union = self.boolean_shapes(shapes, "union", node.p, "projection")?;
                        let input = union[0].geometry.ok_or_else(|| self.error(node.p,"Unsupported native projection input"))?;
                        let solid = geometry_ops::solid_program::Program::from_roots(&self.geometry.borrow(), &[input])
                            .map_err(|error| self.error(node.p,error.message))?;
                        return self.emit_profile("projection", geometry_ops::profile_program::Node::Projection {
                            solid: Box::new(solid), cut: matches!(values.get("cut"),Some(Value::Bool(true))),
                        }, node.p);
                    }
                    return Ok(vec![descriptor("projection", 2)]);
                }
                let cut = self.arg(node, "cut", 0, Value::Bool(false), &ctx)?;
                let _ = cut;
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                let mut output = Vec::new();
                for shape in shapes {
                    if shape.dimension != 3 {
                        return Err(self.error(node.p, "projection() requires 3D children"));
                    }
                    output.push(descriptor("projection", 2));
                }
                Ok(output)
            }
            "offset" => {
                if self.is_stable() {
                    let mut values = HashMap::new();
                    let r_expression = call_args(node)
                        .iter()
                        .find(|(key, _)| key == "r")
                        .or_else(|| call_args(node).iter().find(|(key, _)| key == "_0"));
                    for (name, expression) in [
                        ("r", r_expression.map(|(_, expression)| expression)),
                        (
                            "delta",
                            call_args(node)
                                .iter()
                                .find(|(key, _)| key == "delta")
                                .map(|(_, expression)| expression),
                        ),
                        (
                            "chamfer",
                            call_args(node)
                                .iter()
                                .find(|(key, _)| key == "chamfer")
                                .map(|(_, expression)| expression),
                        ),
                    ]
                    .into_iter()
                    .filter_map(|(name, expression)| {
                        expression.map(|expression| (name, expression))
                    }) {
                        values.insert(name.to_string(), self.eval_expression(expression, &ctx, 0)?);
                    }
                    let plan = crate::offset_plan::resolve(
                        values.get("r").and_then(Value::as_number),
                        values.get("delta").and_then(Value::as_number),
                        matches!(values.get("chamfer"), Some(Value::Bool(true))),
                    );
                    let segments =
                        if self.record_geometry && plan.join == crate::offset_plan::Join::Round {
                            for name in ["$fn", "$fa", "$fs"] {
                                if let Some((_, expression)) =
                                    call_args(node).iter().find(|(key, _)| key == name)
                                {
                                    values.insert(
                                        name.to_string(),
                                        self.eval_expression(expression, &ctx, 0)?,
                                    );
                                }
                            }
                            self.stable_fragment_count(&values, &ctx, plan.distance.abs())?
                        } else {
                            8
                        };
                    let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                    let mut output = Vec::new();
                    for shape in shapes {
                        if shape.dimension != 2 {
                            return Err(self.error(node.p, "offset() requires 2D children"));
                        }
                        if self.record_geometry {
                            if !plan.distance.is_finite() {
                                return Err(self.error(node.p, "Invalid offset"));
                            }
                            let input = shape.profile.ok_or_else(|| {
                                self.error(node.p, "Unsupported native offset profile")
                            })?;
                            output.extend(self.emit_profile(
                                "offset",
                                geometry_ops::profile_program::Node::Offset {
                                    input,
                                    distance: plan.distance,
                                    join: plan.join,
                                    segments,
                                },
                                node.p,
                            )?);
                        } else {
                            output.push(descriptor("offset", 2));
                        }
                    }
                    return Ok(output);
                }
                let distance = {
                    let delta = self.arg(node, "delta", 0, Value::Number(1.0), &ctx)?;
                    let raw = self.arg(node, "r", 0, delta, &ctx)?;
                    self.finite_number(&raw, node.p, "offset distance")?
                };
                let _ = distance;
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                let mut output = Vec::new();
                for shape in shapes {
                    if shape.dimension != 2 {
                        return Err(self.error(node.p, "offset() requires 2D children"));
                    }
                    output.push(descriptor("offset", 2));
                }
                Ok(output)
            }
            "group" => {
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                if self.is_stable() {
                    return self.boolean_shapes(shapes, "union", node.p, "group");
                }
                Ok(shapes)
            }
            "render" => {
                if self.is_stable() {
                    self.bind_stable_module(node, &ctx, &["convexity"], &[])?;
                    let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                    return self.boolean_shapes(shapes, "union", node.p, "render");
                }
                self.eval_nodes(&node.children, &ctx, true)
            }
            "if" => {
                let condition = self.arg(node, "_0", 0, Value::Bool(false), &ctx)?;
                let branch = if truthy(&condition) {
                    &node.children
                } else {
                    &node.alternative
                };
                self.eval_nodes(branch, &ctx, true)
            }
            "let" => {
                if self.is_stable() {
                    let env = self.evaluate_sequential_bindings(&node.call_arguments, &ctx, 0)?;
                    let overlay = self.stable_overlay_context(&ctx, env);
                    return self.eval_nodes(&node.children, &overlay, false);
                }
                let mut env = ctx.env.borrow().clone();
                for (name, expression) in call_args(node) {
                    if name.starts_with('_') {
                        continue;
                    }
                    let value = self.eval_expression(expression, &ctx, 0)?;
                    env.insert(name.clone(), value);
                }
                self.eval_nodes(&node.children, &ctx.with_env(env), false)
            }
            "for" => self.eval_for(node, &ctx, "for"),
            "intersection_for" => {
                self.require_stable(node)?;
                let shapes = self.eval_for(node, &ctx, "intersection_for")?;
                self.boolean_shapes(shapes, "intersection", node.p, "intersection_for")
            }
            "echo" => {
                self.require_stable(node)?;
                let mut values = Vec::new();
                for (name, expression) in call_args(node) {
                    let value = format_value(&self.eval_expression(expression, &ctx, 0)?);
                    let kind = call_arg_kinds(node)
                        .iter()
                        .find(|(key, _)| key == name)
                        .map(|(_, kind)| *kind);
                    values.push(if kind == Some("named") {
                        format!("{name} = {value}")
                    } else {
                        value
                    });
                }
                self.warnings.borrow_mut().push(format!(
                    "ECHO:{}",
                    if values.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", values.join(", "))
                    }
                ));
                self.eval_nodes(&node.children, &ctx, true)
            }
            "children" => {
                let (all, child_context) = self.passed_call_children(&ctx);
                if self.is_stable() {
                    let selector = call_args(node).iter().find(|(key, _)| key == "_0");
                    let selected = match selector {
                        None => None,
                        Some((_, expression)) => {
                            let value = self.eval_expression(expression, &ctx, 0)?;
                            Some(self.resolve_children_selection(Some(&value), all.len())?)
                        }
                    };
                    match selected {
                        None => self.eval_nodes(all, &child_context, true),
                        Some(indices) => {
                            let mut output = Vec::new();
                            for index in indices {
                                if let Some(statement) = all.get(index) {
                                    output.extend(self.eval_nodes(
                                        std::slice::from_ref(statement),
                                        &child_context,
                                        true,
                                    )?);
                                }
                            }
                            Ok(output)
                        }
                    }
                } else {
                    let index = self.arg(node, "_0", 0, Value::Undef, &ctx)?;
                    if index.is_undef() {
                        return self.eval_nodes(all, &child_context, true);
                    }
                    let index = self
                        .finite_number(&index, node.p, "children index")?
                        .trunc();
                    if index < 0.0 {
                        return Ok(Vec::new());
                    }
                    match all.get(index as usize) {
                        None => Ok(Vec::new()),
                        Some(statement) => {
                            self.eval_nodes(std::slice::from_ref(statement), &child_context, true)
                        }
                    }
                }
            }
            _ => unreachable!("selected native node family"),
        }
    }
}
