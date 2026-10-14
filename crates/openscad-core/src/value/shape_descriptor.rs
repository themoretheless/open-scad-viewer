use super::*;

/// Stage-2 shape descriptor: geometry modules contribute accounted stubs
/// (real kernels arrive with migration stage 3); counts and dimensions follow
/// the TS evaluator's shape-list semantics exactly.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeDescriptor {
    pub geometry: Option<usize>,
    pub profile: Option<usize>,
    pub name: String,
    pub dimension: u8,
}

/// Failure of any evaluation step. `Error` carries a positioned diagnostic
/// (the TS `OpenSCADParseError` analogue); `Aborted` is the TS `AbortedError`;
/// `ViewportRoot` is the internal stable-profile `!` root selection (the TS
/// `StableViewportRootSelection` throw), caught by the top-level loop.
#[derive(Debug, Clone)]
pub enum EvalFailure {
    Error(ParseError),
    Aborted,
    ViewportRoot(Vec<ShapeDescriptor>),
}

impl EvalFailure {
    pub fn at(position: usize, message: impl Into<String>) -> Self {
        Self::Error(ParseError::new(position, message))
    }
}

pub type EvalResult<T> = Result<T, EvalFailure>;

/// Function value (`function(...) x` literals and resolved `function` definitions).
pub struct FunctionValue<'a> {
    pub name: Option<String>,
    pub params: &'a [ModuleParam],
    pub body: &'a Expr,
    pub closure: Rc<HashMap<String, Value<'a>>>,
    pub lexical_scope: Option<Rc<StableScope<'a>>>,
}

/// OpenSCAD value domain: undef, bool, number, string, vector, range, function.
#[derive(Debug, Clone)]
pub enum Value<'a> {
    Undef,
    Bool(bool),
    Number(f64),
    Str(Rc<str>),
    Vector(Rc<Vec<Value<'a>>>),
    Range { start: f64, step: f64, end: f64 },
    Function(Rc<FunctionValue<'a>>),
}

impl<'a> Value<'a> {
    pub fn string(s: impl Into<String>) -> Self {
        Self::Str(Rc::from(s.into().as_str()))
    }
    pub fn vector(items: Vec<Value<'a>>) -> Self {
        Self::Vector(Rc::new(items))
    }
    pub fn is_undef(&self) -> bool {
        matches!(self, Self::Undef)
    }
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_vector(&self) -> Option<&Rc<Vec<Value<'a>>>> {
        match self {
            Self::Vector(items) => Some(items),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn is_function(&self) -> bool {
        matches!(self, Self::Function(_))
    }
    pub fn range(&self) -> Option<(f64, f64, f64)> {
        match self {
            Self::Range { start, step, end } => Some((*start, *step, *end)),
            _ => None,
        }
    }
}

/// `openScadType` from the TS value semantics.
pub fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Undef => "undefined",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::Str(_) => "string",
        Value::Vector(_) => "vector",
        Value::Range { .. } => "range",
        Value::Function(_) => "function",
    }
}

/// OpenSCAD boolean conversion: false, undef, zero and empty containers are false.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Undef => false,
        Value::Bool(v) => *v,
        Value::Number(v) => *v != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::Vector(items) => !items.is_empty(),
        // A range stays truthy even when its direction makes iteration empty.
        Value::Range { .. } | Value::Function(_) => true,
    }
}

const MAX_UINT32: f64 = 0xffff_ffffu64 as f64;

/// IEEE-754 `nextUp` port (TS DataView bit manipulation).
pub fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

/// `RangeType::numValues()` semantics from the TS port.
pub fn range_item_count(start: f64, step: f64, end: f64) -> f64 {
    if start.is_nan() || step.is_nan() || end.is_nan() {
        return 0.0;
    }
    if (step < 0.0 && start < end) || (step >= 0.0 && start > end) {
        return 0.0;
    }
    if start == end || !step.is_finite() {
        return 1.0;
    }
    if !start.is_finite() || !end.is_finite() || step == 0.0 {
        return MAX_UINT32;
    }
    let steps = next_up((end - start) / step).floor();
    if !steps.is_finite() || steps >= MAX_UINT32 {
        return MAX_UINT32;
    }
    (steps + 1.0).max(0.0)
}

/// Deep equality including range comparison by item count (`openScadDeepEqual`).
pub fn deep_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Vector(a), Value::Vector(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| deep_equal(a, b))
        }
        (Value::Vector(_), _) | (_, Value::Vector(_)) => false,
        (
            Value::Range {
                start: s1,
                step: st1,
                end: e1,
            },
            Value::Range {
                start: s2,
                step: st2,
                end: e2,
            },
        ) => {
            let left_count = range_item_count(*s1, *st1, *e1);
            let right_count = range_item_count(*s2, *st2, *e2);
            if left_count == 0.0 {
                return right_count == 0.0;
            }
            right_count != 0.0 && s1 == s2 && st1 == st2 && left_count == right_count
        }
        (Value::Range { .. }, _) | (_, Value::Range { .. }) => false,
        (Value::Undef, Value::Undef) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        _ => false,
    }
}

/// Lexicographic `compareValues` of the evaluator value semantics. Number
/// comparison returns the raw difference like the TS `left - right`.
pub fn compare_values(left: &Value, right: &Value) -> Option<f64> {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => Some(a - b),
        (Value::Str(a), Value::Str(b)) => Some(if a < b {
            -1.0
        } else if a > b {
            1.0
        } else {
            0.0
        }),
        (Value::Bool(a), Value::Bool(b)) => Some(*a as u8 as f64 - *b as u8 as f64),
        (Value::Vector(a), Value::Vector(b)) => {
            let length = a.len().min(b.len());
            for index in 0..length {
                if deep_equal(&a[index], &b[index]) {
                    continue;
                }
                return compare_values(&a[index], &b[index]);
            }
            Some(a.len() as f64 - b.len() as f64)
        }
        (
            Value::Range {
                start: s1,
                step: st1,
                end: e1,
            },
            Value::Range {
                start: s2,
                step: st2,
                end: e2,
            },
        ) => {
            let left_count = range_item_count(*s1, *st1, *e1);
            let right_count = range_item_count(*s2, *st2, *e2);
            if left_count == 0.0 || right_count == 0.0 {
                return Some(if left_count == right_count {
                    0.0
                } else if left_count == 0.0 {
                    -1.0
                } else {
                    1.0
                });
            }
            if s1 != s2 {
                return Some(if s1 < s2 { -1.0 } else { 1.0 });
            }
            if st1 != st2 {
                return Some(if st1 < st2 { -1.0 } else { 1.0 });
            }
            Some(left_count - right_count)
        }
        _ => None,
    }
}

pub(super) fn token(operator: TT) -> &'static str {
    match operator {
        TT::Plus => "+",
        TT::Minus => "-",
        TT::Star => "*",
        TT::Slash => "/",
        TT::Percent => "%",
        TT::Caret => "^",
        TT::Lt => "<",
        TT::Gt => ">",
        TT::LtEq => "<=",
        TT::GtEq => ">=",
        TT::EqEq => "==",
        TT::NotEq => "!=",
        TT::And => "&&",
        TT::Or => "||",
        other => other.name(),
    }
}

/// Host callbacks required by the pure value operators (warnings and the
/// vector-allocation budget accounting of the evaluator).
type RegisterArray<'h, 'a> =
    Box<dyn FnMut(Vec<Value<'a>>, &str) -> EvalResult<Vec<Value<'a>>> + 'h>;

pub struct SemanticsContext<'h, 'a> {
    pub max_range_items: usize,
    pub warn: Box<dyn FnMut(String) + 'h>,
    pub register_array: RegisterArray<'h, 'a>,
}

impl<'h, 'a> SemanticsContext<'h, 'a> {
    pub(super) fn register(&mut self, values: Vec<Value<'a>>, label: &str) -> EvalResult<Vec<Value<'a>>> {
        (self.register_array)(values, label)
    }
    pub(super) fn undefined_operation(
        &mut self,
        operator: TT,
        left: &Value<'a>,
        right: &Value<'a>,
    ) -> Value<'a> {
        (self.warn)(format!(
            "Undefined operation ({} {} {})",
            type_name(left),
            token(operator),
            type_name(right)
        ));
        Value::Undef
    }
}

/// `materializeOpenScadRange`: iteration expansion with epsilon and the
/// item-count warning.
pub fn materialize_range<'a>(
    start: f64,
    step: f64,
    end: f64,
    ctx: &mut SemanticsContext<'_, 'a>,
) -> EvalResult<Vec<Value<'a>>> {
    let output: Vec<Value<'a>> = Vec::new();
    if step == 0.0 || ![start, step, end].iter().all(|v| v.is_finite()) {
        (ctx.warn)("Invalid range bounds produce an empty range".to_string());
        return ctx.register(output, "range");
    }
    let forward = step > 0.0;
    let epsilon = 1.0_f64.max(start.abs()).max(end.abs()) * 1e-12;
    let mut output = output;
    let mut value = start;
    while if forward {
        value <= end + epsilon
    } else {
        value >= end - epsilon
    } {
        if output.len() >= ctx.max_range_items {
            (ctx.warn)(format!(
                "Range exceeds {} items",
                locale(ctx.max_range_items)
            ));
            return Ok(Vec::new());
        }
        output.push(Value::Number(value));
        value += step;
    }
    ctx.register(output, "range")
}

/// `openScadUnary`.
pub fn unary<'a>(
    operator: TT,
    value: &Value<'a>,
    ctx: &mut SemanticsContext<'_, 'a>,
) -> EvalResult<Value<'a>> {
    if operator == TT::Not {
        return Ok(Value::Bool(!truthy(value)));
    }
    match value {
        Value::Number(v) => Ok(Value::Number(if operator == TT::Minus { -v } else { *v })),
        Value::Vector(items) => {
            let mut mapped = Vec::with_capacity(items.len());
            for item in items.iter() {
                mapped.push(unary(operator, item, ctx)?);
            }
            Ok(Value::vector(ctx.register(mapped, "unary vector")?))
        }
        other => {
            (ctx.warn)(format!(
                "Undefined unary operation ({}{})",
                token(operator),
                type_name(other)
            ));
            Ok(Value::Undef)
        }
    }
}

fn numeric_vector<'v, 'a>(value: &'v Value<'a>) -> Option<&'v Rc<Vec<Value<'a>>>> {
    match value {
        Value::Vector(items) if items.iter().all(|i| matches!(i, Value::Number(_))) => Some(items),
        _ => None,
    }
}

fn numeric_matrix<'v, 'a>(value: &'v Value<'a>) -> Option<&'v Rc<Vec<Value<'a>>>> {
    match value {
        Value::Vector(rows)
            if !rows.is_empty() && rows.iter().all(|r| numeric_vector(r).is_some()) =>
        {
            Some(rows)
        }
        _ => None,
    }
}

fn scale_vector<'a>(
    value: &[Value<'a>],
    scalar: f64,
    ctx: &mut SemanticsContext<'_, 'a>,
) -> EvalResult<Vec<Value<'a>>> {
    let mut out = Vec::with_capacity(value.len());
    for item in value {
        out.push(match item {
            Value::Vector(items) => {
                let scaled = scale_vector(items, scalar, ctx)?;
                Value::vector(ctx.register(scaled, "scaled vector")?)
            }
            Value::Number(v) => Value::Number(v * scalar),
            other => ctx.undefined_operation(TT::Star, other, &Value::Number(scalar)),
        });
    }
    Ok(out)
}

fn dot(left: &[Value], right: &[Value]) -> Option<f64> {
    if left.len() != right.len() {
        return None;
    }
    let mut result = 0.0;
    for index in 0..left.len() {
        result += left[index].as_number()? * right[index].as_number()?;
    }
    Some(result)
}

fn matrix_columns_rows<'a>(matrix: &[Value<'a>]) -> Vec<Vec<f64>> {
    matrix
        .iter()
        .map(|row| {
            row.as_vector()
                .map(|r| {
                    r.iter()
                        .map(|v| v.as_number().unwrap_or(f64::NAN))
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect()
}

pub(super) fn multiply<'a>(
    left: &Value<'a>,
    right: &Value<'a>,
    ctx: &mut SemanticsContext<'_, 'a>,
) -> EvalResult<Value<'a>> {
    if let (Value::Number(a), Value::Number(b)) = (left, right) {
        return Ok(Value::Number(a * b));
    }
    if let Value::Number(scalar) = left
        && let Value::Vector(items) = right
    {
        let items = items.clone();
        let scaled = scale_vector(&items, *scalar, ctx)?;
        return Ok(Value::vector(ctx.register(scaled, "scaled vector")?));
    }
    if let Value::Vector(items) = left
        && let Value::Number(scalar) = right
    {
        let items = items.clone();
        let scaled = scale_vector(&items, *scalar, ctx)?;
        return Ok(Value::vector(ctx.register(scaled, "scaled vector")?));
    }
    if let (Some(l), Some(r)) = (numeric_vector(left), numeric_vector(right)) {
        let (l, r) = (l.clone(), r.clone());
        if let Some(result) = dot(&l, &r) {
            return Ok(Value::Number(result));
        }
        (ctx.warn)(format!(
            "vector*vector requires matching lengths ({} != {})",
            l.len(),
            r.len()
        ));
        return Ok(Value::Undef);
    }
    if let (Some(l), Some(r)) = (numeric_matrix(left), numeric_vector(right)) {
        let (l, r) = (l.clone(), r.clone());
        if l.iter()
            .any(|row| row.as_vector().map_or(0, |v| v.len()) != r.len())
        {
            (ctx.warn)("matrix*vector requires matching dimensions".to_string());
            return Ok(Value::Undef);
        }
        let mut out = Vec::with_capacity(l.len());
        for row in l.iter() {
            let row_values = row.as_vector().cloned().unwrap_or_default();
            out.push(Value::Number(dot(&row_values, &r).unwrap_or(f64::NAN)));
        }
        return Ok(Value::vector(ctx.register(out, "matrix-vector product")?));
    }
    if let (Some(l), Some(r)) = (numeric_vector(left), numeric_matrix(right)) {
        let (l, r) = (l.clone(), r.clone());
        let rows = matrix_columns_rows(&r);
        if rows.len() != l.len() || rows.iter().any(|row| row.len() != rows[0].len()) {
            (ctx.warn)("vector*matrix requires matching rectangular dimensions".to_string());
            return Ok(Value::Undef);
        }
        let columns = rows[0].len();
        let mut out = Vec::with_capacity(columns);
        for column in 0..columns {
            let mut value = 0.0;
            for (row, row_values) in rows.iter().enumerate() {
                value += l[row].as_number().unwrap_or(f64::NAN) * row_values[column];
            }
            out.push(Value::Number(value));
        }
        return Ok(Value::vector(ctx.register(out, "vector-matrix product")?));
    }
    if let (Some(l), Some(r)) = (numeric_matrix(left), numeric_matrix(right)) {
        let (l, r) = (l.clone(), r.clone());
        let left_rows = matrix_columns_rows(&l);
        let right_rows = matrix_columns_rows(&r);
        let shared = left_rows[0].len();
        if left_rows.iter().any(|row| row.len() != shared)
            || right_rows.len() != shared
            || right_rows
                .iter()
                .any(|row| row.len() != right_rows[0].len())
        {
            (ctx.warn)("matrix*matrix requires matching rectangular dimensions".to_string());
            return Ok(Value::Undef);
        }
        let columns = right_rows[0].len();
        let mut out = Vec::with_capacity(left_rows.len());
        for row in &left_rows {
            let mut product_row = Vec::with_capacity(columns);
            for (column, _) in right_rows[0].iter().enumerate().take(columns) {
                let mut value = 0.0;
                for index in 0..shared {
                    value += row[index] * right_rows[index][column];
                }
                product_row.push(Value::Number(value));
            }
            out.push(Value::vector(
                ctx.register(product_row, "matrix product row")?,
            ));
        }
        return Ok(Value::vector(ctx.register(out, "matrix product")?));
    }
    Ok(ctx.undefined_operation(TT::Star, left, right))
}
