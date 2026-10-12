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

/// `toLocaleString()` grouping used by the pinned limit messages.
pub fn locale(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// ECMAScript `Number.prototype.toString` for finite numbers.
pub fn js_number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value == f64::INFINITY {
        return "Infinity".to_string();
    }
    if value == f64::NEG_INFINITY {
        return "-Infinity".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let negative = value < 0.0;
    let rendered = format!("{:e}", value.abs());
    let (mantissa, exponent) = rendered.split_once('e').expect("LowerExp has exponent");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent + 1;
    let body = if k <= n && n <= 21 {
        let mut out = digits;
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
        out
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        let mut out = String::from("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(&digits);
        out
    } else {
        let mut out = String::new();
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if n > 0 { '+' } else { '-' });
        out.push_str(&(n - 1).abs().to_string());
        out
    };
    if negative { format!("-{body}") } else { body }
}

/// ECMAScript `Number.prototype.toPrecision(p)` for finite nonzero numbers.
pub fn js_to_precision(value: f64, precision: usize) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if !value.is_finite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if value == 0.0 {
        return if precision > 1 {
            format!("0.{}", "0".repeat(precision - 1))
        } else {
            "0".to_string()
        };
    }
    let negative = value < 0.0;
    let rendered = format!("{:.*e}", precision - 1, value.abs());
    let (mantissa, exponent) = rendered.split_once('e').expect("LowerExp has exponent");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let body = if exponent < -6 || exponent >= precision as i32 {
        let mut out = String::new();
        out.push_str(&digits[..1]);
        if precision > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if exponent >= 0 { '+' } else { '-' });
        out.push_str(&exponent.abs().to_string());
        out
    } else if exponent >= 0 {
        let n = exponent as usize + 1;
        let mut out = digits[..n].to_string();
        if precision > n {
            out.push('.');
            out.push_str(&digits[n..]);
        }
        out
    } else {
        let mut out = String::from("0.");
        out.extend(std::iter::repeat_n('0', (-exponent - 1) as usize));
        out.push_str(&digits);
        out
    };
    if negative { format!("-{body}") } else { body }
}

/// ECMAScript `Number.prototype.toExponential(fractionDigits)`.
pub fn js_to_exponential(value: f64, fraction_digits: usize) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if !value.is_finite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    if value == 0.0 {
        return format!("0.{}e+0", "0".repeat(fraction_digits));
    }
    let negative = value < 0.0;
    let rendered = format!("{:.*e}", fraction_digits, value.abs());
    let (mantissa, exponent) = rendered.split_once('e').expect("LowerExp has exponent");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let body = format!(
        "{mantissa}e{}{}",
        if exponent >= 0 { '+' } else { '-' },
        exponent.abs()
    );
    if negative { format!("-{body}") } else { body }
}

fn js_round_to_precision(value: f64, precision: usize) -> f64 {
    js_to_precision(value, precision).parse().unwrap_or(value)
}

/// `formatNumber` from the TS built-ins: six significant digits with the
/// exponent-notation boundary at 1e-6 and trailing-zero trimming.
pub fn format_number(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value == f64::INFINITY {
        return "inf".to_string();
    }
    if value == f64::NEG_INFINITY {
        return "-inf".to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let mut rendered = js_to_precision(value, 6);
    // Math.log10 lands exactly on integers at powers of ten in the JS host;
    // snap there so the -6 boundary matches.
    let raw_exponent = value.abs().log10();
    let exponent = if (raw_exponent - raw_exponent.round()).abs() < 1e-9 {
        raw_exponent.round() as i32
    } else {
        raw_exponent.floor() as i32
    };
    if !rendered.contains('e') && exponent <= -6 {
        rendered = js_to_exponential(value, 5);
    }
    match rendered.find('e') {
        None => rendered,
        Some(at) => {
            let suffix = &rendered[at..];
            let mut significand = rendered[..at].to_string();
            if significand.contains('.') {
                while significand.ends_with('0') {
                    significand.pop();
                }
                if significand.ends_with('.') {
                    significand.pop();
                }
            }
            format!("{significand}{suffix}")
        }
    }
}

/// `JSON.stringify` for strings (the escapes JSON emits for control chars).
pub fn json_stringify(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `formatOpenScadValue` (echo/assert formatting; strings are JSON-quoted).
pub fn format_value(value: &Value) -> String {
    match value {
        Value::Undef => "undef".to_string(),
        Value::Str(s) => json_stringify(s),
        Value::Bool(v) => if *v { "true" } else { "false" }.to_string(),
        Value::Number(v) => {
            if v.is_nan() {
                "nan".to_string()
            } else if *v == f64::INFINITY {
                "inf".to_string()
            } else if *v == f64::NEG_INFINITY {
                "-inf".to_string()
            } else if *v == 0.0 {
                "0".to_string()
            } else {
                js_number_to_string(js_round_to_precision(*v, 6))
            }
        }
        Value::Vector(items) => format!(
            "[{}]",
            items
                .iter()
                .map(format_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Range { start, step, end } => format!(
            "[{} : {} : {}]",
            format_value(&Value::Number(*start)),
            format_value(&Value::Number(*step)),
            format_value(&Value::Number(*end))
        ),
        Value::Function(_) => "function(...)".to_string(),
    }
}

/// `compactDiagnosticText` from the TS evaluator.
pub fn compact_diagnostic_text(value: &str, limit: usize) -> String {
    let mut compact = String::new();
    let mut pending_space = false;
    for ch in value.trim().chars() {
        if ch.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space && !compact.is_empty() {
                compact.push(' ');
            }
            pending_space = false;
            compact.push(ch);
        }
    }
    if compact.chars().count() <= limit {
        return compact;
    }
    let truncated: String = compact.chars().take(limit - 1).collect();
    format!("{truncated}…")
}

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
