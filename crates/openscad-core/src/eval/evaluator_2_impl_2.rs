use super::*;

impl<'a> Evaluator<'a> {
    pub(super) fn eval_node(&self, node: &'a CallNode, parent: &Ctx<'a>) -> EvalResult<Vec<ShapeDescriptor>> {
        if self.record_geometry
            && ((self.is_stable()
                && !matches!(
                    node.name.as_str(),
                    "cube"
                        | "rotate"
                        | "multmatrix"
                        | "translate"
                        | "scale"
                        | "mirror"
                        | "cylinder"
                        | "sphere"
                        | "square"
                        | "circle"
                        | "polygon"
                        | "linear_extrude"
                        | "rotate_extrude"
                        | "projection"
                        | "resize"
                        | "polyhedron"
                        | "offset"
                        | "hull"
                        | "minkowski"
                        | "intersection"
                        | "difference"
                        | "union"
                        | "for"
                        | "if"
                        | "let"
                        | "children"
                        | "assert"
                        | "echo"
                        | "group"
                        | "render"
                )
                && !self.modules.contains_key(&node.name))
                || (!self.is_stable()
                    && !matches!(
                        node.name.as_str(),
                        "cube"
                            | "rotate"
                            | "scale"
                            | "mirror"
                            | "multmatrix"
                            | "sphere"
                            | "cylinder"
                            | "translate"
                            | "union"
                            | "intersection"
                            | "difference"
                            | "for"
                            | "if"
                            | "let"
                            | "children"
                            | "assert"
                            | "echo"
                            | "group"
                            | "render"
                    )
                    && !self.modules.contains_key(&node.name)))
        {
            return Err(self.error(
                node.p,
                format!(
                    "Native geometry recording does not yet support {}() for this profile",
                    node.name
                ),
            ));
        }
        if self.is_stable() && !parent.viewport_root_locked && has_modifier(node, "root") {
            let locked = Ctx {
                viewport_root_locked: true,
                viewport_root_owner: Some(node as *const CallNode),
                ..parent.clone()
            };
            let shapes = self.eval_node(node, &locked)?;
            if viewport_root_activates(node, &shapes) {
                return Err(EvalFailure::ViewportRoot(shapes));
            }
            return Ok(shapes);
        }
        if parent.depth >= MAX_EVAL_DEPTH {
            return Err(self.error(
                node.p,
                format!("Evaluation exceeds {MAX_EVAL_DEPTH} nested calls"),
            ));
        }
        let ctx = Ctx {
            depth: parent.depth + 1,
            ..parent.clone()
        };

        match node.name.as_str() {
            "assign" | "assert" | "cube" | "sphere" | "cylinder" | "polyhedron" | "import" | "import_stl" | "import_off" | "import_dxf" | "surface" | "text" | "square" | "circle" | "polygon" => self.eval_node_cases_1(node, ctx),
            "translate" | "rotate" | "scale" | "resize" | "mirror" | "multmatrix" | "color" | "union" | "difference" | "intersection" | "minkowski" | "hull" => self.eval_node_cases_2(node, ctx),
            "linear_extrude" | "rotate_extrude" | "dxf_linear_extrude" | "dxf_rotate_extrude" | "projection" | "offset" | "group" | "render" | "if" | "let" | "for" | "intersection_for" | "echo" | "children" => self.eval_node_cases_3(node, ctx),
            "child" => self.eval_node_cases_4(node, ctx),
            _ => {
                if self.is_stable() {
                    let declaration = ctx
                        .stable_scope
                        .as_ref()
                        .and_then(|scope| scope.module_declaration(&node.name));
                    if let Some((module, scope)) = declaration {
                        return self.eval_user_module(node, module, &ctx, Some(scope));
                    }
                } else if let Some(module) = self.modules.get(&node.name).copied() {
                    return self.eval_user_module(node, module, &ctx, None);
                }
                Err(self.error(
                    node.p,
                    format!("Unsupported geometry operation {}()", node.name),
                ))
            }
        }
    }

}
