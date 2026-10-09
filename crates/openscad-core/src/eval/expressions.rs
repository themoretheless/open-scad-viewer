use super::*;

impl<'a> Evaluator<'a> {
pub(super) fn eval_expression(
        &self,
        expr: &'a Expr,
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<Value<'a>> {
        self.bump_ops(expr_p(expr))?;
        if depth >= MAX_EXPRESSION_DEPTH {
            return Err(self.error(
                expr_p(expr),
                format!("Expression exceeds {MAX_EXPRESSION_DEPTH} evaluated levels"),
            ));
        }
        match expr {
            Expr::Literal { value, .. } => Ok(match value {
                LiteralValue::Undef => Value::Undef,
                LiteralValue::Bool(v) => Value::Bool(*v),
                LiteralValue::Number(v) => Value::Number(*v),
                LiteralValue::String(v) => Value::string(v.clone()),
            }),
            Expr::Identifier { name, p } => {
                if name == "PI" {
                    return Ok(Value::Number(std::f64::consts::PI));
                }
                let resolved = if self.is_stable() {
                    self.resolve_stable_variable(name, ctx)?
                } else {
                    match ctx.env.borrow().get(name) {
                        Some(value) => VariableResolution {
                            found: true,
                            value: value.clone(),
                        },
                        None => VariableResolution {
                            found: false,
                            value: Value::Undef,
                        },
                    }
                };
                if !resolved.found {
                    if !self.is_stable() {
                        return Err(self.error(*p, format!("Unknown variable {name}")));
                    }
                    self.warn(format!("Ignoring unknown variable '{name}'"));
                    return Ok(Value::Undef);
                }
                Ok(resolved.value)
            }
            Expr::Vector { items, p } => {
                if !self.is_stable() {
                    let mut values = Vec::with_capacity(items.len());
                    for item in items {
                        values.push(self.eval_expression(item, ctx, depth + 1)?);
                    }
                    return Ok(Value::vector(
                        Rc::try_unwrap(self.register_vector(
                            Rc::new(values),
                            *p,
                            "Evaluated value",
                        )?)
                        .unwrap_or_else(|_| unreachable!("fresh Rc")),
                    ));
                }
                let mut values: Vec<Value<'a>> = Vec::new();
                for item in items {
                    let value = self.eval_expression(item, ctx, depth + 1)?;
                    if item.is_list_comprehension()
                        && let Value::Vector(comprehension) = &value
                    {
                        self.append_comprehension_values(&mut values, comprehension, item)?;
                        continue;
                    }
                    values.push(value);
                }
                Ok(Value::vector(
                    Rc::try_unwrap(self.register_vector(Rc::new(values), *p, "Evaluated value")?)
                        .unwrap_or_else(|_| unreachable!("fresh Rc")),
                ))
            }
            Expr::Range {
                start,
                step,
                end,
                p,
            } => {
                if self.is_stable() {
                    let start = self.eval_expression(start, ctx, depth + 1)?;
                    let end = self.eval_expression(end, ctx, depth + 1)?;
                    let step = match step {
                        None => Value::Number(1.0),
                        Some(step) => self.eval_expression(step, ctx, depth + 1)?,
                    };
                    let (Value::Number(start), Value::Number(step), Value::Number(end)) =
                        (&start, &step, &end)
                    else {
                        self.warn("Invalid range bounds produce undef");
                        return Ok(Value::Undef);
                    };
                    if ![start, step, end].iter().all(|v| v.is_finite()) {
                        self.warn("Invalid range bounds produce undef");
                        return Ok(Value::Undef);
                    }
                    if *step > 0.0 && start > end {
                        self.warn("begin is greater than the end, but step is positive");
                    } else if *step < 0.0 && start < end {
                        self.warn("begin is smaller than the end, but step is negative");
                    }
                    return Ok(Value::Range {
                        start: *start,
                        step: *step,
                        end: *end,
                    });
                }
                let start = self.finite_number(
                    &self.eval_expression(start, ctx, depth + 1)?,
                    *p,
                    "range start",
                )?;
                let end = self.finite_number(
                    &self.eval_expression(end, ctx, depth + 1)?,
                    *p,
                    "range end",
                )?;
                let step = match step {
                    None => 1.0,
                    Some(step) => self.finite_number(
                        &self.eval_expression(step, ctx, depth + 1)?,
                        *p,
                        "range step",
                    )?,
                };
                if step == 0.0 {
                    return Err(self.error(*p, "Range step cannot be zero"));
                }
                let mut values = Vec::new();
                let forward = step > 0.0;
                let mut value = start;
                while if forward {
                    value <= end + 1e-10
                } else {
                    value >= end - 1e-10
                } {
                    values.push(Value::Number(value));
                    if values.len() > MAX_RANGE_ITEMS {
                        return Err(self.error(
                            *p,
                            format!("Range exceeds {} items", locale(MAX_RANGE_ITEMS)),
                        ));
                    }
                    value += step;
                }
                Ok(Value::vector(
                    Rc::try_unwrap(self.register_vector(Rc::new(values), *p, "Evaluated value")?)
                        .unwrap_or_else(|_| unreachable!("fresh Rc")),
                ))
            }
            Expr::Unary { op, value, p } => {
                let value = self.eval_expression(value, ctx, depth + 1)?;
                if self.is_stable() {
                    return unary(*op, &value, &mut self.semantics(*p));
                }
                if *op == TT::Not {
                    return Ok(Value::Bool(!truthy(&value)));
                }
                let number = self.finite_number(&value, *p, "unary operand")?;
                Ok(Value::Number(if *op == TT::Minus {
                    -number
                } else {
                    number
                }))
            }
            Expr::Binary { op, left, right, p } => {
                if self.is_stable() {
                    let left = self.eval_expression(left, ctx, depth + 1)?;
                    if *op == TT::And && !truthy(&left) {
                        return Ok(Value::Bool(false));
                    }
                    if *op == TT::Or && truthy(&left) {
                        return Ok(Value::Bool(true));
                    }
                    let right = self.eval_expression(right, ctx, depth + 1)?;
                    return binary(*op, &left, &right, &mut self.semantics(*p));
                }
                if *op == TT::And {
                    let left = self.eval_expression(left, ctx, depth + 1)?;
                    if !truthy(&left) {
                        return Ok(Value::Bool(false));
                    }
                    let right = self.eval_expression(right, ctx, depth + 1)?;
                    return Ok(Value::Bool(truthy(&right)));
                }
                if *op == TT::Or {
                    let left = self.eval_expression(left, ctx, depth + 1)?;
                    if truthy(&left) {
                        return Ok(Value::Bool(true));
                    }
                    let right = self.eval_expression(right, ctx, depth + 1)?;
                    return Ok(Value::Bool(truthy(&right)));
                }
                let left = self.eval_expression(left, ctx, depth + 1)?;
                let right = self.eval_expression(right, ctx, depth + 1)?;
                if *op == TT::EqEq {
                    return Ok(Value::Bool(deep_equal(&left, &right)));
                }
                if *op == TT::NotEq {
                    return Ok(Value::Bool(!deep_equal(&left, &right)));
                }
                if matches!(op, TT::Lt | TT::Gt | TT::LtEq | TT::GtEq) {
                    let a = self.finite_number(&left, *p, "comparison operand")?;
                    let b = self.finite_number(&right, *p, "comparison operand")?;
                    return Ok(Value::Bool(match op {
                        TT::Lt => a < b,
                        TT::Gt => a > b,
                        TT::LtEq => a <= b,
                        _ => a >= b,
                    }));
                }
                let a = self.finite_number(&left, *p, "arithmetic operand")?;
                let b = self.finite_number(&right, *p, "arithmetic operand")?;
                let result = match op {
                    TT::Plus => a + b,
                    TT::Minus => a - b,
                    TT::Star => a * b,
                    TT::Slash => a / b,
                    TT::Percent => a % b,
                    _ => a.powf(b),
                };
                if !result.is_finite() {
                    return Err(self.error(*p, "Expression produced a non-finite number"));
                }
                Ok(Value::Number(result))
            }
            Expr::Ternary { test, yes, no, .. } => {
                let condition = self.eval_expression(test, ctx, depth + 1)?;
                if truthy(&condition) {
                    self.eval_expression(yes, ctx, depth + 1)
                } else {
                    self.eval_expression(no, ctx, depth + 1)
                }
            }
            Expr::Index { value, index, p } => {
                let value = self.eval_expression(value, ctx, depth + 1)?;
                if self.is_stable() {
                    let index = self.eval_expression(index, ctx, depth + 1)?;
                    return index_value(&value, &index, &mut self.semantics(*p));
                }
                let raw =
                    self.finite_number(&self.eval_expression(index, ctx, depth + 1)?, *p, "index")?;
                let index = raw.trunc();
                match &value {
                    Value::Vector(items) => {
                        if index < 0.0 || index >= items.len() as f64 {
                            return Ok(Value::Undef);
                        }
                        Ok(items[index as usize].clone())
                    }
                    // The JS host indexes strings by UTF-16 code unit.
                    Value::Str(s) => {
                        if index < 0.0 {
                            return Ok(Value::Undef);
                        }
                        Ok(s.encode_utf16()
                            .nth(index as usize)
                            .map_or(Value::Undef, |unit| {
                                Value::string(String::from_utf16_lossy(&[unit]))
                            }))
                    }
                    _ => Err(self.error(*p, "Only vectors and strings can be indexed")),
                }
            }
            Expr::Member { value, name, p } => {
                let value = self.eval_expression(value, ctx, depth + 1)?;
                if self.is_stable() {
                    return member(&value, name, &mut self.semantics(*p));
                }
                if let Value::Vector(items) = &value {
                    let index = match name.as_str() {
                        "x" => Some(0),
                        "y" => Some(1),
                        "z" => Some(2),
                        _ => None,
                    };
                    if let Some(index) = index {
                        return Ok(items.get(index).cloned().unwrap_or(Value::Undef));
                    }
                }
                Err(self.error(*p, format!("Value has no member {name}")))
            }
            Expr::Function { params, body, .. } => {
                let value = FunctionValue {
                    name: None,
                    params,
                    body,
                    closure: Rc::new(ctx.env.borrow().clone()),
                    lexical_scope: if self.is_stable() {
                        ctx.stable_scope.clone()
                    } else {
                        None
                    },
                };
                Ok(Value::Function(Rc::new(value)))
            }
            Expr::Call { .. } => self.eval_function_call(expr, ctx, depth),
            Expr::Let { args, body, p } => {
                if !self.is_stable() {
                    return Err(self.error(*p, "let expression is not supported"));
                }
                let env = self.evaluate_sequential_bindings(args, ctx, depth + 1)?;
                let overlay = self.stable_overlay_context(ctx, env);
                self.eval_expression(body, &overlay, depth + 1)
            }
            Expr::Assert { args, body, p } => {
                if !self.is_stable() {
                    return Err(self.error(*p, "assert expression is not supported"));
                }
                self.eval_assert_expression(args, body.as_deref(), *p, ctx, depth)
            }
            Expr::Echo { args, body, p } => {
                if !self.is_stable() {
                    return Err(self.error(*p, "echo expression is not supported"));
                }
                self.eval_echo_expression(args, body.as_deref(), ctx, depth)
            }
            Expr::LcFor { .. }
            | Expr::LcForC { .. }
            | Expr::LcIf { .. }
            | Expr::LcLet { .. }
            | Expr::LcEach { .. } => {
                if !self.is_stable() {
                    return Err(self.error(expr_p(expr), "list comprehension is not supported"));
                }
                let values = self.eval_list_comprehension(expr, ctx, depth)?;
                Ok(Value::vector(
                    Rc::try_unwrap(self.register_vector(
                        Rc::new(values),
                        expr_p(expr),
                        "list comprehension",
                    )?)
                    .unwrap_or_else(|_| unreachable!("fresh Rc")),
                ))
            }
        }
    }
}
