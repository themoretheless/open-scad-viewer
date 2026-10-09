use super::*;

impl<'a> Evaluator<'a> {
pub(super) fn finite_number(&self, value: &Value, p: usize, label: &str) -> EvalResult<f64> {
        match value.as_number() {
            Some(v) if v.is_finite() => Ok(v),
            _ => Err(self.error(p, format!("{label} must be a finite number"))),
        }
    }

    pub(super) fn vector_value(&self, value: &Value, p: usize, label: &str) -> EvalResult<Vec<f64>> {
        let Some(items) = value.as_vector() else {
            return Err(self.error(p, format!("{label} must be a vector")));
        };
        let items = items.clone();
        items
            .iter()
            .map(|item| self.finite_number(item, p, label))
            .collect()
    }

    pub(super) fn evaluate_sequential_bindings(
        &self,
        args: &'a [ExpressionArgument],
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<HashMap<String, Value<'a>>> {
        let mut env = ctx.env.borrow().clone();
        let mut assigned = HashSet::new();
        for argument in args {
            let overlay = self.stable_overlay_context(ctx, env.clone());
            let value = self.eval_expression(&argument.value, &overlay, depth + 1)?;
            let Some(name) = &argument.name else {
                self.warn(format!(
                    "Ignoring assignment without variable name {}",
                    format_value(&value)
                ));
                continue;
            };
            if assigned.contains(name) {
                self.warn(format!(
                    "Ignoring duplicate variable assignment {name} = {}",
                    format_value(&value)
                ));
                continue;
            }
            assigned.insert(name.clone());
            env.insert(name.clone(), value);
        }
        Ok(env)
    }

    pub(super) fn stable_iterable(
        &self,
        value: &Value<'a>,
        _ctx: &Ctx<'a>,
        position: usize,
    ) -> EvalResult<Vec<Value<'a>>> {
        match value {
            Value::Range { start, step, end } => {
                materialize_range(*start, *step, *end, &mut self.semantics(position))
            }
            Value::Vector(items) => Ok(items.as_ref().clone()),
            Value::Str(s) => {
                let items: Vec<Value<'a>> =
                    s.chars().map(|c| Value::string(c.to_string())).collect();
                Ok(Rc::try_unwrap(self.register_vector(
                    Rc::new(items),
                    position,
                    "string iteration",
                )?)
                .unwrap_or_else(|_| unreachable!("fresh Rc")))
            }
            Value::Undef => Ok(Vec::new()),
            other => Ok(vec![other.clone()]),
        }
    }

    pub(super) fn append_comprehension_values(
        &self,
        output: &mut Vec<Value<'a>>,
        values: &[Value<'a>],
        expr: &Expr,
    ) -> EvalResult<()> {
        if output.len() + values.len() > MAX_VALUE_ELEMENTS {
            return Err(self.error(
                expr_p(expr),
                format!(
                    "List comprehension exceeds {} elements",
                    locale(MAX_VALUE_ELEMENTS)
                ),
            ));
        }
        output.extend(values.iter().cloned());
        Ok(())
    }

    pub(super) fn eval_comprehension_element(
        &self,
        expr: &'a Expr,
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<Vec<Value<'a>>> {
        let value = self.eval_expression(expr, ctx, depth + 1)?;
        if expr.is_list_comprehension()
            && let Value::Vector(items) = &value
        {
            return Ok(items.as_ref().clone());
        }
        Ok(vec![value])
    }

    pub(super) fn eval_list_comprehension(
        &self,
        expr: &'a Expr,
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<Vec<Value<'a>>> {
        match expr {
            Expr::LcEach { value, p } => {
                let value = self.eval_expression(value, ctx, depth + 1)?;
                self.stable_iterable(&value, ctx, *p)
            }
            Expr::LcIf {
                condition, yes, no, ..
            } => {
                let condition = self.eval_expression(condition, ctx, depth + 1)?;
                let selected = if truthy(&condition) {
                    Some(yes)
                } else {
                    no.as_ref()
                };
                match selected {
                    None => Ok(Vec::new()),
                    Some(selected) => self.eval_comprehension_element(selected, ctx, depth + 1),
                }
            }
            Expr::LcLet { args, body, .. } => {
                let env = self.evaluate_sequential_bindings(args, ctx, depth + 1)?;
                let overlay = self.stable_overlay_context(ctx, env);
                self.eval_list_comprehension(body, &overlay, depth + 1)
            }
            Expr::LcFor { args, body, p } => {
                let mut output: Vec<Value<'a>> = Vec::new();
                self.lc_for_visit(
                    args,
                    0,
                    ctx,
                    &mut |iteration_ctx| {
                        let values =
                            self.eval_comprehension_element(body, iteration_ctx, depth + 1)?;
                        self.append_comprehension_values(&mut output, &values, expr)?;
                        Ok(())
                    },
                    *p,
                )?;
                Ok(output)
            }
            Expr::LcForC {
                init,
                condition,
                update,
                body,
                p,
            } => {
                let mut output: Vec<Value<'a>> = Vec::new();
                let env = self.evaluate_sequential_bindings(init, ctx, depth + 1)?;
                let mut iteration_ctx = self.stable_overlay_context(ctx, env);
                loop {
                    let condition_value =
                        self.eval_expression(condition, &iteration_ctx, depth + 1)?;
                    if !truthy(&condition_value) {
                        break;
                    }
                    self.bump_ops(*p)?;
                    let values =
                        self.eval_comprehension_element(body, &iteration_ctx, depth + 1)?;
                    self.append_comprehension_values(&mut output, &values, expr)?;
                    let env =
                        self.evaluate_sequential_bindings(update, &iteration_ctx, depth + 1)?;
                    iteration_ctx = self.stable_overlay_context(&iteration_ctx, env);
                }
                Ok(output)
            }
            _ => unreachable!("list comprehension kind checked by caller"),
        }
    }

    pub(super) fn lc_for_visit(
        &self,
        args: &'a [ExpressionArgument],
        binding_index: usize,
        ctx: &Ctx<'a>,
        visit: &mut dyn FnMut(&Ctx<'a>) -> EvalResult<()>,
        p: usize,
    ) -> EvalResult<()> {
        if binding_index >= args.len() {
            return visit(ctx);
        }
        let binding = &args[binding_index];
        let iterable = {
            let value = self.eval_expression(&binding.value, ctx, 1)?;
            self.stable_iterable(&value, ctx, binding.p)?
        };
        let Some(name) = &binding.name else {
            self.warn("Ignoring for() iterator without variable name");
            return Ok(());
        };
        for value in iterable {
            self.bump_ops(p)?;
            let mut env = ctx.env.borrow().clone();
            env.insert(name.clone(), value);
            let overlay = self.stable_overlay_context(ctx, env);
            self.lc_for_visit(args, binding_index + 1, &overlay, visit, p)?;
        }
        Ok(())
    }

    pub(super) fn eval_assert_expression(
        &self,
        args: &'a [ExpressionArgument],
        body: Option<&'a Expr>,
        p: usize,
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<Value<'a>> {
        let resolved =
            self.resolve_stable_expression_arguments(args, &["condition", "message"], ctx);
        let condition_argument = resolved.get("condition");
        let message_argument = resolved.get("message");
        let condition = match condition_argument {
            Some(argument) => self.eval_expression(&argument.value, ctx, depth + 1)?,
            None => Value::Undef,
        };
        let message = match message_argument {
            Some(argument) => self.eval_expression(&argument.value, ctx, depth + 1)?,
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
            return Err(self.error(p, format!("Assertion '{condition_text}' failed{detail}")));
        }
        match body {
            None => Ok(Value::Undef),
            Some(body) => self.eval_expression(body, ctx, depth + 1),
        }
    }

    pub(super) fn eval_echo_expression(
        &self,
        args: &'a [ExpressionArgument],
        body: Option<&'a Expr>,
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<Value<'a>> {
        let mut values = Vec::new();
        for argument in args {
            let value = format_value(&self.eval_expression(&argument.value, ctx, depth + 1)?);
            values.push(match &argument.name {
                None => value,
                Some(name) => format!("{name} = {value}"),
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
        match body {
            None => Ok(Value::Undef),
            Some(body) => self.eval_expression(body, ctx, depth + 1),
        }
    }
}
