use super::*;

impl<'a> Evaluator<'a> {
    pub(super) fn require_stable(&self, node: &CallNode) -> EvalResult<()> {
        if !self.is_stable() {
            return Err(self.error(
                node.p,
                format!("Unsupported geometry operation {}()", node.name),
            ));
        }
        Ok(())
    }

    pub(super) fn import_project_required(&self, node: &CallNode, display_name: &str) -> EvalFailure {
        EvalFailure::Error(
            ParseError::coded(
                node.p,
                format!(
                    "{display_name}() requires an OpenSCAD project so its file is resolved inside the bounded project VFS."
                ),
                "E_IMPORT_PROJECT_REQUIRED",
            )
            .with_end(node.end),
        )
    }

    pub(super) fn compatibility_deprecation(&self, node: &CallNode, replacement: &str) {
        if node.name == "child" {
            self.warn("child() will be removed in future releases. Use children() instead.");
        } else {
            self.warn(format!(
                "The {}() module will be removed in future releases. Use {replacement} instead.",
                node.name
            ));
        }
    }

    pub(super) fn warn_ignored_primitive_children(&self, node: &CallNode) {
        if !node.children.is_empty() {
            self.warn(format!("{}() ignores child geometry", node.name));
        }
    }

    /// `evaluateStableBuiltinModuleArguments`: bind a stable built-in module
    /// without re-evaluating argument expressions; every supplied expression is
    /// evaluated exactly once, including ignored arguments whose echo/assert
    /// effects remain observable.
    pub(super) fn bind_stable_module(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
        positional_names: &[&str],
        named_only_names: &[&str],
    ) -> EvalResult<HashMap<String, Value<'a>>> {
        let allowed: HashSet<&str> = positional_names
            .iter()
            .chain(named_only_names)
            .copied()
            .collect();
        let mut resolved: HashMap<String, usize> = HashMap::new();
        for (index, argument) in node.call_arguments.iter().enumerate() {
            let name = match &argument.name {
                Some(name) => Some(name.clone()),
                None => positional_names
                    .iter()
                    .find(|parameter| !resolved.contains_key(**parameter))
                    .map(|s| s.to_string()),
            };
            let Some(name) = name else {
                self.warn(format!(
                    "Ignoring excess positional argument to {}()",
                    node.name
                ));
                continue;
            };
            if !allowed.contains(name.as_str()) {
                self.warn(format!(
                    "Ignoring unknown argument {name} to {}()",
                    node.name
                ));
                continue;
            }
            if resolved.contains_key(&name) {
                self.warn(format!(
                    "Argument {name} was specified more than once for {}()",
                    node.name
                ));
            }
            resolved.insert(name, index);
        }
        let mut evaluated: Vec<Value<'a>> = Vec::with_capacity(node.call_arguments.len());
        for argument in &node.call_arguments {
            evaluated.push(self.eval_expression(&argument.value, ctx, 0)?);
        }
        Ok(resolved
            .into_iter()
            .map(|(name, index)| (name, evaluated[index].clone()))
            .collect())
    }

    pub(super) fn indexed_rows(&self, values: &HashMap<String,Value<'a>>, name:&str) -> Vec<Vec<Option<f64>>> {
        match values.get(name) {
            Some(Value::Vector(rows)) => rows.iter().map(|row| match row {
                Value::Vector(values)=>values.iter().map(Value::as_number).collect(),
                _=>Vec::new(),
            }).collect(),
            _=>Vec::new(),
        }
    }


    pub(super) fn eval_for(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
        diagnostic_name: &str,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        if self.is_stable() {
            let bindings = &node.call_arguments;
            let mut output = Vec::new();
            self.for_visit_stable(
                node,
                bindings,
                0,
                ctx,
                &mut |iteration_ctx| {
                    let shapes = self.eval_nodes(&node.children, iteration_ctx, false)?;
                    output.extend(shapes);
                    if output.len() > MAX_SHAPES {
                        return Err(self.error(
                            node.p,
                            format!("Model exceeds the {} object limit", locale(MAX_SHAPES)),
                        ));
                    }
                    Ok(())
                },
                diagnostic_name,
            )?;
            return Ok(output);
        }
        let entries: Vec<&(String, Expr)> = call_args(node)
            .iter()
            .filter(|(name, _)| !name.starts_with('_'))
            .collect();
        if entries.len() != 1 {
            return Err(self.error(
                node.p,
                format!("{diagnostic_name}() currently requires one named iterator"),
            ));
        }
        let (name, expression) = entries[0];
        let values = self.eval_expression(expression, ctx, 0)?;
        let Value::Vector(values) = values else {
            return Err(self.error(
                node.p,
                format!("{diagnostic_name}() iterator must be a vector or range"),
            ));
        };
        let mut output = Vec::new();
        for value in values.iter() {
            self.poll()?;
            self.bump_ops(node.p)?;
            let mut env = ctx.env.borrow().clone();
            env.insert(name.clone(), value.clone());
            let iteration_ctx = ctx.with_env(env);
            output.extend(self.eval_nodes(&node.children, &iteration_ctx, false)?);
            if output.len() > MAX_SHAPES {
                return Err(self.error(
                    node.p,
                    format!("Model exceeds the {} object limit", locale(MAX_SHAPES)),
                ));
            }
        }
        Ok(output)
    }

    pub(super) fn for_visit_stable(
        &self,
        node: &'a CallNode,
        bindings: &'a [ExpressionArgument],
        binding_index: usize,
        ctx: &Ctx<'a>,
        visit: &mut dyn FnMut(&Ctx<'a>) -> EvalResult<()>,
        diagnostic_name: &str,
    ) -> EvalResult<()> {
        if binding_index >= bindings.len() {
            return visit(ctx);
        }
        let binding = &bindings[binding_index];
        let value = self.eval_expression(&binding.value, ctx, 0)?;
        let values = self.stable_iterable(&value, ctx, binding.p)?;
        let Some(name) = &binding.name else {
            self.warn(format!(
                "Ignoring {diagnostic_name}() iterator without variable name"
            ));
            return Ok(());
        };
        for value in values {
            self.poll()?;
            self.bump_ops(node.p)?;
            let mut env = ctx.env.borrow().clone();
            env.insert(name.clone(), value);
            let overlay = self.stable_overlay_context(ctx, env);
            self.for_visit_stable(
                node,
                bindings,
                binding_index + 1,
                &overlay,
                visit,
                diagnostic_name,
            )?;
        }
        Ok(())
    }

    pub(super) fn eval_assert_statement(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        if self.is_stable() {
            let resolved = self.resolve_stable_expression_arguments(
                &node.call_arguments,
                &["condition", "message"],
                ctx,
            );
            let condition_argument = resolved.get("condition");
            let message_argument = resolved.get("message");
            let condition = match condition_argument {
                Some(argument) => self.eval_expression(&argument.value, ctx, 0)?,
                None => Value::Undef,
            };
            let message = match message_argument {
                Some(argument) => self.eval_expression(&argument.value, ctx, 0)?,
                None => Value::Undef,
            };
            if !truthy(&condition) {
                let condition_text = match condition_argument {
                    Some(argument) => {
                        compact_diagnostic_text(&self.slice_units(argument.p, argument.end), 240)
                    }
                    None => "undef".to_string(),
                };
                let detail = if message_argument.is_some() {
                    format!(
                        ": {}",
                        compact_diagnostic_text(&format_value(&message), 240)
                    )
                } else {
                    String::new()
                };
                return Err(self.error(
                    node.p,
                    format!("Assertion '{condition_text}' failed{detail}"),
                ));
            }
            return self.eval_nodes(&node.children, ctx, true);
        }
        let bound = self.bind_assert_arguments(node)?;
        let condition = self.eval_expression(bound.condition, ctx, 0)?;
        // OpenSCAD binds call arguments before executing the module: a supplied
        // message is evaluated even when the assertion passes.
        let message = match bound.message {
            Some(expression) => self.eval_expression(expression, ctx, 0)?,
            None => Value::Undef,
        };
        if !truthy(&condition) {
            let detail = if bound.message.is_some() {
                format!(
                    ": {}",
                    compact_diagnostic_text(&value_to_string(&message), 240)
                )
            } else {
                String::new()
            };
            return Err(self.error(
                node.p,
                format!("Assertion '{}' failed{detail}", bound.condition_text),
            ));
        }
        self.eval_nodes(&node.children, ctx, true)
    }

    pub(super) fn bind_assert_arguments(&self, node: &'a CallNode) -> EvalResult<BoundAssert<'a>> {
        let args = call_args(node);
        let kinds = call_arg_kinds(node);
        let unknown = args.iter().find(|(key, _)| {
            let named =
                kinds.iter().find(|(k, _)| k == key).map(|(_, kind)| *kind) == Some("named");
            if named {
                key != "condition" && key != "message"
            } else {
                key != "_0" && key != "_1"
            }
        });
        if let Some((unknown, _)) = unknown {
            return Err(self.error(
                node.p,
                format!("assert() does not accept argument {unknown}"),
            ));
        }
        let has = |key: &str| args.iter().any(|(k, _)| k == key);
        if has("_0") && has("condition") {
            return Err(self.error(node.p, "assert() condition was provided more than once"));
        }
        if has("_1") && has("message") {
            return Err(self.error(node.p, "assert() message was provided more than once"));
        }
        let condition_key = if has("condition") {
            "condition"
        } else if has("_0") {
            "_0"
        } else {
            return Err(self.error(node.p, "assert() requires a condition"));
        };
        let message_key = if has("message") {
            Some("message")
        } else if has("_1") {
            Some("_1")
        } else {
            None
        };
        let span = node
            .arg_spans
            .iter()
            .find(|(k, _)| k == condition_key)
            .map(|(_, span)| *span);
        let raw_condition = match span {
            Some((start, end)) => self.slice_units(start, end),
            None => "condition".to_string(),
        };
        let condition = &args.iter().find(|(k, _)| k == condition_key).unwrap().1;
        let message = message_key.map(|key| &args.iter().find(|(k, _)| k == key).unwrap().1);
        Ok(BoundAssert {
            condition,
            condition_text: compact_diagnostic_text(&raw_condition, 240),
            message,
        })
    }

    pub(super) fn eval_user_module(
        &self,
        call: &'a CallNode,
        module: &'a ModuleNode,
        ctx: &Ctx<'a>,
        definition_scope: Option<Rc<StableScope<'a>>>,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        if self.is_stable() {
            let Some(definition_scope) = definition_scope else {
                return Err(self.error(
                    call.p,
                    format!(
                        "Stable module {}() is missing its lexical scope",
                        module.name
                    ),
                ));
            };
            let module_stack = ctx.push_module(&module.name);
            let mut definition_env = definition_scope.env.borrow().clone();
            overlay_dynamic_variables(&mut definition_env, &ctx.env.borrow());
            definition_env.insert(
                "$children".to_string(),
                Value::Number(call.children.len() as f64),
            );
            definition_env.insert(
                "$parent_modules".to_string(),
                Value::Number(module_stack.len() as f64),
            );
            let definition_ctx = Ctx {
                env: env_of(definition_env.clone()),
                stable_scope: Some(definition_scope.clone()),
                scope_visible_before: usize::MAX,
                module_stack,
                ..ctx.clone()
            };
            let args = &call.call_arguments;
            let parameter_names: Vec<&str> =
                module.params.iter().map(|p| p.name.as_str()).collect();
            let resolved = self.resolve_stable_expression_arguments(args, &parameter_names, ctx);
            let mut caller_values: Vec<Value<'a>> = Vec::with_capacity(args.len());
            for argument in args {
                caller_values.push(self.eval_expression(&argument.value, ctx, 0)?);
            }
            let caller_value_of = |argument: &ExpressionArgument| -> Value<'a> {
                let index = args.iter().position(|a| std::ptr::eq(a, argument)).unwrap();
                caller_values[index].clone()
            };
            let mut env = definition_env.clone();
            for parameter in &module.params {
                let value = match resolved.get(&parameter.name) {
                    Some(supplied) => caller_value_of(supplied),
                    None => match &parameter.default_value {
                        Some(default) => self.eval_expression(
                            default,
                            &definition_ctx.with_env(definition_env.clone()),
                            0,
                        )?,
                        None => Value::Undef,
                    },
                };
                env.insert(parameter.name.clone(), value);
            }
            let module_ctx = Ctx {
                env: env_of(env),
                call_children: Some(CallChildren::Passed(Rc::new(PassedCallChildren {
                    statements: &call.children,
                    env: env_of(ctx.env.borrow().clone()),
                    scope: ctx.stable_scope.clone().or(Some(definition_scope)),
                    continuation: ctx.call_children.clone(),
                }))),
                ..definition_ctx
            };
            let shapes = self.eval_nodes(&module.children, &module_ctx, false)?;
            return self.boolean_shapes(shapes, "union", call.p, &module.name);
        }
        let mut env = ctx.env.borrow().clone();
        let mut positional: Vec<&(String, Expr)> = call_args(call)
            .iter()
            .filter(|(name, _)| name.starts_with('_'))
            .collect();
        positional.sort_by_key(|(name, _)| name[1..].parse::<usize>().unwrap_or(0));
        for (index, parameter) in module.params.iter().enumerate() {
            let expression = call_args(call)
                .iter()
                .find(|(name, _)| name == &parameter.name)
                .map(|(_, expression)| expression)
                .or_else(|| positional.get(index).map(|entry| &entry.1))
                .or(parameter.default_value.as_ref());
            let value = match expression {
                Some(expression) => {
                    let param_ctx = ctx.with_env(env.clone());
                    self.eval_expression(expression, &param_ctx, 0)?
                }
                None => Value::Undef,
            };
            env.insert(parameter.name.clone(), value);
        }
        let module_ctx = Ctx {
            env: env_of(env),
            call_children: Some(CallChildren::List(&call.children)),
            module_stack: ctx.push_module(&module.name),
            ..ctx.clone()
        };
        self.eval_nodes(&module.children, &module_ctx, false)
    }

    /// `resolveOpenScadChildrenSelection` from the stable geometry semantics.
    pub(super) fn resolve_children_selection(
        &self,
        value: Option<&Value<'a>>,
        child_count: usize,
    ) -> EvalResult<Vec<usize>> {
        let Some(value) = value else {
            return Ok((0..child_count).collect());
        };
        let candidates: Vec<Value<'a>> = match value {
            Value::Number(_) => vec![value.clone()],
            Value::Vector(items) => items.as_ref().clone(),
            Value::Range { start, step, end } => {
                match crate::children_selection::expand_range(
                    *start,
                    *step,
                    *end,
                    MAX_RANGE_ITEMS as u64,
                ) {
                    Ok(values) => values.into_iter().map(Value::Number).collect(),
                    Err(crate::children_selection::RangeIssue::Invalid) => {
                        self.warn("Invalid children range was ignored");
                        return Ok(Vec::new());
                    }
                    Err(crate::children_selection::RangeIssue::Limit) => {
                        self.warn(format!("Range exceeds {} items", locale(MAX_RANGE_ITEMS)));
                        return Ok(Vec::new());
                    }
                }
            }
            _ => {
                self.warn("children accepts an empty argument list, number, vector, or range");
                return Ok(Vec::new());
            }
        };
        let values = candidates.iter().map(Value::as_number).collect::<Vec<_>>();
        let selection = crate::children_selection::select(&values, child_count as u64);
        for issue in selection.issues {
            match issue {
                crate::children_selection::Issue::Invalid { .. } => {
                    self.warn("Non-numeric children index was ignored")
                }
                crate::children_selection::Issue::OutOfBounds { index } => self.warn(format!(
                    "Children index {} is outside 0..{}",
                    js_number_to_string(index),
                    child_count.saturating_sub(1)
                )),
            }
        }
        Ok(selection
            .indices
            .into_iter()
            .map(|index| index as usize)
            .collect())
    }

    pub(super) fn slice_units(&self, start: usize, end: usize) -> String {
        let end = end.min(self.units.len());
        let start = start.min(end);
        String::from_utf16_lossy(&self.units[start..end])
    }

}
