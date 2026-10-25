use super::*;
impl<'a> Evaluator<'a> {
    pub(super) fn eval_node_cases_1(&self, node: &'a CallNode, ctx: Ctx<'a>) -> EvalResult<Vec<ShapeDescriptor>> {
        match node.name.as_str() {
            "assign" => {
                self.require_stable(node)?;
                let mut env = ctx.env.borrow().clone();
                for argument in &node.call_arguments {
                    // Historical assign() ignores positional arguments without
                    // evaluating them, and each named RHS sees the caller.
                    let Some(name) = &argument.name else {
                        continue;
                    };
                    let value = self.eval_expression(&argument.value, &ctx, 0)?;
                    env.insert(name.clone(), value);
                }
                let overlay = self.stable_overlay_context(&ctx, env);
                let shapes = self.eval_nodes(&node.children, &overlay, false)?;
                self.boolean_shapes(shapes, "union", node.p, "assign")
            }
            "assert" => self.eval_assert_statement(node, &ctx),
            "cube" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(node, &ctx, &["size", "center"], &[])?;
                    self.warn_ignored_primitive_children(node);
                    if !self.record_geometry {
                        return Ok(vec![descriptor("cube", 3)]);
                    }
                    use crate::primitive_plan::{Size, box_plan};
                    let size = match values.get("size") {
                        None | Some(Value::Undef) => Size::Missing,
                        Some(Value::Number(value)) => Size::Scalar(*value),
                        Some(Value::Vector(values)) => {
                            Size::Vector(values.iter().map(Value::as_number).collect())
                        }
                        _ => Size::Invalid,
                    };
                    let plan = box_plan(
                        &size,
                        true,
                        matches!(values.get("center"), Some(Value::Bool(true))),
                    );
                    if plan.defaulted {
                        self.warn("cube size was not a scalar or exact 3-component numeric vector; unit size is used".to_string());
                    }
                    if plan.empty {
                        return self.emit_geometry(
                            "cube",
                            geometry_ops::solid_program::Node::Empty,
                            node.p,
                        );
                    }
                    return self.emit_geometry(
                        "cube",
                        geometry_ops::solid_program::Node::Cube {
                            size: [plan.dimensions[0], plan.dimensions[1], plan.dimensions[2]],
                            center: plan.center,
                        },
                        node.p,
                    );
                }
                let raw = self.arg(node, "size", 0, Value::Number(1.0), &ctx)?;
                let size = match &raw {
                    Value::Vector(_) => self.vector_value(&raw, node.p, "cube size")?,
                    _ => vec![self.finite_number(&raw, node.p, "cube size")?],
                };
                let dimensions = [
                    size.first().copied().unwrap_or(1.0),
                    size.get(1)
                        .copied()
                        .or(size.first().copied())
                        .unwrap_or(1.0),
                    size.get(2)
                        .copied()
                        .or(size.first().copied())
                        .unwrap_or(1.0),
                ];
                if dimensions.iter().any(|v| *v <= 0.0) {
                    return Err(self.error(node.p, "Cube dimensions must be positive"));
                }
                let center = truthy(&self.arg(node, "center", 1, Value::Bool(false), &ctx)?);
                self.emit_geometry(
                    "cube",
                    geometry_ops::solid_program::Node::Cube {
                        size: dimensions,
                        center,
                    },
                    node.p,
                )
            }
            "sphere" => {
                if self.is_stable() {
                    let values =
                        self.bind_stable_module(node, &ctx, &["r"], &["d", "$fn", "$fa", "$fs"])?;
                    self.warn_ignored_primitive_children(node);
                    if !self.record_geometry {
                        return Ok(vec![descriptor("sphere", 3)]);
                    }
                    let radius = crate::primitive_plan::radius_pair(
                        values.get("r").and_then(Value::as_number),
                        values.get("d").and_then(Value::as_number),
                    );
                    if radius.shadowed {
                        self.warn("sphere uses d; the paired r value has no effect".to_string());
                    }
                    if crate::primitive_plan::radial_empty(radius.value) {
                        return self.emit_geometry(
                            "sphere",
                            geometry_ops::solid_program::Node::Empty,
                            node.p,
                        );
                    }
                    let segments = self.stable_fragment_count(&values, &ctx, radius.value)?;
                    return self.emit_geometry(
                        "sphere",
                        geometry_ops::solid_program::Node::Sphere {
                            radius: radius.value,
                            segments,
                        },
                        node.p,
                    );
                }
                let mut radius = self.arg(node, "r", 0, Value::Undef, &ctx)?;
                let diameter = self.arg(node, "d", -1, Value::Undef, &ctx)?;
                if radius.is_undef() {
                    radius = match &diameter {
                        Value::Undef => Value::Number(1.0),
                        other => Value::Number(
                            self.finite_number(other, node.p, "sphere diameter")? / 2.0,
                        ),
                    };
                }
                let r = self.finite_number(&radius, node.p, "sphere radius")?;
                if r <= 0.0 {
                    return Err(self.error(node.p, "Sphere radius must be positive"));
                }
                let segments = self.segments(node, &ctx, 32.0, 4.0, r)? as usize;
                self.emit_geometry(
                    "sphere",
                    geometry_ops::solid_program::Node::Sphere {
                        radius: r,
                        segments,
                    },
                    node.p,
                )
            }
            "cylinder" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(
                        node,
                        &ctx,
                        &["h", "r1", "r2", "center"],
                        &["r", "d", "d1", "d2", "$fn", "$fa", "$fs"],
                    )?;
                    self.warn_ignored_primitive_children(node);
                    if !self.record_geometry {
                        return Ok(vec![descriptor("cylinder", 3)]);
                    }
                    use crate::primitive_plan::{cylinder_plan, radius_pair};
                    let number = |name: &str| values.get(name).and_then(Value::as_number);
                    let common = radius_pair(number("r"), number("d"));
                    let low = radius_pair(number("r1"), number("d1"));
                    let high = radius_pair(number("r2"), number("d2"));
                    for (pair, r, d) in [(common, "r", "d"), (low, "r1", "d1"), (high, "r2", "d2")]
                    {
                        if pair.shadowed {
                            self.warn(format!(
                                "cylinder uses {d}; the paired {r} value has no effect"
                            ));
                        }
                    }
                    let plan = cylinder_plan(number("h"), common, low, high);
                    if plan.ambiguous {
                        self.warn(
                            "cylinder combines a shared radius with an end-specific radius"
                                .to_string(),
                        );
                    }
                    if plan.empty {
                        return self.emit_geometry(
                            "cylinder",
                            geometry_ops::solid_program::Node::Empty,
                            node.p,
                        );
                    }
                    let segments =
                        self.stable_fragment_count(&values, &ctx, plan.fragment_radius)?;
                    return self.emit_geometry(
                        "cylinder",
                        geometry_ops::solid_program::Node::Cylinder {
                            height: plan.height,
                            radii: [plan.radius1, plan.radius2],
                            segments,
                            center: matches!(values.get("center"), Some(Value::Bool(true))),
                        },
                        node.p,
                    );
                }
                let height = self.finite_number(
                    &self.arg(node, "h", 0, Value::Number(1.0), &ctx)?,
                    node.p,
                    "cylinder height",
                )?;
                if height <= 0.0 {
                    return Err(self.error(node.p, "Cylinder height must be positive"));
                }
                let mut low = self.arg(node, "r1", 1, Value::Undef, &ctx)?;
                let mut high = self.arg(node, "r2", 2, Value::Undef, &ctx)?;
                let radius = self.arg(node, "r", 1, Value::Undef, &ctx)?;
                let diameter = self.arg(node, "d", -1, Value::Undef, &ctx)?;
                let d1 = self.arg(node, "d1", -1, Value::Undef, &ctx)?;
                let d2 = self.arg(node, "d2", -1, Value::Undef, &ctx)?;
                if !d1.is_undef() {
                    low = Value::Number(self.finite_number(&d1, node.p, "d1")? / 2.0);
                }
                if !d2.is_undef() {
                    high = Value::Number(self.finite_number(&d2, node.p, "d2")? / 2.0);
                }
                if low.is_undef() && high.is_undef() {
                    let base = if !diameter.is_undef() {
                        self.finite_number(&diameter, node.p, "diameter")? / 2.0
                    } else if !radius.is_undef() {
                        self.finite_number(&radius, node.p, "radius")?
                    } else {
                        1.0
                    };
                    low = Value::Number(base);
                    high = Value::Number(base);
                }
                if low.is_undef() {
                    low = high.clone();
                }
                if high.is_undef() {
                    high = low.clone();
                }
                let r1 = self.finite_number(&low, node.p, "r1")?;
                let r2 = self.finite_number(&high, node.p, "r2")?;
                if r1 < 0.0 || r2 < 0.0 || (r1 == 0.0 && r2 == 0.0) {
                    return Err(self.error(
                        node.p,
                        "Cylinder radii must be non-negative and not both zero",
                    ));
                }
                let center = truthy(&self.arg(node, "center", 3, Value::Bool(false), &ctx)?);
                let segments = self.segments(node, &ctx, 32.0, 3.0, r1.max(r2))? as usize;
                self.emit_geometry(
                    "cylinder",
                    geometry_ops::solid_program::Node::Cylinder {
                        height,
                        radii: [r1, r2],
                        segments,
                        center,
                    },
                    node.p,
                )
            }
            "polyhedron" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(
                        node,
                        &ctx,
                        &["points", "faces", "convexity"],
                        &["triangles"],
                    )?;
                    if self.record_geometry { return self.record_stable_polyhedron(&values,node); }
                    self.warn_ignored_primitive_children(node);
                    return Ok(vec![descriptor("polyhedron", 3)]);
                }
                let points = self.arg(node, "points", 0, Value::vector(Vec::new()), &ctx)?;
                let faces = {
                    let fallback =
                        self.arg(node, "triangles", 1, Value::vector(Vec::new()), &ctx)?;
                    self.arg(node, "faces", 1, fallback, &ctx)?
                };
                if points.as_vector().is_none() || faces.as_vector().is_none() {
                    return Err(self.error(node.p, "polyhedron points and faces must be vectors"));
                }
                for point in points.as_vector().unwrap().clone().iter() {
                    let vector = self.vector_value(point, node.p, "polyhedron point")?;
                    if vector.len() < 3 {
                        return Err(
                            self.error(node.p, "Each polyhedron point needs three coordinates")
                        );
                    }
                }
                let points_len = points.as_vector().unwrap().len();
                for face in faces.as_vector().unwrap().clone().iter() {
                    let polygon = self.vector_value(face, node.p, "polyhedron face")?;
                    if polygon.len() < 3 {
                        return Err(self
                            .error(node.p, "Each polyhedron face needs at least three vertices"));
                    }
                    for index in &polygon {
                        let index = index.trunc();
                        if index < 0.0 || index >= points_len as f64 {
                            return Err(
                                self.error(node.p, "Polyhedron face index is out of bounds")
                            );
                        }
                    }
                }
                Ok(vec![descriptor("polyhedron", 3)])
            }
            "import" => {
                self.require_stable(node)?;
                Err(self.import_project_required(node, "import"))
            }
            "import_stl" | "import_off" | "import_dxf" => {
                self.require_stable(node)?;
                self.compatibility_deprecation(node, "import()");
                Err(self.import_project_required(node, &node.name.clone()))
            }
            "surface" => {
                self.require_stable(node)?;
                Err(EvalFailure::Error(ParseError::coded(
                    node.p,
                    "surface() requires an OpenSCAD project so its file is resolved inside the bounded project VFS.",
                    "E_SURFACE_PROJECT_REQUIRED",
                ).with_end(node.end)))
            }
            "text" => {
                self.require_stable(node)?;
                Err(EvalFailure::Error(ParseError::coded(
                    node.p,
                    "text() requires an OpenSCAD project so fonts are resolved inside the bounded project VFS.",
                    "E_TEXT_PROJECT_REQUIRED",
                ).with_end(node.end)))
            }
            "square" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(node, &ctx, &["size", "center"], &[])?;
                    self.warn_ignored_primitive_children(node);
                    if self.record_geometry {
                        use crate::primitive_plan::{Size, box_plan};
                        let size = match values.get("size") {
                            None | Some(Value::Undef) => Size::Missing,
                            Some(Value::Number(value)) => Size::Scalar(*value),
                            Some(Value::Vector(values)) => {
                                Size::Vector(values.iter().map(Value::as_number).collect())
                            }
                            _ => Size::Invalid,
                        };
                        let plan = box_plan(
                            &size,
                            false,
                            matches!(values.get("center"), Some(Value::Bool(true))),
                        );
                        if plan.defaulted {
                            self.warn("square size was not a scalar or exact 2-component numeric vector; unit size is used".to_string());
                        }
                        return self.emit_profile(
                            "square",
                            if plan.empty {
                                geometry_ops::profile_program::Node::Empty
                            } else {
                                geometry_ops::profile_program::Node::Rectangle {
                                    size: [plan.dimensions[0], plan.dimensions[1]],
                                    center: plan.center,
                                }
                            },
                            node.p,
                        );
                    }
                    return Ok(vec![descriptor("square", 2)]);
                }
                let raw = self.arg(node, "size", 0, Value::Number(1.0), &ctx)?;
                let size = match &raw {
                    Value::Vector(_) => self.vector_value(&raw, node.p, "square size")?,
                    _ => vec![self.finite_number(&raw, node.p, "square size")?],
                };
                let dimensions = [
                    size.first().copied().unwrap_or(1.0),
                    size.get(1)
                        .copied()
                        .or(size.first().copied())
                        .unwrap_or(1.0),
                ];
                if dimensions.iter().any(|v| *v <= 0.0) {
                    return Err(self.error(node.p, "Square dimensions must be positive"));
                }
                self.arg(node, "center", 1, Value::Bool(false), &ctx)?;
                Ok(vec![descriptor("square", 2)])
            }
            "circle" => {
                if self.is_stable() {
                    let values =
                        self.bind_stable_module(node, &ctx, &["r"], &["d", "$fn", "$fa", "$fs"])?;
                    self.warn_ignored_primitive_children(node);
                    if self.record_geometry {
                        let radius = crate::primitive_plan::radius_pair(
                            values.get("r").and_then(Value::as_number),
                            values.get("d").and_then(Value::as_number),
                        );
                        if radius.shadowed {
                            self.warn(
                                "circle uses d; the paired r value has no effect".to_string(),
                            );
                        }
                        if crate::primitive_plan::radial_empty(radius.value) {
                            self.warn("circle parameters describe an empty object".to_string());
                            return self.emit_profile(
                                "circle",
                                geometry_ops::profile_program::Node::Empty,
                                node.p,
                            );
                        }
                        let segments = self.stable_fragment_count(&values, &ctx, radius.value)?;
                        return self.emit_profile(
                            "circle",
                            geometry_ops::profile_program::Node::Circle {
                                radius: radius.value,
                                segments,
                            },
                            node.p,
                        );
                    }
                    return Ok(vec![descriptor("circle", 2)]);
                }
                let mut radius = self.arg(node, "r", 0, Value::Undef, &ctx)?;
                let diameter = self.arg(node, "d", -1, Value::Undef, &ctx)?;
                if radius.is_undef() {
                    radius = match &diameter {
                        Value::Undef => Value::Number(1.0),
                        other => Value::Number(
                            self.finite_number(other, node.p, "circle diameter")? / 2.0,
                        ),
                    };
                }
                let r = self.finite_number(&radius, node.p, "circle radius")?;
                if r <= 0.0 {
                    return Err(self.error(node.p, "Circle radius must be positive"));
                }
                self.segments(node, &ctx, 48.0, 3.0, r)?;
                Ok(vec![descriptor("circle", 2)])
            }
            "polygon" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(
                        node,
                        &ctx,
                        &["points", "paths", "convexity"],
                        &[],
                    )?;
                    self.warn_ignored_primitive_children(node);
                    if self.record_geometry {
                        return self.record_stable_polygon(&values, node.p);
                    }
                    return Ok(vec![descriptor("polygon", 2)]);
                }
                let points_value = self.arg(node, "points", 0, Value::vector(Vec::new()), &ctx)?;
                let Some(points) = points_value.as_vector().cloned() else {
                    return Err(self.error(node.p, "polygon points must be a vector"));
                };
                for point in points.iter() {
                    let vector = self.vector_value(point, node.p, "polygon point")?;
                    if vector.len() < 2 {
                        return Err(self.error(node.p, "Each polygon point needs two coordinates"));
                    }
                }
                let paths_value = self.arg(node, "paths", 1, Value::Undef, &ctx)?;
                if !paths_value.is_undef() {
                    let Some(paths) = paths_value.as_vector().cloned() else {
                        return Err(self.error(node.p, "polygon paths must be a vector"));
                    };
                    for path in paths.iter() {
                        let indices = self.vector_value(path, node.p, "polygon path")?;
                        for index in indices {
                            let point = points.get(index.trunc() as usize);
                            if point.is_none() || index.trunc() < 0.0 {
                                return Err(
                                    self.error(node.p, "Polygon path index is out of bounds")
                                );
                            }
                        }
                    }
                }
                Ok(vec![descriptor("polygon", 2)])
            }
            _ => unreachable!("selected native node family"),
        }
    }
}
