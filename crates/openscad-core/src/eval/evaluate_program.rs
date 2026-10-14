use super::*;

pub(super) struct BoundAssert<'a> {
    pub(super) condition: &'a Expr,
    pub(super) condition_text: String,
    pub(super) message: Option<&'a Expr>,
}

pub(super) fn descriptor(name: &str, dimension: u8) -> ShapeDescriptor {
    ShapeDescriptor {
        name: name.to_string(),
        dimension,
        geometry: None,
        profile: None,
    }
}

pub(super) fn viewport_root_activates(node: &CallNode, shapes: &[ShapeDescriptor]) -> bool {
    if !shapes.is_empty() {
        return true;
    }
    // OpenSCAD does not create an empty CSG container for these transparent
    // effect/branch calls.
    !["if", "assert", "echo", "children", "child"].contains(&node.name.as_str())
}

pub(super) fn call_args(node: &CallNode) -> &[(String, Expr)] {
    &node.args
}

pub(super) fn call_arg_kinds(node: &CallNode) -> &[(String, &'static str)] {
    &node.arg_kinds
}

pub(super) fn expr_p(expr: &Expr) -> usize {
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

pub(super) fn node_p(node: &Statement) -> usize {
    match node {
        Statement::Call(call) => call.p,
        Statement::Assign(assign) => assign.p,
        Statement::Module(module) => module.p,
        Statement::Function(function) => function.p,
        Statement::Directive(directive) => directive.p,
    }
}

pub(super) fn overlay_dynamic_variables<'a>(
    target: &mut HashMap<String, Value<'a>>,
    source: &HashMap<String, Value<'a>>,
) {
    for (name, value) in source {
        if name.starts_with('$') {
            target.insert(name.clone(), value.clone());
        }
    }
}

pub(super) fn value_to_string(value: &Value) -> String {
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

pub(super) fn collect_modules<'a>(nodes: &'a [Statement], modules: &mut HashMap<String, &'a ModuleNode>) {
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

pub(super) fn collect_functions<'a>(
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

pub(super) static CSS_COLORS: &[&str] = &[
    "red", "green", "blue", "yellow", "cyan", "magenta", "white", "black", "orange", "gray",
    "grey", "pink", "purple", "brown", "lime", "navy", "teal",
];

pub(super) fn is_hex_color(value: &str) -> bool {
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
