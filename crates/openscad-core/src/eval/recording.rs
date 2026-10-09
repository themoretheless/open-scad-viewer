use super::*;

impl<'a> Evaluator<'a> {
pub(super) fn record_stable_polyhedron(&self, values:&HashMap<String,Value<'a>>, node:&'a CallNode) -> EvalResult<Vec<ShapeDescriptor>> {
        let faces_name=if values.get("faces").is_some_and(|v| !v.is_undef()) {"faces"}
            else if values.get("triangles").is_some_and(|v| !v.is_undef()) {"triangles"} else {"faces"};
        if faces_name=="triangles" {self.warn("polyhedron triangles is a legacy alias; faces is the stable spelling");}
        let plan=crate::indexed_primitive::expand_faces(&self.indexed_rows(values,"points"),&self.indexed_rows(values,faces_name),usize::MAX);
        use crate::indexed_primitive::FaceEvent;
        for event in &plan.events {
            match event {
                FaceEvent::OutOfBounds {..}=>self.warn("polyhedron skipped a face entry whose point index is outside the point vector"),
                FaceEvent::InvalidPoint {..}=>self.warn("polyhedron stopped after a referenced point failed exact vec2/vec3 conversion"),
                _=>return Err(self.error(node.p,"Native polyhedron expansion exceeded its input budget")),
            }
        }
        self.warn_ignored_primitive_children(node);
        if plan.empty {return self.emit_geometry("polyhedron",geometry_ops::solid_program::Node::Empty,node.p);}
        let packed=geometry_ops::polygon_mesh::reversed_fans(&plan.polygons).map_err(|error|self.error(node.p,error.message))?;
        if !packed.usable {
            self.warn("polyhedron() produced no usable finite faces");
            return self.emit_geometry("polyhedron",geometry_ops::solid_program::Node::Empty,node.p);
        }
        self.emit_geometry("polyhedron",geometry_ops::solid_program::Node::Mesh {
            mesh:geometry_ops::Triangles {positions:packed.vertices,indices:packed.indices.into_iter().map(|index|index as usize).collect()},
            empty_on_failure:true,
        },node.p)
    }
    pub(super) fn record_stable_polygon(
        &self,
        values: &HashMap<String, Value<'a>>,
        p: usize,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let points = self.indexed_rows(values,"points");
        let paths = self.indexed_rows(values,"paths");
        let plan = crate::indexed_primitive::expand_polygon(
            &points,
            &paths,
            paths.len(),
            usize::MAX,
            usize::MAX,
        );
        use crate::indexed_primitive::FaceEvent;
        for event in &plan.events {
            match event {
                FaceEvent::OutOfBounds { .. } => self.warn(
                    "polygon skipped a path entry whose point index is outside the point vector"
                        .to_string(),
                ),
                FaceEvent::InvalidPoint { .. } => self.warn(
                    "polygon produced no outlines because a point failed exact vec2 conversion"
                        .to_string(),
                ),
                _ => {
                    return Err(self.error(p, "Native polygon expansion exceeded its input budget"));
                }
            }
        }
        let finite = plan
            .outlines
            .iter()
            .flatten()
            .flatten()
            .all(|v| v.is_finite());
        if !plan.empty && !finite {
            self.warn("polygon() produced no usable finite outlines".to_string());
        }
        self.emit_profile(
            "polygon",
            if plan.empty || !finite {
                geometry_ops::profile_program::Node::Empty
            } else {
                geometry_ops::profile_program::Node::EvenOddRings(plan.outlines)
            },
            p,
        )
    }

    pub(super) fn record_stable_extrusion(
        &self,
        sections: Vec<ShapeDescriptor>,
        values: &HashMap<String, Value<'a>>,
        mut fragments: [f64; 3],
        p: usize,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        use crate::primitive_plan::Size;
        let scale = match values.get("scale") {
            None | Some(Value::Undef) => Size::Missing,
            Some(Value::Number(value)) => Size::Scalar(*value),
            Some(Value::Vector(values)) => {
                Size::Vector(values.iter().map(Value::as_number).collect())
            }
            _ => Size::Invalid,
        };
        let number = |name: &str| values.get(name).and_then(Value::as_number);
        let plan = crate::extrusion_plan::parameters(
            number("height"),
            values.get("height").is_some_and(|v| !v.is_undef()),
            &scale,
            number("twist"),
            matches!(values.get("center"), Some(Value::Bool(true))),
            number("slices"),
        );
        if plan.height_defaulted {
            self.warn("Invalid linear_extrude height was replaced with 100".to_string());
        }
        if plan.scale_defaulted {
            self.warn("Invalid linear_extrude scale was replaced with [1, 1]".to_string());
        }
        if plan.empty {
            return self.emit_geometry(
                "linear_extrude",
                geometry_ops::solid_program::Node::Empty,
                p,
            );
        }
        for value in &mut fragments[1..] {
            if value.is_finite() {
                *value = value.max(0.01);
            }
        }
        let root = sections
            .first()
            .and_then(|shape| shape.profile)
            .ok_or_else(|| self.error(p, "Unsupported native extrusion profile"))?;
        let profile =
            geometry_ops::profile_program::Program::from_roots(&self.profiles.borrow(), &[root])
                .map_err(|error| self.error(p, &error.to_string()))?;
        self.emit_geometry(
            "linear_extrude",
            geometry_ops::solid_program::Node::ExtrudeProfile {
                profile,
                slice_policy: Some(geometry_ops::extrude_slices::Policy {
                    explicit: plan.explicit_slices,
                    fragments,
                    maximum: 512,
                }),
                height: plan.height,
                slices: 0,
                twist: plan.twist,
                scale: plan.scale,
                center: plan.center,
            },
            p,
        )
    }

    pub(super) fn stable_fragment_specials(
        &self,
        values: &HashMap<String, Value<'a>>,
        ctx: &Ctx<'a>,
    ) -> EvalResult<[f64; 3]> {
        let mut special = [0., 12., 2.];
        for (index, name) in ["$fn", "$fa", "$fs"].iter().enumerate() {
            if let Some(value) = values.get(*name) {
                special[index] = value.as_number().unwrap_or(0.);
            } else {
                let resolved = self.resolve_stable_variable(name, ctx)?;
                if resolved.found {
                    special[index] = resolved.value.as_number().unwrap_or(0.);
                }
            }
        }
        Ok(special)
    }

    pub(super) fn stable_fragment_count(
        &self,
        values: &HashMap<String, Value<'a>>,
        ctx: &Ctx<'a>,
        radius: f64,
    ) -> EvalResult<usize> {
        let special = self.stable_fragment_specials(values, ctx)?;
        let maximum = if self.quality == Quality::Preview {
            48.
        } else {
            256.
        };
        let resolution =
            crate::fragments::resolve(radius, special[0], special[1], special[2], maximum);
        for warning in &resolution.warnings {
            self.warn(warning.message(maximum as usize));
        }
        if resolution.reduced {
            self.reduced.set(true);
        }
        Ok(resolution.fragments as usize)
    }

    pub(super) fn segments(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
        fallback: f64,
        minimum: f64,
        _radius: f64,
    ) -> EvalResult<f64> {
        if self.is_stable() {
            return Ok(0.0); // fragment resolution belongs to the geometry stage
        }
        let local = self.arg(node, "$fn", -1, Value::Undef, ctx)?;
        let global = ctx.env.borrow().get("$fn").cloned().unwrap_or(Value::Undef);
        let raw = if local.is_undef() || local.as_number() == Some(0.0) {
            global
        } else {
            local
        };
        let requested = if raw.is_undef() || raw.as_number() == Some(0.0) {
            None
        } else {
            Some(self.finite_number(&raw, node.p, "$fn")?)
        };
        let selection = crate::fragments::legacy(
            requested,
            fallback,
            minimum,
            self.quality == Quality::Preview,
        );
        if selection.clamped {
            self.warn(format!(
                "$fn={} was clamped to {} for {} rendering",
                js_number_to_string(selection.before_cap),
                js_number_to_string(selection.maximum),
                if self.quality == Quality::Preview {
                    "preview"
                } else {
                    "full"
                }
            ));
        }
        if selection.reduced {
            self.reduced.set(true);
        }
        Ok(selection.segments)
    }

    pub(super) fn parse_legacy_color(&self, value: &Value, p: usize) -> EvalResult<()> {
        match value {
            Value::Vector(_) => {
                self.vector_value(value, p, "color")?;
                Ok(())
            }
            Value::Str(name) => {
                let lower = name.to_lowercase();
                if CSS_COLORS.contains(&lower.as_str()) {
                    return Ok(());
                }
                if is_hex_color(name) {
                    return Ok(());
                }
                Err(self.error(p, format!("Unknown color {name}")))
            }
            _ => Err(self.error(p, "color() expects a name or RGB(A) vector")),
        }
    }

    pub(super) fn stable_vector_geometry(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
        value: Option<&Value<'a>>,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        use crate::transform_plan::{VectorTransform, vector_transform};
        let kind = match node.name.as_str() {
            "translate" => VectorTransform::Translate,
            "scale" => VectorTransform::Scale,
            _ => VectorTransform::Mirror,
        };
        let vector = match value {
            Some(Value::Vector(v)) => Some(v.iter().map(Value::as_number).collect::<Vec<_>>()),
            _ => None,
        };
        let plan = vector_transform(kind, vector.as_deref(), value.and_then(Value::as_number));
        if !plan.valid {
            self.warn(
                match kind {
                    VectorTransform::Translate => {
                        "translate uses identity because v is not a finite exact vec2 or vec3"
                    }
                    VectorTransform::Scale => {
                        "scale uses identity because v is not a scalar or exact vec2/vec3"
                    }
                    VectorTransform::Mirror => {
                        "mirror uses its x-normal default because v is not an exact vec2 or vec3"
                    }
                }
                .to_string(),
            );
        }
        self.stable_matrix_geometry(node, ctx, plan.matrix)
    }

    pub(super) fn stable_matrix_geometry(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
        matrix: [f64; 16],
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let shapes = self.transform_stable_children(node, ctx)?;
        self.record_stable_matrix(shapes, matrix, &node.name, node.p)
    }

    pub(super) fn record_stable_matrix(
        &self,
        shapes: Vec<ShapeDescriptor>,
        matrix: [f64; 16],
        name: &str,
        p: usize,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let dimension = shapes.first().map_or(3, |shape| shape.dimension);
        let analysis = crate::transform_plan::analyze(matrix);
        let singular = if dimension == 2 {
            analysis.singular2d
        } else {
            analysis.singular3d
        };
        if singular || analysis.drops_children {
            if singular {
                self.warn(format!(
                    "{name}() produced empty geometry from a singular transform"
                ));
            } else {
                self.warn(format!(
                    "{name} produced a non-finite matrix, so its child geometry is removed"
                ));
            }
            return if dimension == 2 {
                self.emit_profile(name, geometry_ops::profile_program::Node::Empty, p)
            } else {
                self.emit_geometry(name, geometry_ops::solid_program::Node::Empty, p)
            };
        }
        if dimension == 2 {
            if !analysis.affine2d {
                return Err(self.error(p, "Unsupported native projective profile transform"));
            }
            let matrix = std::array::from_fn(|row| {
                std::array::from_fn(|column| analysis.matrix2d[column * 3 + row])
            });
            let shape = shapes
                .first()
                .ok_or_else(|| self.error(p, "Missing native profile input"))?;
            let input = shape
                .profile
                .ok_or_else(|| self.error(p, "Unsupported native profile input"))?;
            return self.emit_profile(
                name,
                geometry_ops::profile_program::Node::Transform { input, matrix },
                p,
            );
        }
        let matrix =
            std::array::from_fn(|row| std::array::from_fn(|column| matrix[column * 4 + row]));
        self.affine_geometry(shapes, matrix, p)
    }

    pub(super) fn transform_stable_children(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let shapes = self.eval_nodes(&node.children, ctx, true)?;
        self.boolean_shapes(shapes, "union", node.p, &node.name)
    }

    pub(super) fn boolean_shapes(
        &self,
        mut shapes: Vec<ShapeDescriptor>,
        operation: &str,
        p: usize,
        diagnostic_name: &str,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        if shapes.is_empty() {
            if self.record_geometry && self.is_stable() {
                return self.emit_geometry(
                    diagnostic_name,
                    geometry_ops::solid_program::Node::Empty,
                    p,
                );
            }
            return Ok(shapes);
        }
        let dimension = shapes[0].dimension;
        if shapes.iter().any(|s| s.dimension != dimension) {
            if !self.is_stable() {
                return Err(self.error(
                    p,
                    format!("{diagnostic_name}() cannot mix 2D and 3D children"),
                ));
            }
            self.warn(format!(
                "{diagnostic_name}() ignored child geometry with a different dimension"
            ));
            if operation == "intersection" {
                return Ok(Vec::new());
            }
            shapes.retain(|s| s.dimension == dimension);
        }
        if shapes.len() == 1 {
            return Ok(shapes);
        }
        if self.record_geometry {
            let operation_kind = match operation {
                "union" => geometry_ops::solid_program::Boolean::Union,
                "intersection" => geometry_ops::solid_program::Boolean::Intersection,
                "difference" => geometry_ops::solid_program::Boolean::Difference,
                _ => return Err(self.error(p, "Unsupported native boolean")),
            };
            if dimension == 2 {
                let inputs = shapes
                    .iter()
                    .map(|shape| {
                        shape
                            .profile
                            .ok_or_else(|| self.error(p, "Unsupported native profile input"))
                    })
                    .collect::<EvalResult<Vec<_>>>()?;
                return self.emit_profile(
                    operation,
                    geometry_ops::profile_program::Node::Boolean {
                        operation: operation_kind,
                        inputs,
                    },
                    p,
                );
            }
            let inputs = shapes
                .iter()
                .map(|s| {
                    s.geometry
                        .ok_or_else(|| self.error(p, "Unsupported native geometry input"))
                })
                .collect::<EvalResult<Vec<_>>>()?;
            return self.emit_geometry(
                operation,
                geometry_ops::solid_program::Node::Boolean {
                    operation: operation_kind,
                    inputs,
                },
                p,
            );
        }
        Ok(vec![descriptor(operation, dimension)])
    }

    pub(super) fn difference_children(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        if node.children.is_empty() {
            return Ok(Vec::new());
        }
        if self.is_stable() {
            let child_context = self.enter_stable_statement_scope(&node.children, ctx)?;
            let base = self.boolean_shapes(
                self.eval_prepared_nodes(&node.children[..1], &child_context)?,
                "union",
                node.p,
                "union",
            )?;
            let cutters = self.boolean_shapes(
                self.eval_prepared_nodes(&node.children[1..], &child_context)?,
                "union",
                node.p,
                "union",
            )?;
            if base.is_empty() || cutters.is_empty() {
                return Ok(base);
            }
            if base[0].dimension != cutters[0].dimension {
                self.warn("difference() ignored child geometry with a different dimension");
                return Ok(base);
            }
            if self.record_geometry {
                return self.boolean_shapes(
                    base.into_iter().chain(cutters).collect(),
                    "difference",
                    node.p,
                    "difference",
                );
            }
            return Ok(vec![descriptor("difference", base[0].dimension)]);
        }
        let base = self.boolean_shapes(
            self.eval_nodes(&node.children[..1], ctx, true)?,
            "union",
            node.p,
            "union",
        )?;
        let cutters = self.boolean_shapes(
            self.eval_nodes(&node.children[1..], ctx, true)?,
            "union",
            node.p,
            "union",
        )?;
        if base.is_empty() || cutters.is_empty() {
            return Ok(base);
        }
        if base[0].dimension != cutters[0].dimension {
            return Err(self.error(node.p, "difference() cannot mix 2D and 3D children"));
        }
        if self.record_geometry {
            let inputs = [&base[0], &cutters[0]]
                .iter()
                .map(|s| {
                    s.geometry
                        .ok_or_else(|| self.error(node.p, "Unsupported native geometry input"))
                })
                .collect::<EvalResult<Vec<_>>>()?;
            return self.emit_geometry(
                "difference",
                geometry_ops::solid_program::Node::Boolean {
                    operation: geometry_ops::solid_program::Boolean::Difference,
                    inputs,
                },
                node.p,
            );
        }
        Ok(vec![descriptor("difference", base[0].dimension)])
    }
}
