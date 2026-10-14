use super::*;

impl<'a> Evaluator<'a> {
    pub fn new(
        statements: &'a [Statement],
        units: &'a [u16],
        profile: LanguageProfile,
        options: EvaluatorOptions<'a>,
    ) -> Self {
        let mut functions = HashMap::new();
        let mut modules = HashMap::new();
        collect_functions(statements, &mut functions);
        collect_modules(statements, &mut modules);
        Self {
            record_geometry: options.record_geometry,
            geometry: RefCell::new(Vec::new()),
            profiles: RefCell::new(Vec::new()),
            units,
            profile,
            quality: options.quality,
            animation_time: options.animation_time,
            ops: Cell::new(0),
            value_budget: Cell::new(0),
            value_meta: RefCell::new(HashMap::new()),
            warnings: RefCell::new(Vec::new()),
            functions,
            modules,
            reduced: Cell::new(false),
            should_abort: options.should_abort,
            random_host: options.random,
            random_state: Cell::new(0x853c49e6748fea9b),
        }
    }

    pub(super) fn store_geometry(
        &self,
        node: geometry_ops::solid_program::Node,
        p: usize,
    ) -> EvalResult<usize> {
        let mut nodes = self.geometry.borrow_mut();
        if nodes.len() >= crate::MAX_AST_NODES {
            return Err(self.error(p, "Native geometry program budget exceeded"));
        }
        let id = nodes.len();
        nodes.push(node);
        Ok(id)
    }
    pub(super) fn emit_geometry(
        &self,
        name: &str,
        node: geometry_ops::solid_program::Node,
        p: usize,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let mut shape = descriptor(name, 3);
        if self.record_geometry {
            shape.geometry = Some(self.store_geometry(node, p)?)
        }
        Ok(vec![shape])
    }
    pub(super) fn emit_profile(
        &self,
        name: &str,
        node: geometry_ops::profile_program::Node,
        p: usize,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let mut shape = descriptor(name, 2);
        let mut profiles = self.profiles.borrow_mut();
        if profiles.len() >= crate::MAX_AST_NODES {
            return Err(self.error(p, "Native profile program budget exceeded"));
        }
        shape.profile = Some(profiles.len());
        profiles.push(node);
        Ok(vec![shape])
    }
    pub(super) fn affine_geometry(
        &self,
        mut shapes: Vec<ShapeDescriptor>,
        matrix: geometry_ops::solid_program::affine::AffineMatrix,
        p: usize,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        if self.record_geometry {
            for shape in &mut shapes {
                let input = shape
                    .geometry
                    .ok_or_else(|| self.error(p, "Unsupported native geometry input"))?;
                shape.geometry = Some(self.store_geometry(
                    geometry_ops::solid_program::Node::Transform { input, matrix },
                    p,
                )?);
            }
        }
        Ok(shapes)
    }
    pub(super) fn is_stable(&self) -> bool {
        self.profile.is_stable()
    }

    pub(super) fn error(&self, p: usize, message: impl Into<String>) -> EvalFailure {
        EvalFailure::Error(ParseError::new(p, message))
    }

    pub(super) fn warn(&self, message: impl Into<String>) {
        let message = message.into();
        let mut warnings = self.warnings.borrow_mut();
        if !warnings.contains(&message) {
            warnings.push(message);
        }
    }

    pub(super) fn poll(&self) -> EvalResult<()> {
        if let Some(abort) = self.should_abort
            && abort()
        {
            return Err(EvalFailure::Aborted);
        }
        Ok(())
    }

    pub(super) fn bump_ops(&self, p: usize) -> EvalResult<()> {
        let ops = self.ops.get() + 1;
        self.ops.set(ops);
        if ops > MAX_EVAL_OPS {
            return Err(self.error(
                p,
                format!(
                    "Model exceeds the {} evaluation step limit",
                    locale(MAX_EVAL_OPS as usize)
                ),
            ));
        }
        Ok(())
    }

    pub(super) fn value_weight(&self, value: &Value<'a>) -> u64 {
        match value {
            Value::Vector(items) => self
                .value_meta
                .borrow()
                .get(&(Rc::as_ptr(items)))
                .map_or(items.len() as u64 + 1, |(weight, _)| *weight),
            Value::Str(s) => s.encode_utf16().count().max(1) as u64,
            _ => 1,
        }
    }

    pub(super) fn value_depth(&self, value: &Value<'a>) -> u32 {
        match value {
            Value::Vector(items) => self
                .value_meta
                .borrow()
                .get(&(Rc::as_ptr(items)))
                .map_or(1, |(_, depth)| *depth),
            _ => 0,
        }
    }

    /// `registerArrayValue`: per-value weight/depth accounting against the
    /// 500,000-unit allocation budget.
    pub(super) fn register_vector(
        &self,
        value: Rc<Vec<Value<'a>>>,
        p: usize,
        label: &str,
    ) -> EvalResult<Rc<Vec<Value<'a>>>> {
        let mut weight = 1u64;
        let mut depth = 1u32;
        for item in value.iter() {
            weight += self.value_weight(item);
            depth = depth.max(self.value_depth(item) + 1);
            if weight > MAX_EVALUATED_VALUE_UNITS {
                return Err(self.error(
                    p,
                    format!(
                        "{label} exceeds {} units",
                        locale(MAX_EVALUATED_VALUE_UNITS as usize)
                    ),
                ));
            }
        }
        if depth as usize > MAX_EXPRESSION_DEPTH {
            return Err(self.error(
                p,
                format!("Evaluated value exceeds {MAX_EXPRESSION_DEPTH} nested levels"),
            ));
        }
        let used = self.value_budget.get() + weight;
        self.value_budget.set(used);
        if used > MAX_EVALUATED_VALUE_UNITS {
            return Err(self.error(
                p,
                format!(
                    "{label} exceeds the {} value-allocation budget",
                    locale(MAX_EVALUATED_VALUE_UNITS as usize)
                ),
            ));
        }
        self.value_meta
            .borrow_mut()
            .insert(Rc::as_ptr(&value), (weight, depth));
        Ok(value)
    }

    pub(super) fn register_string(&self, value: String, p: usize, label: &str) -> EvalResult<String> {
        let used = self.value_budget.get() + value.encode_utf16().count().max(1) as u64;
        self.value_budget.set(used);
        if used > MAX_EVALUATED_VALUE_UNITS {
            return Err(self.error(
                p,
                format!(
                    "{label} exceeds the {} value-allocation budget",
                    locale(MAX_EVALUATED_VALUE_UNITS as usize)
                ),
            ));
        }
        Ok(value)
    }

    pub(super) fn semantics<'h>(&'h self, p: usize) -> SemanticsContext<'h, 'a> {
        SemanticsContext {
            max_range_items: MAX_RANGE_ITEMS,
            warn: Box::new(move |message| self.warn(message)),
            register_array: Box::new(move |values, label| {
                Ok(
                    Rc::try_unwrap(self.register_vector(Rc::new(values), p, label)?)
                        .unwrap_or_else(|_| unreachable!("fresh Rc")),
                )
            }),
        }
    }

    // ------------------------------------------------------------------
    // Scope handling (stable profile) and overlays
    // ------------------------------------------------------------------

    pub(super) fn resolve_stable_variable(
        &self,
        name: &str,
        ctx: &Ctx<'a>,
    ) -> EvalResult<VariableResolution<'a>> {
        if name.starts_with('$')
            && let Some(value) = ctx.env.borrow().get(name)
        {
            return Ok(VariableResolution {
                found: true,
                value: value.clone(),
            });
        }
        let mut scope = ctx.stable_scope.clone();
        let mut visible_before = ctx.scope_visible_before;
        while let Some(current) = scope {
            let warn_self = self;
            let local = current.resolve_local(
                name,
                visible_before,
                &mut |expression, site| {
                    let site_ctx = Ctx {
                        env: env_of(site.env),
                        stable_scope: Some(site.scope),
                        scope_visible_before: site.visible_before,
                        ..ctx.clone()
                    };
                    self.eval_expression(expression, &site_ctx, 0)
                },
                &mut |message| warn_self.warn(message),
            )?;
            if local.found {
                return Ok(local);
            }
            if let Some(root) = &ctx.stable_scope
                && Rc::ptr_eq(&current, root)
                && let Some(value) = ctx.env.borrow().get(name)
            {
                return Ok(VariableResolution {
                    found: true,
                    value: value.clone(),
                });
            }
            scope = current.parent.clone();
            visible_before = usize::MAX;
        }
        if let Some(value) = ctx.env.borrow().get(name) {
            return Ok(VariableResolution {
                found: true,
                value: value.clone(),
            });
        }
        Ok(VariableResolution {
            found: false,
            value: Value::Undef,
        })
    }

    pub(super) fn stable_overlay_context(&self, ctx: &Ctx<'a>, env: HashMap<String, Value<'a>>) -> Ctx<'a> {
        if !self.is_stable() {
            return ctx.with_env(env);
        }
        let scope = Rc::new(StableScope::new(&[], ctx.stable_scope.clone(), env));
        Ctx {
            env: scope.env.clone(),
            stable_scope: Some(scope),
            scope_visible_before: usize::MAX,
            ..ctx.clone()
        }
    }

    pub(super) fn enter_stable_statement_scope(
        &self,
        statements: &'a [Statement],
        parent: &Ctx<'a>,
    ) -> EvalResult<Ctx<'a>> {
        let scope = Rc::new(StableScope::new(
            statements,
            parent.stable_scope.clone(),
            parent.env.borrow().clone(),
        ));
        let ctx = Ctx {
            env: scope.env.clone(),
            stable_scope: Some(scope.clone()),
            scope_visible_before: usize::MAX,
            ..parent.clone()
        };
        for name in scope.dynamic_variable_names() {
            let resolved = scope.resolve_local(
                &name,
                usize::MAX,
                &mut |expression, site| {
                    let site_ctx = Ctx {
                        env: env_of(site.env),
                        stable_scope: Some(site.scope),
                        scope_visible_before: site.visible_before,
                        ..ctx.clone()
                    };
                    self.eval_expression(expression, &site_ctx, 0)
                },
                &mut |message| self.warn(message),
            )?;
            if resolved.found {
                scope.env.borrow_mut().insert(name, resolved.value);
            }
        }
        Ok(ctx)
    }

    // ------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------





    // ------------------------------------------------------------------
    // Function calls
    // ------------------------------------------------------------------

    pub(super) fn resolve_stable_expression_arguments(
        &self,
        args: &'a [ExpressionArgument],
        parameter_names: &[&str],
        _ctx: &Ctx<'a>,
    ) -> HashMap<String, &'a ExpressionArgument> {
        let mut resolved: HashMap<String, &'a ExpressionArgument> = HashMap::new();
        let parameters: HashSet<&str> = parameter_names.iter().copied().collect();
        for argument in args {
            let name = match &argument.name {
                Some(name) => Some(name.clone()),
                None => parameter_names
                    .iter()
                    .find(|parameter| !resolved.contains_key(**parameter))
                    .map(|s| s.to_string()),
            };
            let Some(name) = name else {
                self.warn("Ignoring excess positional argument");
                continue;
            };
            if !parameters.contains(name.as_str()) {
                self.warn(format!("Ignoring unknown argument {name}"));
                continue;
            }
            if argument.name.is_some() && resolved.contains_key(&name) {
                self.warn(format!("Argument {name} was specified more than once"));
            }
            resolved.insert(name, argument);
        }
        resolved
    }



    // ------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------

    pub(super) fn eval_nodes(
        &self,
        nodes: &'a [Statement],
        parent: &Ctx<'a>,
        scoped: bool,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let ctx = if self.is_stable() {
            self.enter_stable_statement_scope(nodes, parent)?
        } else if scoped {
            parent.with_env(parent.env.borrow().clone())
        } else {
            parent.clone()
        };
        self.eval_prepared_nodes(nodes, &ctx)
    }

    pub(super) fn eval_prepared_call(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let background = self.is_stable()
            && ctx.viewport_root_owner != Some(node as *const CallNode)
            && has_modifier(node, "background");
        let shapes = self.eval_node(node, ctx)?;
        if !background {
            return Ok(shapes);
        }
        if self.quality == Quality::Preview && !shapes.is_empty() {
            self.warn("Viewport background (%) geometry is omitted in preview because the mesh result contract has no background-layer metadata");
        }
        Ok(Vec::new())
    }

    pub(super) fn eval_prepared_nodes(
        &self,
        nodes: &'a [Statement],
        ctx: &Ctx<'a>,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let mut output = Vec::new();
        for node in nodes {
            if self.stable_viewport_disabled(node) {
                continue;
            }
            self.bump_ops(node_p(node))?;
            match node {
                Statement::Assign(assign) => {
                    if !self.is_stable() {
                        let value = self.eval_expression(&assign.value, ctx, 0)?;
                        ctx.env.borrow_mut().insert(assign.name.clone(), value);
                    }
                    continue;
                }
                Statement::Module(_) | Statement::Function(_) => continue,
                Statement::Directive(_) => continue,
                Statement::Call(call) => {
                    output.extend(self.eval_prepared_call(call, ctx)?);
                }
            }
            if output.len() > MAX_SHAPES {
                return Err(self.error(
                    node_p(node),
                    format!("Model exceeds the {} object limit", locale(MAX_SHAPES)),
                ));
            }
        }
        Ok(output)
    }

    pub(super) fn stable_viewport_disabled(&self, statement: &Statement) -> bool {
        self.is_stable()
            && matches!(statement, Statement::Call(call) if has_modifier(call, "disable"))
    }

    /// Top-level statement loop (synchronous; cancellation polled per
    /// statement like the TS cooperative checkpoint).
    pub(super) fn eval_top_level(
        &self,
        nodes: &'a [Statement],
        ctx: &Ctx<'a>,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        self.poll()?;
        let mut output = Vec::new();
        for node in nodes {
            self.poll()?;
            if self.stable_viewport_disabled(node) {
                continue;
            }
            self.bump_ops(node_p(node))?;
            match node {
                Statement::Assign(assign) => {
                    if !self.is_stable() {
                        let value = self.eval_expression(&assign.value, ctx, 0)?;
                        ctx.env.borrow_mut().insert(assign.name.clone(), value);
                    }
                    continue;
                }
                Statement::Module(_) | Statement::Function(_) => continue,
                Statement::Directive(_) => continue,
                Statement::Call(call) => output.extend(self.eval_prepared_call(call, ctx)?),
            }
            if output.len() > MAX_SHAPES {
                return Err(self.error(
                    node_p(node),
                    format!("Model exceeds the {} object limit", locale(MAX_SHAPES)),
                ));
            }
        }
        Ok(output)
    }

    pub(super) fn passed_call_children<'c>(&self, ctx: &'c Ctx<'a>) -> (&'a [Statement], Ctx<'a>) {
        match &ctx.call_children {
            Some(CallChildren::Passed(passed)) => {
                let child_ctx = Ctx {
                    env: env_of(passed.env.borrow().clone()),
                    stable_scope: passed.scope.clone(),
                    scope_visible_before: usize::MAX,
                    call_children: passed.continuation.clone(),
                    ..ctx.clone()
                };
                (passed.statements, child_ctx)
            }
            Some(CallChildren::List(statements)) => (statements, ctx.clone()),
            None => (&[], ctx.clone()),
        }
    }

    pub(super) fn arg(
        &self,
        node: &'a CallNode,
        name: &str,
        position: i64,
        fallback: Value<'a>,
        ctx: &Ctx<'a>,
    ) -> EvalResult<Value<'a>> {
        let expression = call_args(node)
            .iter()
            .find(|(key, _)| key == name)
            .or_else(|| {
                if position < 0 {
                    None
                } else {
                    call_args(node)
                        .iter()
                        .find(|(key, _)| key == &format!("_{position}"))
                }
            });
        match expression {
            Some((_, expression)) => self.eval_expression(expression, ctx, 0),
            None => Ok(fallback),
        }
    }

}
