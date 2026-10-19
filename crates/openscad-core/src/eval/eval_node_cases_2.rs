use super::*;
impl<'a> Evaluator<'a> {
    pub(super) fn eval_node_cases_2(&self, node: &'a CallNode, ctx: Ctx<'a>) -> EvalResult<Vec<ShapeDescriptor>> {
        match node.name.as_str() {
            "translate" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(node, &ctx, &["v"], &[])?;
                    if self.record_geometry {
                        return self.stable_vector_geometry(node, &ctx, values.get("v"));
                    }
                    return self.transform_stable_children(node, &ctx);
                }
                let raw = self.arg(
                    node,
                    "v",
                    0,
                    Value::vector(vec![
                        Value::Number(0.0),
                        Value::Number(0.0),
                        Value::Number(0.0),
                    ]),
                    &ctx,
                )?;
                let vector = self.vector_value(&raw, node.p, "translate vector")?;
                let delta = std::array::from_fn(|i| vector.get(i).copied().unwrap_or(0.));
                let mut shapes = self.eval_nodes(&node.children, &ctx, true)?;
                if self.record_geometry {
                    for shape in &mut shapes {
                        let input = shape.geometry.ok_or_else(|| {
                            self.error(node.p, "Unsupported native geometry input")
                        })?;
                        shape.geometry = Some(self.store_geometry(
                            geometry_ops::solid_program::Node::Translate { input, delta },
                            node.p,
                        )?);
                    }
                }
                Ok(shapes)
            }
            "rotate" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(node, &ctx, &["a", "v"], &[])?;
                    if self.record_geometry {
                        use crate::transform_plan::{
                            euler_arguments, euler_matrix, scalar_rotation,
                        };
                        let axis = values.get("v").filter(|value| !value.is_undef());
                        let matrix = if let Some(Value::Vector(components)) = values.get("a") {
                            let numbers =
                                components.iter().map(Value::as_number).collect::<Vec<_>>();
                            let plan = euler_arguments(&numbers, components.len());
                            if !plan.valid {
                                self.warn("rotate retained its component-wise fallback matrix after a vector conversion problem".to_string());
                            } else if axis.is_some() {
                                self.warn("rotate ignores v when a is a vector".to_string());
                            }
                            euler_matrix(plan.angles)
                        } else {
                            let numbers = match axis {
                                Some(Value::Vector(components)) => Some(
                                    components.iter().map(Value::as_number).collect::<Vec<_>>(),
                                ),
                                _ => None,
                            };
                            let plan = scalar_rotation(
                                values.get("a").and_then(Value::as_number),
                                numbers.as_deref(),
                                axis.is_some(),
                            );
                            if !plan.valid {
                                self.warn("rotate replaced an invalid scalar angle or axis with its neutral/default value".to_string());
                            }
                            plan.matrix
                        };
                        return self.stable_matrix_geometry(node, &ctx, matrix);
                    }
                    return self.transform_stable_children(node, &ctx);
                }
                let angle = self.arg(node, "a", 0, Value::Number(0.0), &ctx)?;
                let axis = self.arg(node, "v", 1, Value::Undef, &ctx)?;
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                for shape in &shapes {
                    if shape.dimension == 2 {
                        match &angle {
                            Value::Vector(_) => {
                                let vector = self.vector_value(&angle, node.p, "rotation")?;
                                let _ = vector.get(2).copied().unwrap_or(0.0);
                            }
                            _ => {
                                self.finite_number(&angle, node.p, "rotation")?;
                            }
                        }
                    } else {
                        match &angle {
                            Value::Vector(_) => {
                                self.vector_value(&angle, node.p, "rotation")?;
                            }
                            _ => {
                                self.finite_number(&angle, node.p, "rotation")?;
                                if !axis.is_undef() {
                                    let vector =
                                        self.vector_value(&axis, node.p, "rotation axis")?;
                                    let length = f64::hypot(
                                        f64::hypot(
                                            vector.first().copied().unwrap_or(0.0),
                                            vector.get(1).copied().unwrap_or(0.0),
                                        ),
                                        vector.get(2).copied().unwrap_or(0.0),
                                    );
                                    if length == 0.0 {
                                        return Err(
                                            self.error(node.p, "Rotation axis cannot be zero")
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                if self.record_geometry && !shapes.is_empty() {
                    use geometry_ops::solid_program::affine;
                    let matrix = if matches!(angle, Value::Vector(_)) {
                        let v = self.vector_value(&angle, node.p, "rotation")?;
                        affine::euler_degrees(std::array::from_fn(|i| {
                            v.get(i).copied().unwrap_or(0.)
                        }))
                    } else {
                        let degrees = self.finite_number(&angle, node.p, "rotation")?;
                        if axis.is_undef() {
                            affine::euler_degrees([0., 0., degrees])
                        } else {
                            let v = self.vector_value(&axis, node.p, "rotation axis")?;
                            affine::axis_angle_degrees(
                                std::array::from_fn(|i| v.get(i).copied().unwrap_or(0.)),
                                degrees,
                            )
                            .ok_or_else(|| self.error(node.p, "Rotation axis cannot be zero"))?
                        }
                    };
                    return self.affine_geometry(shapes, matrix, node.p);
                }
                Ok(shapes)
            }
            "scale" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(node, &ctx, &["v"], &[])?;
                    if self.record_geometry {
                        return self.stable_vector_geometry(node, &ctx, values.get("v"));
                    }
                    return self.transform_stable_children(node, &ctx);
                }
                let raw = self.arg(
                    node,
                    "v",
                    0,
                    Value::vector(vec![
                        Value::Number(1.0),
                        Value::Number(1.0),
                        Value::Number(1.0),
                    ]),
                    &ctx,
                )?;
                let values = match &raw {
                    Value::Vector(_) => self.vector_value(&raw, node.p, "scale vector")?,
                    _ => vec![self.finite_number(&raw, node.p, "scale")?],
                };
                let sx = values.first().copied().unwrap_or(1.0);
                let sy = values.get(1).copied().unwrap_or(sx);
                let sz = values.get(2).copied().unwrap_or(sx);
                if [sx, sy, sz].contains(&0.0) {
                    return Err(self.error(node.p, "Scale values cannot be zero"));
                }
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                self.affine_geometry(
                    shapes,
                    geometry_ops::solid_program::affine::scaling([sx, sy, sz]),
                    node.p,
                )
            }
            "resize" => {
                self.require_stable(node)?;
                let values = self.bind_stable_module(node, &ctx, &["newsize", "auto", "convexity"], &[])?;
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                if shapes.is_empty() {
                    return Ok(shapes);
                }
                let dimension = shapes[0].dimension;
                if shapes.iter().any(|s| s.dimension != dimension) {
                    return Err(self.error(node.p, "resize() cannot mix 2D and 3D children"));
                }
                if self.record_geometry {
                    let raw_targets = match values.get("newsize") {
                        None => Some(Vec::new()),
                        Some(Value::Vector(values)) => Some(values.iter().map(Value::as_number).collect::<Vec<_>>()),
                        _ => None,
                    };
                    let invalid_newsize = raw_targets.is_none();
                    let target = |axis:usize| raw_targets.as_ref().and_then(|values| values.get(axis).copied().flatten()).filter(|value| value.is_finite() && *value>0.);
                    let automatic = |axis:usize| match values.get("auto") {
                        Some(Value::Bool(true)) => true,
                        Some(Value::Vector(values)) => matches!(values.get(axis),Some(Value::Bool(true))),
                        _ => false,
                    };
                    let union=self.boolean_shapes(shapes,"union",node.p,"resize")?;
                    if dimension==2 {
                        let input=union[0].profile.ok_or_else(|| self.error(node.p,"Unsupported native resize profile"))?;
                        return self.emit_profile("resize",geometry_ops::profile_program::Node::Resize {
                            input,invalid_newsize,targets:std::array::from_fn(target),automatic:std::array::from_fn(automatic),
                        },node.p);
                    }
                    let input=union[0].geometry.ok_or_else(|| self.error(node.p,"Unsupported native resize solid"))?;
                    return self.emit_geometry("resize",geometry_ops::solid_program::Node::Resize {
                        input,invalid_newsize,targets:std::array::from_fn(target),automatic:std::array::from_fn(automatic),
                    },node.p);
                }
                Ok(shapes)
            }
            "mirror" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(node, &ctx, &["v"], &[])?;
                    if self.record_geometry {
                        return self.stable_vector_geometry(node, &ctx, values.get("v"));
                    }
                    return self.transform_stable_children(node, &ctx);
                }
                let raw = self.arg(
                    node,
                    "v",
                    0,
                    Value::vector(vec![
                        Value::Number(1.0),
                        Value::Number(0.0),
                        Value::Number(0.0),
                    ]),
                    &ctx,
                )?;
                let v = self.vector_value(&raw, node.p, "mirror normal")?;
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                if self.record_geometry && !shapes.is_empty() {
                    let matrix =
                        geometry_ops::solid_program::affine::reflection(std::array::from_fn(|i| {
                            v.get(i).copied().unwrap_or(0.)
                        }))
                        .ok_or_else(|| self.error(node.p, "Mirror normal cannot be zero"))?;
                    return self.affine_geometry(shapes, matrix, node.p);
                }
                Ok(shapes)
            }
            "multmatrix" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(node, &ctx, &["m"], &[])?;
                    if self.record_geometry {
                        let rows = match values.get("m") {
                            Some(Value::Vector(rows)) => Some(
                                rows.iter()
                                    .take(4)
                                    .map(|row| match row {
                                        Value::Vector(cells) => cells
                                            .iter()
                                            .take(4)
                                            .map(Value::as_number)
                                            .collect::<Vec<_>>(),
                                        _ => Vec::new(),
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            _ => None,
                        };
                        let plan = crate::transform_plan::authored_matrix(rows.as_deref());
                        return self.stable_matrix_geometry(node, &ctx, plan.matrix);
                    }
                    return self.transform_stable_children(node, &ctx);
                }
                let value = self.arg(node, "m", 0, Value::Undef, &ctx)?;
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                let has_2d = shapes.iter().any(|s| s.dimension == 2);
                let has_3d = shapes.iter().any(|s| s.dimension == 3);
                let mut matrix = geometry_ops::solid_program::affine::IDENTITY;
                if has_3d || has_2d {
                    let Some(rows) = value.as_vector().cloned() else {
                        return Err(self.error(node.p, "multmatrix requires a 4x4 matrix"));
                    };
                    if rows.len() < 3 {
                        return Err(self.error(node.p, "multmatrix requires a 4x4 matrix"));
                    }
                    for (index, row) in rows.iter().enumerate() {
                        let row = self.vector_value(row, node.p, "matrix row")?;
                        if row.len() < 4 {
                            return Err(self.error(node.p, "multmatrix requires a 4x4 matrix"));
                        }
                        if index < 3 {
                            matrix[index].copy_from_slice(&row[..4]);
                        }
                    }
                }
                self.affine_geometry(shapes, matrix, node.p)
            }
            "color" => {
                let color_value = self.arg(
                    node,
                    "c",
                    0,
                    if self.is_stable() {
                        Value::Undef
                    } else {
                        Value::vector(vec![
                            Value::Number(0.5),
                            Value::Number(0.5),
                            Value::Number(0.5),
                        ])
                    },
                    &ctx,
                )?;
                let alpha = self.arg(node, "alpha", 1, Value::Undef, &ctx)?;
                if !self.is_stable() {
                    self.parse_legacy_color(&color_value, node.p)?;
                    if !alpha.is_undef() {
                        self.finite_number(&alpha, node.p, "color alpha")?;
                    }
                }
                self.eval_nodes(&node.children, &ctx, true)
            }
            "union" => {
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                self.boolean_shapes(shapes, "union", node.p, "union")
            }
            "difference" => self.difference_children(node, &ctx),
            "intersection" => {
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                self.boolean_shapes(shapes, "intersection", node.p, "intersection")
            }
            "minkowski" => {
                self.require_stable(node)?;
                let mut shapes = self.eval_nodes(&node.children, &ctx, true)?;
                if shapes.is_empty() {
                    return Ok(shapes);
                }
                let dimension = shapes[0].dimension;
                if shapes.iter().any(|s| s.dimension != dimension) {
                    return Err(self.error(node.p, "minkowski() cannot mix 2D and 3D children"));
                }
                if shapes.len() == 1 {
                    return Ok(shapes);
                }
                if self.record_geometry {
                    if dimension == 2 {
                        let inputs = shapes
                            .iter()
                            .map(|shape| {
                                shape.profile.ok_or_else(|| {
                                    self.error(node.p, "Unsupported native Minkowski profile")
                                })
                            })
                            .collect::<EvalResult<Vec<_>>>()?;
                        return self.emit_profile(
                            "minkowski",
                            geometry_ops::profile_program::Node::Minkowski { inputs },
                            node.p,
                        );
                    }
                    let inputs = shapes
                        .iter()
                        .map(|shape| {
                            shape.geometry.ok_or_else(|| {
                                self.error(node.p, "Unsupported native Minkowski geometry")
                            })
                        })
                        .collect::<EvalResult<Vec<_>>>()?;
                    return self.emit_geometry(
                        "minkowski",
                        geometry_ops::solid_program::Node::Minkowski { inputs },
                        node.p,
                    );
                }
                shapes.truncate(1);
                Ok(vec![descriptor("minkowski", dimension)])
            }
            "hull" => {
                let mut shapes = self.eval_nodes(&node.children, &ctx, true)?;
                if shapes.is_empty() {
                    return Ok(shapes);
                }
                let dimension = shapes[0].dimension;
                if shapes.iter().any(|s| s.dimension != dimension) {
                    if !self.is_stable() {
                        return Err(self.error(node.p, "hull() cannot mix 2D and 3D children"));
                    }
                    self.warn("hull() ignored child geometry with a different dimension");
                    shapes.retain(|s| s.dimension == dimension);
                }
                if self.record_geometry {
                    if dimension == 2 {
                        let inputs = shapes
                            .iter()
                            .map(|shape| {
                                shape.profile.ok_or_else(|| {
                                    self.error(node.p, "Unsupported native hull profile")
                                })
                            })
                            .collect::<EvalResult<Vec<_>>>()?;
                        return self.emit_profile(
                            "hull",
                            geometry_ops::profile_program::Node::Hull { inputs },
                            node.p,
                        );
                    }
                    let inputs = shapes
                        .iter()
                        .map(|shape| {
                            shape.geometry.ok_or_else(|| {
                                self.error(node.p, "Unsupported native hull geometry")
                            })
                        })
                        .collect::<EvalResult<Vec<_>>>()?;
                    return self.emit_geometry(
                        "hull",
                        geometry_ops::solid_program::Node::Hull { inputs },
                        node.p,
                    );
                }
                Ok(vec![descriptor("hull", dimension)])
            }
            _ => unreachable!("selected native node family"),
        }
    }
}
