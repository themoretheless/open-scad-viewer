use super::*;

impl<'a> Evaluator<'a> {
pub(super) fn eval_function_call(
        &self,
        expr: &'a Expr,
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<Value<'a>> {
        let Expr::Call {
            name,
            callee,
            args,
            p,
        } = expr
        else {
            unreachable!()
        };
        if let Some(name) = name {
            let declaration = if self.is_stable() {
                ctx.stable_scope
                    .as_ref()
                    .and_then(|scope| scope.function_declaration(name))
            } else {
                None
            };
            let definition = declaration
                .as_ref()
                .map(|(node, _)| *node)
                .or_else(|| self.functions.get(name).copied());
            if let Some(definition) = definition
                && (!self.is_stable() || declaration.is_some())
            {
                let closure = match &declaration {
                    Some((_, scope)) => scope.env.borrow().clone(),
                    None => ctx.env.borrow().clone(),
                };
                let value = FunctionValue {
                    name: Some(definition.name.clone()),
                    params: &definition.params,
                    body: &definition.body,
                    closure: Rc::new(closure),
                    lexical_scope: if self.is_stable() {
                        declaration
                            .as_ref()
                            .map(|(_, scope)| scope.clone())
                            .or_else(|| ctx.stable_scope.clone())
                    } else {
                        None
                    },
                };
                return self.invoke_user_function(&value, args, ctx, depth);
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
            if resolved.found
                && let Value::Function(function) = &resolved.value
            {
                let function = function.clone();
                return self.invoke_user_function(&function, args, ctx, depth);
            }
            return self.eval_builtin(name, args, *p, ctx, depth);
        }
        let callee = self.eval_expression(callee, ctx, depth + 1)?;
        let Value::Function(function) = &callee else {
            return Err(self.error(*p, "Expression is not callable"));
        };
        let function = function.clone();
        self.invoke_user_function(&function, args, ctx, depth)
    }

    pub(super) fn invoke_user_function(
        &self,
        function: &FunctionValue<'a>,
        args: &'a [ExpressionArgument],
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<Value<'a>> {
        if ctx.function_stack.len() >= MAX_EVAL_DEPTH {
            return Err(self.error(
                args.first().map_or(expr_p(function.body), |a| a.p),
                format!("Evaluation exceeds {MAX_EVAL_DEPTH} nested function calls"),
            ));
        }
        if self.is_stable() {
            let lexical_scope = function
                .lexical_scope
                .clone()
                .or_else(|| ctx.stable_scope.clone());
            let Some(lexical_scope) = lexical_scope else {
                return Err(self.error(
                    args.first().map_or(expr_p(function.body), |a| a.p),
                    "Stable function is missing its lexical scope",
                ));
            };
            let parameter_names: Vec<&str> =
                function.params.iter().map(|p| p.name.as_str()).collect();
            let resolved = self.resolve_stable_expression_arguments(args, &parameter_names, ctx);
            let mut caller_values: Vec<Value<'a>> = Vec::with_capacity(args.len());
            for argument in args {
                caller_values.push(self.eval_expression(&argument.value, ctx, depth + 1)?);
            }
            let caller_value_of = |argument: &ExpressionArgument| -> Value<'a> {
                // ExpressionArgument identity by position in the args slice.
                let index = args.iter().position(|a| std::ptr::eq(a, argument)).unwrap();
                caller_values[index].clone()
            };
            let mut definition_env = (*function.closure).clone();
            overlay_dynamic_variables(&mut definition_env, &ctx.env.borrow());
            let definition_ctx = Ctx {
                env: env_of(definition_env.clone()),
                stable_scope: Some(lexical_scope),
                scope_visible_before: usize::MAX,
                ..ctx.clone()
            };
            let mut env = definition_env.clone();
            for parameter in function.params {
                let value = match resolved.get(&parameter.name) {
                    Some(supplied) => caller_value_of(supplied),
                    None => match &parameter.default_value {
                        Some(default) => self.eval_expression(
                            default,
                            &definition_ctx.with_env(definition_env.clone()),
                            depth + 1,
                        )?,
                        None => Value::Undef,
                    },
                };
                env.insert(parameter.name.clone(), value);
            }
            let body_ctx = Ctx {
                function_stack: ctx
                    .push_function(function.name.as_deref().unwrap_or("<anonymous>")),
                ..self.stable_overlay_context(&definition_ctx, env)
            };
            return self.eval_expression(function.body, &body_ctx, depth + 1);
        }
        // Subset profile: positional + named with strict validation.
        let positional: Vec<&ExpressionArgument> =
            args.iter().filter(|a| a.name.is_none()).collect();
        let mut named: HashMap<&str, &ExpressionArgument> = HashMap::new();
        for argument in args.iter().filter(|a| a.name.is_some()) {
            named.insert(argument.name.as_deref().unwrap(), argument);
        }
        let parameter_names: HashSet<&str> =
            function.params.iter().map(|p| p.name.as_str()).collect();
        for name in named.keys() {
            if !parameter_names.contains(name) {
                let p = args
                    .iter()
                    .find(|a| a.name.as_deref() == Some(name))
                    .map_or(expr_p(function.body), |a| a.p);
                return Err(self.error(p, format!("Unknown argument {name}")));
            }
        }
        if positional.len() > function.params.len() {
            let p = positional
                .get(function.params.len())
                .map_or(expr_p(function.body), |a| a.p);
            return Err(self.error(p, "Too many function arguments"));
        }
        let mut caller_values: Vec<Value<'a>> = Vec::with_capacity(args.len());
        for argument in args {
            caller_values.push(self.eval_expression(&argument.value, ctx, depth + 1)?);
        }
        let caller_value_of = |argument: &ExpressionArgument| -> Value<'a> {
            let index = args.iter().position(|a| std::ptr::eq(a, argument)).unwrap();
            caller_values[index].clone()
        };
        let mut env = (*function.closure).clone();
        for (index, parameter) in function.params.iter().enumerate() {
            let supplied = named
                .get(parameter.name.as_str())
                .copied()
                .or_else(|| positional.get(index).copied());
            let value = match supplied {
                Some(argument) => caller_value_of(argument),
                None => match &parameter.default_value {
                    Some(default) => {
                        let default_ctx = ctx.with_env(env.clone());
                        self.eval_expression(default, &default_ctx, depth + 1)?
                    }
                    None => Value::Undef,
                },
            };
            env.insert(parameter.name.clone(), value);
        }
        let body_ctx = Ctx {
            env: env_of(env),
            function_stack: ctx.push_function(function.name.as_deref().unwrap_or("<anonymous>")),
            ..ctx.clone()
        };
        self.eval_expression(function.body, &body_ctx, depth + 1)
    }

    pub(super) fn compatibility_string(&self, value: &Value) -> String {
        match value {
            Value::Undef => String::new(),
            Value::Str(s) => s.to_string(),
            Value::Number(v) => js_number_to_string(*v),
            Value::Bool(v) => if *v { "true" } else { "false" }.to_string(),
            other => format_value(other),
        }
    }

    pub(super) fn eval_dxf_query_builtin(
        &self,
        name: &str,
        args: &'a [ExpressionArgument],
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<Value<'a>> {
        let supported: HashSet<&str> = if name == "dxf_dim" {
            ["file", "layer", "origin", "scale", "name"]
                .into_iter()
                .collect()
        } else {
            ["file", "layer", "origin", "scale"].into_iter().collect()
        };
        let mut values: HashMap<String, Value<'a>> = HashMap::new();
        for argument in args {
            let value = self.eval_expression(&argument.value, ctx, depth + 1)?;
            let argument_name = argument.name.clone().unwrap_or_default();
            if !supported.contains(argument_name.as_str()) {
                self.warn(format!("{name}(..., {argument_name}=...) is not supported"));
                continue;
            }
            values.insert(argument_name, value);
        }
        let specifier = self.compatibility_string(values.get("file").unwrap_or(&Value::Undef));
        // Project compilation arrives with stage 4; without a project the
        // reference runtime warns and yields undef.
        self.warn(format!("Can't open DXF file '{specifier}'!"));
        Ok(Value::Undef)
    }

    pub(super) fn eval_builtin(
        &self,
        name: &str,
        args: &'a [ExpressionArgument],
        p: usize,
        ctx: &Ctx<'a>,
        depth: usize,
    ) -> EvalResult<Value<'a>> {
        if name == "assert" {
            return Err(self.error(
                p,
                "Expression-form assert() is not supported; use statement assert()",
            ));
        }
        if self.is_stable() && (name == "dxf_dim" || name == "dxf_cross") {
            return self.eval_dxf_query_builtin(name, args, ctx, depth);
        }
        if !self.is_stable() && args.iter().any(|argument| argument.name.is_some()) {
            return Err(self.error(
                p,
                format!("{name}() does not accept named arguments in this engine revision"),
            ));
        }
        let is_undef_probe = self.is_stable() && name == "is_undef" && args.len() == 1;
        let mut access = |index: usize| -> Result<Value<'a>, BuiltinError> {
            let argument = &args[index];
            // OpenSCAD permits probing an undeclared bare name with is_undef()
            // without emitting the ordinary unknown-variable warning.
            if is_undef_probe && let Expr::Identifier { name, .. } = &argument.value {
                if name == "PI" {
                    return Ok(Value::Number(std::f64::consts::PI));
                }
                let resolved = self.resolve_stable_variable(name, ctx)?;
                return Ok(if resolved.found {
                    resolved.value
                } else {
                    Value::Undef
                });
            }
            Ok(self.eval_expression(&argument.value, ctx, depth + 1)?)
        };
        let mut warn = |message: String| self.warn(message);
        let mut register_array =
            |items: Vec<Value<'a>>, label: &str| -> EvalResult<Vec<Value<'a>>> {
                if items.len() > MAX_VALUE_ELEMENTS {
                    return Err(self.error(
                        p,
                        format!("{label} exceeds {} elements", locale(MAX_VALUE_ELEMENTS)),
                    ));
                }
                Ok(
                    Rc::try_unwrap(self.register_vector(Rc::new(items), p, label)?)
                        .unwrap_or_else(|_| unreachable!("fresh Rc")),
                )
            };
        let mut register_string = |value: String, label: &str| -> EvalResult<String> {
            if value.encode_utf16().count() > MAX_VALUE_ELEMENTS {
                return Err(self.error(
                    p,
                    format!("{label} exceeds {} characters", locale(MAX_VALUE_ELEMENTS)),
                ));
            }
            self.register_string(value, p, label)
        };
        let mut random = || {
            if let Some(host) = self.random_host
                && let Ok(mut host) = host.try_borrow_mut()
            {
                return (host)();
            }
            let (next, value) = splitmix64(self.random_state.get());
            self.random_state.set(next);
            value
        };
        let parent_module = |depth: usize| -> Option<String> {
            let stack = &ctx.module_stack;
            stack
                .len()
                .checked_sub(1 + depth)
                .map(|index| stack[index].clone())
        };
        let mut builtin_ctx = BuiltinContext {
            warn: &mut warn,
            register_array: &mut register_array,
            register_string: &mut register_string,
            random: &mut random,
            parent_module: &parent_module,
        };
        let Some(result) =
            builtins::evaluate_builtin(name, args.len(), &mut access, &mut builtin_ctx)
        else {
            return Err(self.error(p, format!("Unsupported function {name}()")));
        };
        match result {
            Ok(value) => Ok(value),
            Err(BuiltinError::Eval(failure)) => Err(failure),
            Err(BuiltinError::Fail(message)) => {
                if self.is_stable() {
                    self.warn(message);
                    Ok(Value::Undef)
                } else {
                    Err(self.error(p, message))
                }
            }
        }
    }
}
