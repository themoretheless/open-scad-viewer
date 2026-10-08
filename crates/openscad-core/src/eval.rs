//! OpenSCAD value evaluation for both repository language profiles.
//! Default evaluation accounts shape names and dimensions for diagnostics.
//! Opt-in recording emits a typed solid program for supported operations;
//! independent geometry kernels execute that program through the host adapter.
use crate::ast::*;
use crate::builtins::{self, BuiltinContext, BuiltinError};
use crate::lexer::TT;
use crate::value::index as index_value;
use crate::value::*;
use crate::{LanguageProfile, ParseError};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Mirrors `MAX_EVAL_DEPTH` in `openscadParser.ts`.
pub const MAX_EVAL_DEPTH: usize = 128;
/// Mirrors `MAX_EVAL_OPS` in `openscadParser.ts`.
pub const MAX_EVAL_OPS: u64 = 1_000_000;
/// Mirrors `MAX_VALUE_ELEMENTS` in `openscadParser.ts`.
pub const MAX_VALUE_ELEMENTS: usize = 1_000_000;
/// Mirrors `MAX_RANGE_ITEMS` in `openscadParser.ts`.
pub const MAX_RANGE_ITEMS: usize = 10_000;
/// Mirrors `MAX_EVALUATED_VALUE_UNITS` in `openscadParser.ts`.
pub const MAX_EVALUATED_VALUE_UNITS: u64 = 500_000;
/// Mirrors `MAX_SHAPES` in `openscadParser.ts`.
pub const MAX_SHAPES: usize = 1_000;
/// Mirrors `MAX_FN` in `openscadParser.ts`.
pub const MAX_FN: f64 = 256.0;

use crate::{MAX_EXPRESSION_DEPTH, MAX_SOURCE_LENGTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Preview,
    Full,
}

/// Host knobs for one evaluation. `should_abort` is the synchronous
/// cancellation flag (TS `shouldAbort` polled at statement/loop checkpoints).
pub struct EvaluatorOptions<'h> {
    pub record_geometry: bool,
    pub should_abort: Option<&'h dyn Fn() -> bool>,
    /// Unseeded `rands` stream. TS uses `Math.random`; the deterministic
    /// default here is a splitmix64 stream (seeded `rands` is exact either way).
    pub random: Option<&'h RefCell<dyn FnMut() -> f64>>,
    /// Host animation position exposed as `$t` (0..1).
    pub animation_time: f64,
    pub quality: Quality,
}

impl Default for EvaluatorOptions<'_> {
    fn default() -> Self {
        Self {
            record_geometry: false,
            should_abort: None,
            random: None,
            animation_time: 0.0,
            quality: Quality::Full,
        }
    }
}

/// Result of a successful evaluation.
pub struct Evaluation {
    pub geometry: Option<geometry_ops::solid_program::Program>,
    pub shapes: Vec<ShapeDescriptor>,
    pub warnings: Vec<String>,
    pub reduced: bool,
}

#[derive(Clone)]
enum CallChildren<'a> {
    List(&'a [Statement]),
    Passed(Rc<PassedCallChildren<'a>>),
}

struct PassedCallChildren<'a> {
    statements: &'a [Statement],
    env: Env<'a>,
    scope: Option<Rc<StableScope<'a>>>,
    continuation: Option<CallChildren<'a>>,
}

/// Immutable evaluation context, cloned along evaluation paths exactly like
/// the TS `{...ctx, ...}` spreads; shared mutable state lives in `Evaluator`.
#[derive(Clone)]
struct Ctx<'a> {
    env: Env<'a>,
    stable_scope: Option<Rc<StableScope<'a>>>,
    scope_visible_before: usize,
    call_children: Option<CallChildren<'a>>,
    function_stack: Rc<Vec<String>>,
    module_stack: Rc<Vec<String>>,
    depth: usize,
    viewport_root_locked: bool,
    viewport_root_owner: Option<*const CallNode>,
}

impl<'a> Ctx<'a> {
    fn with_env(&self, env: HashMap<String, Value<'a>>) -> Self {
        Self {
            env: env_of(env),
            ..self.clone()
        }
    }
    fn push_function(&self, name: &str) -> Rc<Vec<String>> {
        let mut stack = (*self.function_stack).clone();
        stack.push(name.to_string());
        Rc::new(stack)
    }
    fn push_module(&self, name: &str) -> Rc<Vec<String>> {
        let mut stack = (*self.module_stack).clone();
        stack.push(name.to_string());
        Rc::new(stack)
    }
}

/// Shared evaluator state (one per `evaluate` call).
pub struct Evaluator<'a> {
    record_geometry: bool,
    geometry: RefCell<Vec<geometry_ops::solid_program::Node>>,
    profiles: RefCell<Vec<geometry_ops::profile_program::Node>>,
    units: &'a [u16],
    profile: LanguageProfile,
    quality: Quality,
    ops: Cell<u64>,
    value_budget: Cell<u64>,
    value_meta: RefCell<HashMap<*const Vec<Value<'a>>, (u64, u32)>>,
    warnings: RefCell<Vec<String>>,
    functions: HashMap<String, &'a FunctionNode>,
    modules: HashMap<String, &'a ModuleNode>,
    reduced: Cell<bool>,
    should_abort: Option<&'a dyn Fn() -> bool>,
    random_host: Option<&'a RefCell<dyn FnMut() -> f64>>,
    animation_time: f64,
    random_state: Cell<u64>,
}

fn has_modifier(node: &CallNode, kind: &str) -> bool {
    node.viewport_modifiers.iter().any(|m| m.kind == kind)
}

/// Deterministic splitmix64 stream standing in for the host's unseeded
/// `Math.random` (TS only exposes it through unseeded `rands`, which is
/// nondeterministic there as well).
fn splitmix64(state: u64) -> (u64, f64) {
    let next = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = next;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    (next, (z >> 11) as f64 / 0x20_0000_0000_0000u64 as f64)
}

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

    fn store_geometry(
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
    fn emit_geometry(
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
    fn emit_profile(
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
    fn affine_geometry(
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
    fn is_stable(&self) -> bool {
        self.profile.is_stable()
    }

    fn error(&self, p: usize, message: impl Into<String>) -> EvalFailure {
        EvalFailure::Error(ParseError::new(p, message))
    }

    fn warn(&self, message: impl Into<String>) {
        let message = message.into();
        let mut warnings = self.warnings.borrow_mut();
        if !warnings.contains(&message) {
            warnings.push(message);
        }
    }

    fn poll(&self) -> EvalResult<()> {
        if let Some(abort) = self.should_abort
            && abort()
        {
            return Err(EvalFailure::Aborted);
        }
        Ok(())
    }

    fn bump_ops(&self, p: usize) -> EvalResult<()> {
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

    fn value_weight(&self, value: &Value<'a>) -> u64 {
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

    fn value_depth(&self, value: &Value<'a>) -> u32 {
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
    fn register_vector(
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

    fn register_string(&self, value: String, p: usize, label: &str) -> EvalResult<String> {
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

    fn semantics<'h>(&'h self, p: usize) -> SemanticsContext<'h, 'a> {
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

    fn resolve_stable_variable(
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

    fn stable_overlay_context(&self, ctx: &Ctx<'a>, env: HashMap<String, Value<'a>>) -> Ctx<'a> {
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

    fn enter_stable_statement_scope(
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

    fn eval_expression(
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

    fn finite_number(&self, value: &Value, p: usize, label: &str) -> EvalResult<f64> {
        match value.as_number() {
            Some(v) if v.is_finite() => Ok(v),
            _ => Err(self.error(p, format!("{label} must be a finite number"))),
        }
    }

    fn vector_value(&self, value: &Value, p: usize, label: &str) -> EvalResult<Vec<f64>> {
        let Some(items) = value.as_vector() else {
            return Err(self.error(p, format!("{label} must be a vector")));
        };
        let items = items.clone();
        items
            .iter()
            .map(|item| self.finite_number(item, p, label))
            .collect()
    }

    fn evaluate_sequential_bindings(
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

    fn stable_iterable(
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

    fn append_comprehension_values(
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

    fn eval_comprehension_element(
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

    fn eval_list_comprehension(
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

    fn lc_for_visit(
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

    fn eval_assert_expression(
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

    fn eval_echo_expression(
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

    // ------------------------------------------------------------------
    // Function calls
    // ------------------------------------------------------------------

    fn resolve_stable_expression_arguments(
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

    fn eval_function_call(
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

    fn invoke_user_function(
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

    fn compatibility_string(&self, value: &Value) -> String {
        match value {
            Value::Undef => String::new(),
            Value::Str(s) => s.to_string(),
            Value::Number(v) => js_number_to_string(*v),
            Value::Bool(v) => if *v { "true" } else { "false" }.to_string(),
            other => format_value(other),
        }
    }

    fn eval_dxf_query_builtin(
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

    fn eval_builtin(
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

    // ------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------

    fn eval_nodes(
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

    fn eval_prepared_call(
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

    fn eval_prepared_nodes(
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

    fn stable_viewport_disabled(&self, statement: &Statement) -> bool {
        self.is_stable()
            && matches!(statement, Statement::Call(call) if has_modifier(call, "disable"))
    }

    /// Top-level statement loop (synchronous; cancellation polled per
    /// statement like the TS cooperative checkpoint).
    fn eval_top_level(
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

    fn passed_call_children<'c>(&self, ctx: &'c Ctx<'a>) -> (&'a [Statement], Ctx<'a>) {
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

    fn arg(
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

    fn eval_node(&self, node: &'a CallNode, parent: &Ctx<'a>) -> EvalResult<Vec<ShapeDescriptor>> {
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
            "linear_extrude" => {
                let evaluated = if self.is_stable() {
                    Some(self.bind_stable_module(
                        node,
                        &ctx,
                        &["height", "center", "convexity", "twist", "slices", "scale"],
                        &["$fn", "$fa", "$fs"],
                    )?)
                } else {
                    None
                };
                let sections = self.eval_nodes(&node.children, &ctx, true)?;
                if self.record_geometry && sections.is_empty() {
                    return self.emit_geometry(
                        "linear_extrude",
                        geometry_ops::solid_program::Node::Empty,
                        node.p,
                    );
                }
                let sections = self.boolean_shapes(sections, "union", node.p, "union")?;
                if sections.is_empty() {
                    return Ok(Vec::new());
                }
                if sections[0].dimension != 2 {
                    return Err(self.error(node.p, "linear_extrude() requires 2D children"));
                }
                if evaluated.is_none() {
                    let height = self.finite_number(
                        &self.arg(node, "height", 0, Value::Number(1.0), &ctx)?,
                        node.p,
                        "extrusion height",
                    )?;
                    if height <= 0.0 {
                        return Err(self.error(node.p, "linear_extrude() height must be positive"));
                    }
                    self.finite_number(
                        &self.arg(node, "twist", -1, Value::Number(0.0), &ctx)?,
                        node.p,
                        "extrusion twist",
                    )?;
                    let slices = self.finite_number(
                        &self.arg(node, "slices", -1, Value::Number(0.0), &ctx)?,
                        node.p,
                        "extrusion slices",
                    )?;
                    let _ = slices.trunc().clamp(0.0, 512.0);
                    let raw_scale = self.arg(
                        node,
                        "scale",
                        -1,
                        Value::vector(vec![Value::Number(1.0), Value::Number(1.0)]),
                        &ctx,
                    )?;
                    match &raw_scale {
                        Value::Vector(_) => {
                            self.vector_value(&raw_scale, node.p, "extrusion scale")?;
                        }
                        _ => {
                            self.finite_number(&raw_scale, node.p, "extrusion scale")?;
                        }
                    }
                    self.arg(node, "center", -1, Value::Bool(false), &ctx)?;
                }
                if self.record_geometry {
                    if let Some(values) = evaluated {
                        let fragments = self.stable_fragment_specials(&values, &ctx)?;
                        return self.record_stable_extrusion(sections, &values, fragments, node.p);
                    }
                }
                Ok(vec![descriptor("linear_extrude", 3)])
            }
            "rotate_extrude" => {
                let evaluated = if self.is_stable() {
                    Some(self.bind_stable_module(
                        node,
                        &ctx,
                        &["angle", "convexity"],
                        &["$fn", "$fa", "$fs"],
                    )?)
                } else {
                    None
                };
                let sections = self.eval_nodes(&node.children, &ctx, true)?;
                let sections = self.boolean_shapes(sections, "union", node.p, "union")?;
                if sections.is_empty() {
                    return Ok(Vec::new());
                }
                if sections[0].dimension != 2 {
                    return Err(self.error(node.p, "rotate_extrude() requires 2D children"));
                }
                if evaluated.is_none() {
                    let _angle = self.finite_number(
                        &self.arg(node, "angle", -1, Value::Number(360.0), &ctx)?,
                        node.p,
                        "revolve angle",
                    )?;
                }
                if self.record_geometry {
                    if let Some(values) = evaluated {
                        let angle = values.get("angle").and_then(Value::as_number);
                        let plan = crate::extrusion_plan::revolution_parameters(
                            angle,
                            values.contains_key("angle"),
                            1.,
                            1.,
                        );
                        if plan.angle_defaulted {
                            self.warn(
                                "Invalid rotate_extrude angle was replaced with 360".to_string(),
                            );
                        }
                        if plan.empty {
                            return self.emit_geometry(
                                "rotate_extrude",
                                geometry_ops::solid_program::Node::Empty,
                                node.p,
                            );
                        }
                        let root = sections[0].profile.ok_or_else(|| {
                            self.error(node.p, "Unsupported native revolution profile")
                        })?;
                        let profile = geometry_ops::profile_program::Program::from_roots(
                            &self.profiles.borrow(),
                            &[root],
                        )
                        .map_err(|error| self.error(node.p, &error.to_string()))?;
                        let fragments = self.stable_fragment_specials(&values, &ctx)?;
                        return self.emit_geometry(
                            "rotate_extrude",
                            geometry_ops::solid_program::Node::RevolveProfile {
                                profile,
                                angle: plan.angle,
                                segments: 0,
                                fragment_policy: Some(geometry_ops::fragment_resolution::Policy {
                                    fragments,
                                    maximum: if self.quality == Quality::Preview {
                                        48
                                    } else {
                                        256
                                    },
                                }),
                            },
                            node.p,
                        );
                    }
                }
                Ok(vec![descriptor("rotate_extrude", 3)])
            }
            "dxf_linear_extrude" => {
                self.require_stable(node)?;
                self.compatibility_deprecation(node, "linear_extrude()");
                let values = self.bind_stable_module(
                    node,
                    &ctx,
                    &[
                        "file", "layer", "height", "origin", "scale", "center", "twist", "slices",
                    ],
                    &["convexity", "$fn", "$fa", "$fs"],
                )?;
                let file = values.get("file").cloned().unwrap_or(Value::Undef);
                if let Value::Str(file) = &file
                    && !file.is_empty()
                {
                    return Err(self.import_project_required(node, "dxf_linear_extrude"));
                }
                let sections = self.eval_nodes(&node.children, &ctx, true)?;
                let sections =
                    self.boolean_shapes(sections, "union", node.p, "dxf_linear_extrude")?;
                if sections.is_empty() {
                    return Ok(Vec::new());
                }
                if sections[0].dimension != 2 {
                    return Err(self.error(node.p, "dxf_linear_extrude() requires 2D children"));
                }
                Ok(vec![descriptor("dxf_linear_extrude", 3)])
            }
            "dxf_rotate_extrude" => {
                self.require_stable(node)?;
                self.compatibility_deprecation(node, "rotate_extrude()");
                let values = self.bind_stable_module(
                    node,
                    &ctx,
                    &["file", "layer", "origin", "scale"],
                    &["convexity", "angle", "$fn", "$fa", "$fs"],
                )?;
                let file = values.get("file").cloned().unwrap_or(Value::Undef);
                let file_text = self.compatibility_string(&file);
                if !file_text.is_empty() {
                    return Err(self.import_project_required(node, "dxf_rotate_extrude"));
                }
                let sections = self.eval_nodes(&node.children, &ctx, true)?;
                let sections =
                    self.boolean_shapes(sections, "union", node.p, "dxf_rotate_extrude")?;
                if sections.is_empty() {
                    return Ok(Vec::new());
                }
                if sections[0].dimension != 2 {
                    return Err(self.error(node.p, "dxf_rotate_extrude() requires 2D children"));
                }
                Ok(vec![descriptor("dxf_rotate_extrude", 3)])
            }
            "projection" => {
                if self.is_stable() {
                    let values = self.bind_stable_module(node, &ctx, &["cut", "convexity"], &[])?;
                    let children = self.eval_nodes(&node.children, &ctx, true)?;
                    let solids: Vec<_> = children.iter().filter(|s| s.dimension == 3).collect();
                    if solids.len() != children.len() {
                        self.warn("projection() ignored non-3D child geometry");
                    }
                    if solids.is_empty() {
                        return Ok(Vec::new());
                    }
                    if self.record_geometry {
                        let shapes = solids.into_iter().cloned().collect();
                        let union = self.boolean_shapes(shapes, "union", node.p, "projection")?;
                        let input = union[0].geometry.ok_or_else(|| self.error(node.p,"Unsupported native projection input"))?;
                        let solid = geometry_ops::solid_program::Program::from_roots(&self.geometry.borrow(), &[input])
                            .map_err(|error| self.error(node.p,error.message))?;
                        return self.emit_profile("projection", geometry_ops::profile_program::Node::Projection {
                            solid: Box::new(solid), cut: matches!(values.get("cut"),Some(Value::Bool(true))),
                        }, node.p);
                    }
                    return Ok(vec![descriptor("projection", 2)]);
                }
                let cut = self.arg(node, "cut", 0, Value::Bool(false), &ctx)?;
                let _ = cut;
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                let mut output = Vec::new();
                for shape in shapes {
                    if shape.dimension != 3 {
                        return Err(self.error(node.p, "projection() requires 3D children"));
                    }
                    output.push(descriptor("projection", 2));
                }
                Ok(output)
            }
            "offset" => {
                if self.is_stable() {
                    let mut values = HashMap::new();
                    let r_expression = call_args(node)
                        .iter()
                        .find(|(key, _)| key == "r")
                        .or_else(|| call_args(node).iter().find(|(key, _)| key == "_0"));
                    for (name, expression) in [
                        ("r", r_expression.map(|(_, expression)| expression)),
                        (
                            "delta",
                            call_args(node)
                                .iter()
                                .find(|(key, _)| key == "delta")
                                .map(|(_, expression)| expression),
                        ),
                        (
                            "chamfer",
                            call_args(node)
                                .iter()
                                .find(|(key, _)| key == "chamfer")
                                .map(|(_, expression)| expression),
                        ),
                    ]
                    .into_iter()
                    .filter_map(|(name, expression)| {
                        expression.map(|expression| (name, expression))
                    }) {
                        values.insert(name.to_string(), self.eval_expression(expression, &ctx, 0)?);
                    }
                    let plan = crate::offset_plan::resolve(
                        values.get("r").and_then(Value::as_number),
                        values.get("delta").and_then(Value::as_number),
                        matches!(values.get("chamfer"), Some(Value::Bool(true))),
                    );
                    let segments =
                        if self.record_geometry && plan.join == crate::offset_plan::Join::Round {
                            for name in ["$fn", "$fa", "$fs"] {
                                if let Some((_, expression)) =
                                    call_args(node).iter().find(|(key, _)| key == name)
                                {
                                    values.insert(
                                        name.to_string(),
                                        self.eval_expression(expression, &ctx, 0)?,
                                    );
                                }
                            }
                            self.stable_fragment_count(&values, &ctx, plan.distance.abs())?
                        } else {
                            8
                        };
                    let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                    let mut output = Vec::new();
                    for shape in shapes {
                        if shape.dimension != 2 {
                            return Err(self.error(node.p, "offset() requires 2D children"));
                        }
                        if self.record_geometry {
                            if !plan.distance.is_finite() {
                                return Err(self.error(node.p, "Invalid offset"));
                            }
                            let input = shape.profile.ok_or_else(|| {
                                self.error(node.p, "Unsupported native offset profile")
                            })?;
                            output.extend(self.emit_profile(
                                "offset",
                                geometry_ops::profile_program::Node::Offset {
                                    input,
                                    distance: plan.distance,
                                    join: plan.join,
                                    segments,
                                },
                                node.p,
                            )?);
                        } else {
                            output.push(descriptor("offset", 2));
                        }
                    }
                    return Ok(output);
                }
                let distance = {
                    let delta = self.arg(node, "delta", 0, Value::Number(1.0), &ctx)?;
                    let raw = self.arg(node, "r", 0, delta, &ctx)?;
                    self.finite_number(&raw, node.p, "offset distance")?
                };
                let _ = distance;
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                let mut output = Vec::new();
                for shape in shapes {
                    if shape.dimension != 2 {
                        return Err(self.error(node.p, "offset() requires 2D children"));
                    }
                    output.push(descriptor("offset", 2));
                }
                Ok(output)
            }
            "group" => {
                let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                if self.is_stable() {
                    return self.boolean_shapes(shapes, "union", node.p, "group");
                }
                Ok(shapes)
            }
            "render" => {
                if self.is_stable() {
                    self.bind_stable_module(node, &ctx, &["convexity"], &[])?;
                    let shapes = self.eval_nodes(&node.children, &ctx, true)?;
                    return self.boolean_shapes(shapes, "union", node.p, "render");
                }
                self.eval_nodes(&node.children, &ctx, true)
            }
            "if" => {
                let condition = self.arg(node, "_0", 0, Value::Bool(false), &ctx)?;
                let branch = if truthy(&condition) {
                    &node.children
                } else {
                    &node.alternative
                };
                self.eval_nodes(branch, &ctx, true)
            }
            "let" => {
                if self.is_stable() {
                    let env = self.evaluate_sequential_bindings(&node.call_arguments, &ctx, 0)?;
                    let overlay = self.stable_overlay_context(&ctx, env);
                    return self.eval_nodes(&node.children, &overlay, false);
                }
                let mut env = ctx.env.borrow().clone();
                for (name, expression) in call_args(node) {
                    if name.starts_with('_') {
                        continue;
                    }
                    let value = self.eval_expression(expression, &ctx, 0)?;
                    env.insert(name.clone(), value);
                }
                self.eval_nodes(&node.children, &ctx.with_env(env), false)
            }
            "for" => self.eval_for(node, &ctx, "for"),
            "intersection_for" => {
                self.require_stable(node)?;
                let shapes = self.eval_for(node, &ctx, "intersection_for")?;
                self.boolean_shapes(shapes, "intersection", node.p, "intersection_for")
            }
            "echo" => {
                self.require_stable(node)?;
                let mut values = Vec::new();
                for (name, expression) in call_args(node) {
                    let value = format_value(&self.eval_expression(expression, &ctx, 0)?);
                    let kind = call_arg_kinds(node)
                        .iter()
                        .find(|(key, _)| key == name)
                        .map(|(_, kind)| *kind);
                    values.push(if kind == Some("named") {
                        format!("{name} = {value}")
                    } else {
                        value
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
                self.eval_nodes(&node.children, &ctx, true)
            }
            "children" => {
                let (all, child_context) = self.passed_call_children(&ctx);
                if self.is_stable() {
                    let selector = call_args(node).iter().find(|(key, _)| key == "_0");
                    let selected = match selector {
                        None => None,
                        Some((_, expression)) => {
                            let value = self.eval_expression(expression, &ctx, 0)?;
                            Some(self.resolve_children_selection(Some(&value), all.len())?)
                        }
                    };
                    match selected {
                        None => self.eval_nodes(all, &child_context, true),
                        Some(indices) => {
                            let mut output = Vec::new();
                            for index in indices {
                                if let Some(statement) = all.get(index) {
                                    output.extend(self.eval_nodes(
                                        std::slice::from_ref(statement),
                                        &child_context,
                                        true,
                                    )?);
                                }
                            }
                            Ok(output)
                        }
                    }
                } else {
                    let index = self.arg(node, "_0", 0, Value::Undef, &ctx)?;
                    if index.is_undef() {
                        return self.eval_nodes(all, &child_context, true);
                    }
                    let index = self
                        .finite_number(&index, node.p, "children index")?
                        .trunc();
                    if index < 0.0 {
                        return Ok(Vec::new());
                    }
                    match all.get(index as usize) {
                        None => Ok(Vec::new()),
                        Some(statement) => {
                            self.eval_nodes(std::slice::from_ref(statement), &child_context, true)
                        }
                    }
                }
            }
            "child" => {
                self.require_stable(node)?;
                self.compatibility_deprecation(node, "children()");
                let index_value = match node.call_arguments.first() {
                    None => Value::Number(0.0),
                    Some(argument) => self.eval_expression(&argument.value, &ctx, 0)?,
                };
                let index = match index_value.as_number() {
                    Some(v) if v.is_finite() => v.trunc(),
                    _ => 0.0,
                };
                let index = if index == 0.0 { 0.0 } else { index };
                if index < 0.0 {
                    self.warn(format!(
                        "Negative child index ({}) is not allowed",
                        js_number_to_string(index)
                    ));
                    return Ok(Vec::new());
                }
                let (all, child_context) = self.passed_call_children(&ctx);
                let Some(statement) = all.get(index as usize) else {
                    if ctx.call_children.is_some() {
                        self.warn(format!(
                            "Child index ({}) out of bounds ({} children)",
                            js_number_to_string(index),
                            all.len()
                        ));
                    }
                    return Ok(Vec::new());
                };
                self.eval_nodes(std::slice::from_ref(statement), &child_context, true)
            }
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

    fn require_stable(&self, node: &CallNode) -> EvalResult<()> {
        if !self.is_stable() {
            return Err(self.error(
                node.p,
                format!("Unsupported geometry operation {}()", node.name),
            ));
        }
        Ok(())
    }

    fn import_project_required(&self, node: &CallNode, display_name: &str) -> EvalFailure {
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

    fn compatibility_deprecation(&self, node: &CallNode, replacement: &str) {
        if node.name == "child" {
            self.warn("child() will be removed in future releases. Use children() instead.");
        } else {
            self.warn(format!(
                "The {}() module will be removed in future releases. Use {replacement} instead.",
                node.name
            ));
        }
    }

    fn warn_ignored_primitive_children(&self, node: &CallNode) {
        if !node.children.is_empty() {
            self.warn(format!("{}() ignores child geometry", node.name));
        }
    }

    /// `evaluateStableBuiltinModuleArguments`: bind a stable built-in module
    /// without re-evaluating argument expressions; every supplied expression is
    /// evaluated exactly once, including ignored arguments whose echo/assert
    /// effects remain observable.
    fn bind_stable_module(
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

    fn indexed_rows(&self, values: &HashMap<String,Value<'a>>, name:&str) -> Vec<Vec<Option<f64>>> {
        match values.get(name) {
            Some(Value::Vector(rows)) => rows.iter().map(|row| match row {
                Value::Vector(values)=>values.iter().map(Value::as_number).collect(),
                _=>Vec::new(),
            }).collect(),
            _=>Vec::new(),
        }
    }
    fn record_stable_polyhedron(&self, values:&HashMap<String,Value<'a>>, node:&'a CallNode) -> EvalResult<Vec<ShapeDescriptor>> {
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
    fn record_stable_polygon(
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

    fn record_stable_extrusion(
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

    fn stable_fragment_specials(
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

    fn stable_fragment_count(
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

    fn segments(
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

    fn parse_legacy_color(&self, value: &Value, p: usize) -> EvalResult<()> {
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

    fn stable_vector_geometry(
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

    fn stable_matrix_geometry(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
        matrix: [f64; 16],
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let shapes = self.transform_stable_children(node, ctx)?;
        self.record_stable_matrix(shapes, matrix, &node.name, node.p)
    }

    fn record_stable_matrix(
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

    fn transform_stable_children(
        &self,
        node: &'a CallNode,
        ctx: &Ctx<'a>,
    ) -> EvalResult<Vec<ShapeDescriptor>> {
        let shapes = self.eval_nodes(&node.children, ctx, true)?;
        self.boolean_shapes(shapes, "union", node.p, &node.name)
    }

    fn boolean_shapes(
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

    fn difference_children(
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

    fn eval_for(
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

    fn for_visit_stable(
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

    fn eval_assert_statement(
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

    fn bind_assert_arguments(&self, node: &'a CallNode) -> EvalResult<BoundAssert<'a>> {
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

    fn eval_user_module(
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
    fn resolve_children_selection(
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

    fn slice_units(&self, start: usize, end: usize) -> String {
        let end = end.min(self.units.len());
        let start = start.min(end);
        String::from_utf16_lossy(&self.units[start..end])
    }

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

struct BoundAssert<'a> {
    condition: &'a Expr,
    condition_text: String,
    message: Option<&'a Expr>,
}

fn descriptor(name: &str, dimension: u8) -> ShapeDescriptor {
    ShapeDescriptor {
        name: name.to_string(),
        dimension,
        geometry: None,
        profile: None,
    }
}

fn viewport_root_activates(node: &CallNode, shapes: &[ShapeDescriptor]) -> bool {
    if !shapes.is_empty() {
        return true;
    }
    // OpenSCAD does not create an empty CSG container for these transparent
    // effect/branch calls.
    !["if", "assert", "echo", "children", "child"].contains(&node.name.as_str())
}

fn call_args(node: &CallNode) -> &[(String, Expr)] {
    &node.args
}

fn call_arg_kinds(node: &CallNode) -> &[(String, &'static str)] {
    &node.arg_kinds
}

fn expr_p(expr: &Expr) -> usize {
    match expr {
        Expr::Literal { p, .. }
        | Expr::Identifier { p, .. }
        | Expr::Vector { p, .. }
        | Expr::Range { p, .. }
        | Expr::Unary { p, .. }
        | Expr::Binary { p, .. }
        | Expr::Ternary { p, .. }
        | Expr::Function { p, .. }
        | Expr::Call { p, .. }
        | Expr::Index { p, .. }
        | Expr::Member { p, .. }
        | Expr::Let { p, .. }
        | Expr::Assert { p, .. }
        | Expr::Echo { p, .. }
        | Expr::LcFor { p, .. }
        | Expr::LcForC { p, .. }
        | Expr::LcIf { p, .. }
        | Expr::LcLet { p, .. }
        | Expr::LcEach { p, .. } => *p,
    }
}

fn node_p(node: &Statement) -> usize {
    match node {
        Statement::Call(call) => call.p,
        Statement::Assign(assign) => assign.p,
        Statement::Module(module) => module.p,
        Statement::Function(function) => function.p,
        Statement::Directive(directive) => directive.p,
    }
}

fn overlay_dynamic_variables<'a>(
    target: &mut HashMap<String, Value<'a>>,
    source: &HashMap<String, Value<'a>>,
) {
    for (name, value) in source {
        if name.starts_with('$') {
            target.insert(name.clone(), value.clone());
        }
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Vector(items) => format!(
            "[{}]",
            items
                .iter()
                .map(value_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Function(_) => "function(...)".to_string(),
        Value::Undef => "undef".to_string(),
        Value::Bool(v) => if *v { "true" } else { "false" }.to_string(),
        Value::Number(v) => js_number_to_string(*v),
        Value::Str(s) => s.to_string(),
        Value::Range { .. } => format_value(value),
    }
}

fn collect_modules<'a>(nodes: &'a [Statement], modules: &mut HashMap<String, &'a ModuleNode>) {
    for node in nodes {
        match node {
            Statement::Module(module) => {
                modules.insert(module.name.clone(), module);
            }
            Statement::Call(call) => {
                collect_modules(&call.children, modules);
                collect_modules(&call.alternative, modules);
            }
            _ => {}
        }
    }
}

fn collect_functions<'a>(
    nodes: &'a [Statement],
    functions: &mut HashMap<String, &'a FunctionNode>,
) {
    for node in nodes {
        match node {
            Statement::Function(function) => {
                functions.insert(function.name.clone(), function);
            }
            Statement::Call(call) => {
                collect_functions(&call.children, functions);
                collect_functions(&call.alternative, functions);
            }
            Statement::Module(module) => collect_functions(&module.children, functions),
            _ => {}
        }
    }
}

static CSS_COLORS: &[&str] = &[
    "red", "green", "blue", "yellow", "cyan", "magenta", "white", "black", "orange", "gray",
    "grey", "pink", "purple", "brown", "lime", "navy", "teal",
];

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    (hex.len() == 6 || hex.len() == 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Evaluate a parsed program (migration stage 2, shape-descriptor output).
pub fn evaluate_program<'a>(
    statements: &'a [Statement],
    units: &'a [u16],
    profile: LanguageProfile,
    options: EvaluatorOptions<'a>,
) -> EvalResult<Evaluation> {
    Evaluator::new(statements, units, profile, options).evaluate(statements)
}

/// Convenience wrapper: compile and evaluate one source string.
pub fn evaluate_source(
    source: &str,
    profile: LanguageProfile,
    options: EvaluatorOptions,
) -> Result<Evaluation, crate::Diagnostic> {
    let units: Vec<u16> = source.encode_utf16().collect();
    if units.len() > MAX_SOURCE_LENGTH {
        return Err(ParseError::new(0, "Source exceeds 250,000 characters").resolve(&units));
    }
    let statements = crate::compile_units(&units, profile)?;
    match evaluate_program(&statements, &units, profile, options) {
        Ok(evaluation) => Ok(evaluation),
        Err(EvalFailure::Error(error)) => Err(error.resolve(&units)),
        Err(EvalFailure::Aborted) => Err(ParseError::new(0, "Evaluation aborted").resolve(&units)),
        Err(EvalFailure::ViewportRoot(_)) => unreachable!("root selection is caught at top level"),
    }
}

#[cfg(test)]
mod profile_recording_tests {
    use super::*;
    #[test]
    fn profile_booleans_keep_operand_order_without_solid_nodes() {
        let evaluator = Evaluator::new(
            &[],
            &[],
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        );
        let first = evaluator
            .emit_profile(
                "square",
                geometry_ops::profile_program::Node::Rectangle {
                    size: [4.; 2],
                    center: true,
                },
                0,
            )
            .unwrap();
        let second = evaluator
            .emit_profile(
                "square",
                geometry_ops::profile_program::Node::Rectangle {
                    size: [2.; 2],
                    center: true,
                },
                0,
            )
            .unwrap();
        let shapes = evaluator
            .boolean_shapes([first, second].concat(), "difference", 0, "difference")
            .unwrap();
        assert_eq!(shapes[0].dimension, 2);
        assert_eq!(shapes[0].profile, Some(2));
        assert!(shapes[0].geometry.is_none());
        assert!(evaluator.geometry.borrow().is_empty());
        assert_eq!(
            evaluator.profiles.borrow()[2],
            geometry_ops::profile_program::Node::Boolean {
                operation: geometry_ops::solid_program::Boolean::Difference,
                inputs: vec![0, 1]
            }
        );
    }
}

#[cfg(test)]
mod profile_matrix_tests {
    use super::*;
    #[test]
    fn xy_matrix_ignores_z_collapse_and_preserves_profile_dimension() {
        let evaluator = Evaluator::new(
            &[],
            &[],
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        );
        let shapes = evaluator
            .emit_profile(
                "square",
                geometry_ops::profile_program::Node::Rectangle {
                    size: [2.; 2],
                    center: true,
                },
                0,
            )
            .unwrap();
        let matrix = [
            2., 0., 0., 0., 0., 3., 0., 0., 0., 0., 0., 0., 4., 5., 6., 1.,
        ];
        let shapes = evaluator
            .record_stable_matrix(shapes, matrix, "scale", 0)
            .unwrap();
        assert_eq!(shapes[0].dimension, 2);
        assert_eq!(
            evaluator.profiles.borrow()[1],
            geometry_ops::profile_program::Node::Transform {
                input: 0,
                matrix: [[2., 0., 4.], [0., 3., 5.], [0., 0., 1.]]
            }
        );
        let mut collapsed = matrix;
        collapsed[0] = 0.;
        let empty = evaluator
            .record_stable_matrix(shapes, collapsed, "scale", 0)
            .unwrap();
        assert_eq!(empty[0].dimension, 2);
        assert_eq!(
            evaluator.profiles.borrow()[2],
            geometry_ops::profile_program::Node::Empty
        );
        assert!(evaluator.geometry.borrow().is_empty());
    }
}

#[cfg(test)]
mod extrusion_recording_tests {
    use super::*;
    #[test]
    fn records_profile_and_deferred_policy_with_shared_parameters() {
        let evaluator = Evaluator::new(
            &[],
            &[],
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        );
        evaluator
            .emit_profile(
                "unused",
                geometry_ops::profile_program::Node::Circle {
                    radius: 9.,
                    segments: 12,
                },
                0,
            )
            .unwrap();
        let sections = evaluator
            .emit_profile(
                "square",
                geometry_ops::profile_program::Node::Rectangle {
                    size: [2.; 2],
                    center: true,
                },
                0,
            )
            .unwrap();
        let values = HashMap::from([
            ("height".to_string(), Value::Number(3.)),
            ("twist".to_string(), Value::Number(360.)),
            ("slices".to_string(), Value::Number(3.9)),
        ]);
        let shapes = evaluator
            .record_stable_extrusion(sections, &values, [12., 12., 2.], 0)
            .unwrap();
        assert_eq!(shapes[0].geometry, Some(0));
        assert_eq!(shapes[0].dimension, 3);
        let nodes = evaluator.geometry.borrow();
        let geometry_ops::solid_program::Node::ExtrudeProfile {
            profile,
            slice_policy,
            height,
            twist,
            ..
        } = &nodes[0]
        else {
            panic!("Expected profile extrusion")
        };
        assert_eq!(*height, 3.);
        assert_eq!(*twist, 360.);
        assert_eq!(profile.roots, vec![0]);
        assert_eq!(profile.nodes.len(), 1);
        assert_eq!(slice_policy.as_ref().unwrap().explicit, Some(3.));
        assert_eq!(slice_policy.as_ref().unwrap().fragments, [12., 12., 2.]);
    }
}

#[cfg(test)]
mod polygon_recording_tests {
    use super::*;
    fn vector(values: Vec<Value<'static>>) -> Value<'static> {
        Value::vector(values)
    }
    #[test]
    fn records_even_odd_outlines_and_preserves_conversion_warnings() {
        let evaluator = Evaluator::new(
            &[],
            &[],
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        );
        let points = vector(vec![
            vector(vec![Value::Number(0.), Value::Number(0.)]),
            vector(vec![Value::Number(2.), Value::Number(0.)]),
            vector(vec![Value::Number(0.), Value::Number(2.)]),
        ]);
        let values = HashMap::from([
            ("points".to_string(), points),
            (
                "paths".to_string(),
                vector(vec![vector(vec![
                    Value::Number(0.),
                    Value::Number(1.),
                    Value::Number(99.),
                    Value::Number(2.),
                ])]),
            ),
        ]);
        let shapes = evaluator.record_stable_polygon(&values, 0).unwrap();
        assert_eq!(shapes[0].dimension, 2);
        assert_eq!(
            evaluator.profiles.borrow()[0],
            geometry_ops::profile_program::Node::EvenOddRings(vec![vec![
                [0., 0.],
                [2., 0.],
                [0., 2.]
            ]])
        );
        assert_eq!(
            evaluator.warnings.borrow()[0],
            "polygon skipped a path entry whose point index is outside the point vector"
        );
        let invalid = HashMap::from([(
            "points".to_string(),
            vector(vec![vector(vec![Value::Number(0.)])]),
        )]);
        evaluator.record_stable_polygon(&invalid, 0).unwrap();
        assert_eq!(
            evaluator.profiles.borrow()[1],
            geometry_ops::profile_program::Node::Empty
        );
        assert_eq!(
            evaluator.warnings.borrow()[1],
            "polygon produced no outlines because a point failed exact vec2 conversion"
        );
    }
}
