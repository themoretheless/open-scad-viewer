use super::*;

impl<'a> Evaluator<'a> {
    /// Full program evaluation (the TS `parseInternal` evaluation portion).
    pub fn evaluate(&self, ast: &'a [Statement]) -> EvalResult<Evaluation> {
        let env: HashMap<String, Value<'a>> = if self.is_stable() {
            let animation_time = self.animation_time;
            let mut env = HashMap::new();
            env.insert("$fn".to_string(), Value::Number(0.0));
            env.insert("$fa".to_string(), Value::Number(12.0));
            env.insert("$fs".to_string(), Value::Number(2.0));
            env.insert("$t".to_string(), Value::Number(animation_time));
            env.insert(
                "$preview".to_string(),
                Value::Bool(self.quality == Quality::Preview),
            );
            env.insert(
                "$vpt".to_string(),
                Value::vector(vec![
                    Value::Number(0.0),
                    Value::Number(0.0),
                    Value::Number(0.0),
                ]),
            );
            env.insert(
                "$vpr".to_string(),
                Value::vector(vec![
                    Value::Number(55.0),
                    Value::Number(0.0),
                    Value::Number(25.0),
                ]),
            );
            env.insert("$vpd".to_string(), Value::Number(140.0));
            env.insert("$vpf".to_string(), Value::Number(22.5));
            env
        } else {
            HashMap::from([
                ("$fn".to_string(), Value::Number(0.0)),
                ("$fa".to_string(), Value::Number(12.0)),
                ("$fs".to_string(), Value::Number(2.0)),
            ])
        };
        let mut ctx = Ctx {
            env: env_of(env),
            stable_scope: None,
            scope_visible_before: usize::MAX,
            call_children: None,
            function_stack: Rc::new(Vec::new()),
            module_stack: Rc::new(Vec::new()),
            depth: 0,
            viewport_root_locked: false,
            viewport_root_owner: None,
        };
        if self.is_stable() {
            ctx = self.enter_stable_statement_scope(ast, &ctx)?;
        }
        let mut shapes = match self.eval_top_level(ast, &ctx) {
            Ok(shapes) => shapes,
            Err(EvalFailure::ViewportRoot(shapes)) => shapes,
            Err(failure) => return Err(failure),
        };
        if self.is_stable() {
            shapes = self.boolean_shapes(shapes, "union", 0, "union")?;
        }
        let sections = shapes.iter().filter(|s| s.dimension == 2).count();
        if sections > 0 {
            self.warn(format!(
                "{sections} top-level 2D object(s) are not displayed; wrap them in linear_extrude() or rotate_extrude()"
            ));
        }
        shapes.retain(|s| s.dimension == 3);
        let geometry = if self.record_geometry {
            let roots = shapes
                .iter()
                .map(|s| {
                    s.geometry
                        .ok_or_else(|| self.error(0, "Unsupported native geometry root"))
                })
                .collect::<EvalResult<Vec<_>>>()?;
            Some(geometry_ops::solid_program::Program {
                nodes: self.geometry.borrow().clone(),
                roots,
            })
        } else {
            None
        };
        Ok(Evaluation {
            geometry,
            shapes,
            warnings: self.warnings.borrow().clone(),
            reduced: self.reduced.get(),
        })
    }
}
