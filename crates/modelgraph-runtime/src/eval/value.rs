use super::*;

#[derive(Clone, Debug)]
pub enum Value<'a> {
    Numeric(Numeric),
    Text(&'a str),
    Record(Rc<HashMap<String, Value<'a>>>),
    List(Rc<[Value<'a>]>),
    Closure {
        parameters: &'a [Json],
        body: &'a Json,
        scope: Scope<'a>,
    },
    Geometry {
        function: &'a str,
        scope: Scope<'a>,
    },
}
impl From<f64> for Value<'_> {
    fn from(value: f64) -> Self {
        Self::Numeric(value.into())
    }
}
impl From<Numeric> for Value<'_> {
    fn from(value: Numeric) -> Self {
        Self::Numeric(value)
    }
}
pub fn numeric_value(value: &Value<'_>, path: &str) -> Result<Numeric> {
    match value {
        Value::Numeric(value) => Ok(*value),
        _ => Err(Error::new(
            "type_error",
            path,
            "Expected a number or quantity.",
        )),
    }
}
pub fn numeric(value: &Value<'_>, path: &str) -> Result<f64> {
    units::scalar(numeric_value(value, path)?, path)
}
pub fn sequence<'v, 'a>(value: &'v Value<'a>, path: &str) -> Result<&'v [Value<'a>]> {
    match value {
        Value::List(values) => Ok(values),
        _ => Err(Error::new("type_error", path, "Expected a list.")),
    }
}
pub struct Bound<'a> {
    pub function: &'a Json,
    pub scope: Scope<'a>,
}
pub struct Checks {
    pub constraint_report: Json,
    pub geometry_assertions: Json,
}
pub struct PendingGeometryCheck<'a> {
    pub check: Json,
    pub scope: Scope<'a>,
}
pub(super) struct MemoEntry<'a> {
    pub(super) _scope: Scope<'a>,
    pub(super) expression: &'a Json,
    pub(super) result: Result<Value<'a>>,
}
pub struct Evaluator<'a> {
    pub global_scope: Scope<'a>,
    pub(super) memo: HashMap<(&'a str, usize), MemoEntry<'a>>,
    pub(super) active_memo: HashSet<(&'a str, usize)>,
    pub(super) geometry_check_count: usize,
    pub(super) geometry_effect_count: usize,
    pub pending_geometry_checks: Vec<PendingGeometryCheck<'a>>,
    pub constraint_report: Vec<Json>,
    pub suppress_checks: bool,
    pub(super) expanded_checks: usize,
    pub strict: bool,
    pub functions: HashMap<&'a str, &'a Json>,
    pub(super) parameters: HashMap<&'a str, Numeric>,
    pub(super) document: &'a Json,
    pub(super) steps: usize,
    pub(super) allocated: usize,
}
