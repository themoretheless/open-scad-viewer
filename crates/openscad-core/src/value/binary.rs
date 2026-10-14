use super::*;

/// `openScadBinary` (short-circuiting of && / || stays with the caller).
pub fn binary<'a>(
    operator: TT,
    left: &Value<'a>,
    right: &Value<'a>,
    ctx: &mut SemanticsContext<'_, 'a>,
) -> EvalResult<Value<'a>> {
    match operator {
        TT::EqEq => return Ok(Value::Bool(deep_equal(left, right))),
        TT::NotEq => return Ok(Value::Bool(!deep_equal(left, right))),
        TT::And => return Ok(Value::Bool(truthy(left) && truthy(right))),
        TT::Or => return Ok(Value::Bool(truthy(left) || truthy(right))),
        TT::Lt | TT::Gt | TT::LtEq | TT::GtEq => {
            let Some(comparison) = compare_values(left, right) else {
                (ctx.warn)(format!(
                    "Undefined operation ({} {} {})",
                    type_name(left),
                    token(operator),
                    type_name(right)
                ));
                return Ok(Value::Undef);
            };
            return Ok(Value::Bool(match operator {
                TT::Lt => comparison < 0.0,
                TT::Gt => comparison > 0.0,
                TT::LtEq => comparison <= 0.0,
                _ => comparison >= 0.0,
            }));
        }
        _ => {}
    }
    match operator {
        TT::Plus | TT::Minus => {
            if let (Value::Number(a), Value::Number(b)) = (left, right) {
                return Ok(Value::Number(if operator == TT::Plus {
                    a + b
                } else {
                    a - b
                }));
            }
            if let (Value::Vector(l), Value::Vector(r)) = (left, right) {
                let (l, r) = (l.clone(), r.clone());
                let length = l.len().min(r.len());
                let mut out = Vec::with_capacity(length);
                for index in 0..length {
                    out.push(binary(operator, &l[index], &r[index], ctx)?);
                }
                return Ok(Value::vector(ctx.register(out, "vector arithmetic")?));
            }
            Ok(ctx.undefined_operation(operator, left, right))
        }
        TT::Star => multiply(left, right, ctx),
        TT::Slash => {
            if let (Value::Number(a), Value::Number(b)) = (left, right) {
                return Ok(Value::Number(a / b));
            }
            if let (Value::Vector(l), Value::Number(_)) = (left, right) {
                let l = l.clone();
                let mut out = Vec::with_capacity(l.len());
                for item in l.iter() {
                    out.push(binary(operator, item, right, ctx)?);
                }
                return Ok(Value::vector(ctx.register(out, "vector division")?));
            }
            if let (Value::Number(_), Value::Vector(r)) = (left, right) {
                let r = r.clone();
                let mut out = Vec::with_capacity(r.len());
                for item in r.iter() {
                    out.push(binary(operator, left, item, ctx)?);
                }
                return Ok(Value::vector(ctx.register(out, "vector division")?));
            }
            Ok(ctx.undefined_operation(operator, left, right))
        }
        TT::Percent | TT::Caret => {
            if let (Value::Number(a), Value::Number(b)) = (left, right) {
                return Ok(Value::Number(if operator == TT::Percent {
                    a % b
                } else {
                    a.powf(*b)
                }));
            }
            Ok(ctx.undefined_operation(operator, left, right))
        }
        _ => Ok(ctx.undefined_operation(operator, left, right)),
    }
}

/// `openScadIndex` (numeric indices truncate toward zero; strings index UTF-16
/// code units like the JS host).
pub fn index<'a>(
    value: &Value<'a>,
    index_value: &Value<'a>,
    ctx: &mut SemanticsContext<'_, 'a>,
) -> EvalResult<Value<'a>> {
    let Some(raw) = index_value.as_number() else {
        (ctx.warn)(format!("Undefined index {}", format_value(index_value)));
        return Ok(Value::Undef);
    };
    if !raw.is_finite() {
        (ctx.warn)(format!("Undefined index {}", format_value(index_value)));
        return Ok(Value::Undef);
    }
    let index = raw.trunc();
    if index < 0.0 {
        return Ok(Value::Undef);
    }
    let index = index as usize;
    match value {
        Value::Vector(items) => Ok(items.get(index).cloned().unwrap_or(Value::Undef)),
        // Stable profile indexes strings by code point (`Array.from`).
        Value::Str(s) => Ok(s
            .chars()
            .nth(index)
            .map_or(Value::Undef, |ch| Value::string(ch.to_string()))),
        Value::Range { start, step, end } => Ok(match index {
            0 => Value::Number(*start),
            1 => Value::Number(*step),
            2 => Value::Number(*end),
            _ => Value::Undef,
        }),
        other => {
            (ctx.warn)(format!("Undefined index operation on {}", type_name(other)));
            Ok(Value::Undef)
        }
    }
}

/// `openScadMember` (`.x` / `.y` / `.z` on vectors).
pub fn member<'a>(
    value: &Value<'a>,
    name: &str,
    ctx: &mut SemanticsContext<'_, 'a>,
) -> EvalResult<Value<'a>> {
    if let Value::Vector(items) = value {
        let index = match name {
            "x" => Some(0),
            "y" => Some(1),
            "z" => Some(2),
            _ => None,
        };
        if let Some(index) = index {
            return Ok(items.get(index).cloned().unwrap_or(Value::Undef));
        }
    }
    (ctx.warn)(format!("{} has no member {}", type_name(value), name));
    Ok(Value::Undef)
}

#[path = "formatting.rs"]
mod formatting;
pub use formatting::{locale, js_number_to_string, js_to_precision, js_to_exponential, format_number, json_stringify, format_value, compact_diagnostic_text};


/// Shared mutable variable environment (the TS `Map<string, Value>`; clones of
/// the `Rc` share the map exactly like spreading `{...ctx, env}` shares it).
pub type Env<'a> = Rc<RefCell<HashMap<String, Value<'a>>>>;

pub fn env_of(map: HashMap<String, Value<'_>>) -> Env<'_> {
    Rc::new(RefCell::new(map))
}

/// One activated OpenSCAD lexical statement scope (stable profile), a port of
/// `OpenScadStableScope`: the plan is textual, values are lazy and cached per
/// activation; the last assignment supplies the value everywhere in the scope,
/// but its RHS sees only bindings already present when the target was first
/// introduced.
pub struct StableScope<'a> {
    variables: Vec<(String, usize, &'a Expr)>,
    variable_index: HashMap<String, usize>,
    functions: HashMap<String, &'a FunctionNode>,
    modules: HashMap<String, &'a ModuleNode>,
    pub parent: Option<Rc<StableScope<'a>>>,
    pub env: Env<'a>,
    values: RefCell<HashMap<String, Value<'a>>>,
    resolving: RefCell<HashSet<String>>,
}

/// Evaluation site handed to the lazy variable evaluator.
pub struct ScopeEvaluationSite<'a> {
    pub scope: Rc<StableScope<'a>>,
    pub visible_before: usize,
    pub env: HashMap<String, Value<'a>>,
}

pub struct VariableResolution<'a> {
    pub found: bool,
    pub value: Value<'a>,
}

impl<'a> StableScope<'a> {
    pub fn new(
        statements: &'a [Statement],
        parent: Option<Rc<StableScope<'a>>>,
        env: HashMap<String, Value<'a>>,
    ) -> Self {
        let mut variables: Vec<(String, usize, &'a Expr)> = Vec::new();
        let mut variable_index: HashMap<String, usize> = HashMap::new();
        let mut functions = HashMap::new();
        let mut modules = HashMap::new();
        for (statement_index, statement) in statements.iter().enumerate() {
            match statement {
                Statement::Assign(assign) => match variable_index.get(&assign.name) {
                    Some(&existing) => {
                        variables[existing].2 = &assign.value;
                    }
                    None => {
                        variable_index.insert(assign.name.clone(), variables.len());
                        variables.push((assign.name.clone(), statement_index, &assign.value));
                    }
                },
                Statement::Function(function) => {
                    functions.insert(function.name.clone(), function);
                }
                Statement::Module(module) => {
                    modules.insert(module.name.clone(), module);
                }
                _ => {}
            }
        }
        Self {
            variables,
            variable_index,
            functions,
            modules,
            parent,
            env: env_of(env),
            values: RefCell::new(HashMap::new()),
            resolving: RefCell::new(HashSet::new()),
        }
    }

    pub fn dynamic_variable_names(&self) -> Vec<String> {
        self.variables
            .iter()
            .filter(|(name, _, _)| name.starts_with('$'))
            .map(|(name, _, _)| name.clone())
            .collect()
    }

    pub fn resolve_local(
        self: &Rc<Self>,
        name: &str,
        visible_before: usize,
        evaluate: &mut dyn FnMut(&'a Expr, ScopeEvaluationSite<'a>) -> EvalResult<Value<'a>>,
        warn: &mut dyn FnMut(String),
    ) -> EvalResult<VariableResolution<'a>> {
        let Some(&binding_index) = self.variable_index.get(name) else {
            return Ok(VariableResolution {
                found: false,
                value: Value::Undef,
            });
        };
        let (_, first_statement, binding_value) = &self.variables[binding_index];
        if *first_statement >= visible_before {
            return Ok(VariableResolution {
                found: false,
                value: Value::Undef,
            });
        }
        if let Some(value) = self.values.borrow().get(name) {
            return Ok(VariableResolution {
                found: true,
                value: value.clone(),
            });
        }
        if !self.resolving.borrow_mut().insert(name.to_string()) {
            warn(format!("Ignoring cyclic variable reference '{name}'"));
            return Ok(VariableResolution {
                found: true,
                value: Value::Undef,
            });
        }
        let site = ScopeEvaluationSite {
            scope: self.clone(),
            visible_before: *first_statement,
            env: self.env.borrow().clone(),
        };
        let result = evaluate(binding_value, site);
        self.resolving.borrow_mut().remove(name);
        let value = result?;
        self.values
            .borrow_mut()
            .insert(name.to_string(), value.clone());
        Ok(VariableResolution { found: true, value })
    }

    pub fn function_declaration(
        self: &Rc<Self>,
        name: &str,
    ) -> Option<(&'a FunctionNode, Rc<StableScope<'a>>)> {
        if let Some(node) = self.functions.get(name) {
            return Some((*node, self.clone()));
        }
        self.parent.as_ref()?.function_declaration(name)
    }

    pub fn module_declaration(
        self: &Rc<Self>,
        name: &str,
    ) -> Option<(&'a ModuleNode, Rc<StableScope<'a>>)> {
        if let Some(node) = self.modules.get(name) {
            return Some((*node, self.clone()));
        }
        self.parent.as_ref()?.module_declaration(name)
    }
}

impl std::fmt::Debug for FunctionValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionValue")
            .field("name", &self.name)
            .finish()
    }
}
