//! Bounded expression execution. ASTs stay borrowed and immutable; list values
//! and captured scopes share storage instead of cloning expression subtrees.
use crate::range::resolve_interval;
use crate::units::{self, Dimension, Numeric};
use crate::{Error, Result};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use value_codec::{json, Value as Json};

pub type Scope<'a> = Rc<HashMap<String, Value<'a>>>;
#[derive(Clone, Debug)]
pub enum Value<'a> {
    Numeric(Numeric),
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
pub struct Evaluator<'a> {
    pub strict: bool,
    pub functions: HashMap<&'a str, &'a Json>,
    parameters: HashMap<&'a str, Numeric>,
    document: &'a Json,
    steps: usize,
    allocated: usize,
}
impl<'a> Evaluator<'a> {
    pub fn new(document: &'a Json) -> Result<Self> {
        let mut parameters = HashMap::new();
        for (i, parameter) in array(&document["parameters"]).iter().enumerate() {
            let id = string(&parameter["id"]);
            if parameters.contains_key(id) {
                return Err(Error::new(
                    "duplicate_id",
                    format!("/parameters/{i}/id"),
                    "Parameter IDs must be unique.",
                ));
            }
            let path = format!("/parameters/{i}");
            let value = number(&parameter["value"]);
            let min = parameter.get("min").and_then(Json::as_f64);
            let max = parameter.get("max").and_then(Json::as_f64);
            if min.zip(max).is_some_and(|(a, b)| a > b)
                || min.is_some_and(|min| value < min)
                || max.is_some_and(|max| value > max)
                || (parameter["integer"] == true && value.fract() != 0.0)
            {
                return Err(Error::new(
                    "parameter_constraint",
                    path,
                    format!("Parameter {id} violates its declared bounds or integer requirement."),
                ));
            }
            parameters.insert(
                id,
                if let Some(unit) = parameter.get("unit") {
                    units::quantity(value, string(unit), &path)?
                } else {
                    value.into()
                },
            );
        }
        let mut functions = HashMap::new();
        for function in array(&document["functions"]) {
            functions.insert(string(&function["id"]), function);
        }
        if functions.len() != array(&document["functions"]).len() {
            return Err(Error::new(
                "duplicate_id",
                "/functions",
                "Function IDs must be unique.",
            ));
        }
        // Preserve source-order error reporting even though lookup uses a hash table.
        for function in array(&document["functions"]) {
            if !unique(array(&function["parameters"])) {
                return Err(Error::new(
                    "duplicate_id",
                    format!("/functions/{}", string(&function["id"])),
                    "Function parameters must be unique.",
                ));
            }
        }
        Ok(Self {
            strict: document["type_policy"] == "strict",
            functions,
            parameters,
            document,
            steps: 0,
            allocated: 0,
        })
    }
    fn allocate(&mut self, count: usize, path: &str) -> Result<()> {
        self.allocated += count;
        if self.allocated > 16384 {
            return Err(Error::new(
                "allocation_limit",
                path,
                "List allocation budget 16384 exceeded.",
            ));
        }
        Ok(())
    }
    fn tick(&mut self, depth: usize, path: &str) -> Result<()> {
        // JS short-circuits before incrementing steps when depth is exceeded.
        if depth > 32 {
            return Err(Error::new(
                "evaluation_limit",
                path,
                "Evaluation depth 32 or step budget 100000 exceeded.",
            ));
        }
        self.steps += 1;
        if self.steps > 100_000 {
            return Err(Error::new(
                "evaluation_limit",
                path,
                "Evaluation depth 32 or step budget 100000 exceeded.",
            ));
        }
        Ok(())
    }
    pub fn bind(
        &mut self,
        name: &str,
        args: &'a Json,
        scope: &Scope<'a>,
        path: &str,
        depth: usize,
    ) -> Result<Bound<'a>> {
        let function = *self.functions.get(name).ok_or_else(|| {
            Error::new(
                "unknown_function",
                path,
                format!("Unknown function {name}."),
            )
        })?;
        let parameters = array(&function["parameters"]);
        let Some(args) = args.as_object() else {
            return Err(Error::new(
                "invalid_arguments",
                path,
                "Arguments must exactly match function parameters.",
            ));
        };
        if args.len() != parameters.len()
            || parameters.iter().any(|key| !args.contains_key(string(key)))
        {
            return Err(Error::new(
                "invalid_arguments",
                path,
                "Arguments must exactly match function parameters.",
            ));
        }
        let mut bound = HashMap::with_capacity(parameters.len());
        for key in parameters {
            let key = string(key);
            bound.insert(
                key.to_owned(),
                self.resolve(&args[key], scope, &format!("{path}/args/{key}"), depth + 1)?,
            );
        }
        Ok(Bound {
            function,
            scope: Rc::new(bound),
        })
    }
    pub fn invoke(
        &mut self,
        value: &Value<'a>,
        args: Vec<Value<'a>>,
        path: &str,
        depth: usize,
    ) -> Result<Value<'a>> {
        let Value::Closure {
            parameters,
            body,
            scope,
        } = value
        else {
            return Err(Error::new("type_error", path, "Expected a function value."));
        };
        if args.len() != parameters.len() {
            return Err(Error::new(
                "invalid_arguments",
                path,
                "Lambda arity mismatch.",
            ));
        }
        let mut bound = (**scope).clone();
        for (key, value) in parameters.iter().zip(args) {
            bound.insert(string(key).to_owned(), value);
        }
        self.resolve(body, &Rc::new(bound), &format!("{path}/apply"), depth + 1)
    }
    pub fn evaluate(
        &mut self,
        expr: &'a Json,
        scope: &Scope<'a>,
        path: &str,
        depth: usize,
    ) -> Result<f64> {
        numeric(&self.resolve(expr, scope, path, depth)?, path)
    }
    pub fn field(
        &mut self,
        expr: &'a Json,
        scope: &Scope<'a>,
        path: &str,
        expected: Dimension,
    ) -> Result<f64> {
        units::field(
            numeric_value(&self.resolve(expr, scope, path, 0)?, path)?,
            expected,
            path,
            self.strict,
        )
    }
    pub fn resolve(
        &mut self,
        expr: &'a Json,
        scope: &Scope<'a>,
        path: &str,
        depth: usize,
    ) -> Result<Value<'a>> {
        self.tick(depth, path)?;
        if let Some(value) = expr.as_f64() {
            if !value.is_finite() || value.abs() > 1_000_000.0 {
                return Err(Error::new(
                    "invalid_number",
                    path,
                    "Expression must produce a finite number within +/-1000000.",
                ));
            }
            return Ok(value.into());
        }
        if let Some(key) = expr.get("param") {
            let key = string(key);
            return self
                .parameters
                .get(key)
                .copied()
                .map(Value::Numeric)
                .ok_or_else(|| {
                    Error::new(
                        "unknown_parameter",
                        path,
                        format!("Unknown parameter {key}."),
                    )
                });
        }
        if let Some(key) = expr.get("local") {
            let key = string(key);
            return scope
                .get(key)
                .cloned()
                .ok_or_else(|| Error::new("unknown_local", path, format!("Unknown local {key}.")));
        }
        let op = string(&expr["op"]);
        match op {
            "quantity" => {
                return Ok(
                    units::quantity(number(&expr["value"]), string(&expr["unit"]), path)?.into(),
                )
            }
            "geometry" => {
                let name = string(&expr["function"]);
                let bound = self.bind(name, &expr["args"], scope, path, depth)?;
                if bound.function["kind"] != "geometry" {
                    return Err(Error::new(
                        "type_error",
                        path,
                        "Expected a geometry function.",
                    ));
                }
                return Ok(Value::Geometry {
                    function: name,
                    scope: bound.scope,
                });
            }
            "lambda" => {
                let parameters = array(&expr["parameters"]);
                if !unique(parameters) {
                    return Err(Error::new(
                        "duplicate_id",
                        path,
                        "Lambda parameters must be unique.",
                    ));
                }
                return Ok(Value::Closure {
                    parameters,
                    body: &expr["body"],
                    scope: scope.clone(),
                });
            }
            "apply" => {
                let function = self.resolve(
                    &expr["function"],
                    scope,
                    &format!("{path}/function"),
                    depth + 1,
                )?;
                let mut args = Vec::with_capacity(array(&expr["args"]).len());
                for (i, arg) in array(&expr["args"]).iter().enumerate() {
                    args.push(self.resolve(arg, scope, &format!("{path}/args/{i}"), depth + 1)?);
                }
                return self.invoke(&function, args, path, depth + 1);
            }
            "list" => {
                let items = array(&expr["items"]);
                self.allocate(items.len(), path)?;
                let mut values = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    values.push(self.resolve(
                        item,
                        scope,
                        &format!("{path}/items/{i}"),
                        depth + 1,
                    )?);
                }
                return Ok(Value::List(values.into()));
            }
            "interval" => {
                let start = numeric_value(
                    &self.resolve(&expr["start"], scope, &format!("{path}/start"), depth + 1)?,
                    path,
                )?;
                let end = numeric_value(
                    &self.resolve(&expr["end"], scope, &format!("{path}/end"), depth + 1)?,
                    path,
                )?;
                let count = if let Some(value) = expr.get("count") {
                    Some(numeric_value(
                        &self.resolve(value, scope, &format!("{path}/count"), depth + 1)?,
                        path,
                    )?)
                } else {
                    None
                };
                let step = if let Some(value) = expr.get("step") {
                    Some(numeric_value(
                        &self.resolve(value, scope, &format!("{path}/step"), depth + 1)?,
                        path,
                    )?)
                } else {
                    None
                };
                let values =
                    resolve_interval(start, end, expr["inclusive"] == true, count, step, path)?;
                self.allocate(values.len(), path)?;
                return Ok(Value::List(
                    values.into_iter().map(Value::Numeric).collect(),
                ));
            }
            "zip" => {
                let mut lists = Vec::with_capacity(array(&expr["inputs"]).len());
                for (i, input) in array(&expr["inputs"]).iter().enumerate() {
                    let value =
                        self.resolve(input, scope, &format!("{path}/inputs/{i}"), depth + 1)?;
                    sequence(&value, path)?;
                    lists.push(value);
                }
                let count = lists
                    .first()
                    .map(|value| sequence(value, path).map(<[_]>::len))
                    .transpose()?
                    .unwrap_or(0);
                if lists
                    .iter()
                    .any(|value| matches!(value, Value::List(values) if values.len() != count))
                {
                    return Err(Error::new(
                        "length_mismatch",
                        path,
                        "zip requires equal length sequences.",
                    ));
                }
                self.allocate(count * (lists.len() + 1), path)?;
                let mut output = Vec::with_capacity(count);
                for i in 0..count {
                    output.push(Value::List(
                        lists
                            .iter()
                            .map(|value| sequence(value, path).map(|items| items[i].clone()))
                            .collect::<Result<Vec<_>>>()?
                            .into(),
                    ));
                }
                return Ok(Value::List(output.into()));
            }
            "enumerate" => {
                let value =
                    self.resolve(&expr["input"], scope, &format!("{path}/input"), depth + 1)?;
                let items = sequence(&value, path)?;
                self.allocate(items.len() * 3, path)?;
                return Ok(Value::List(
                    items
                        .iter()
                        .enumerate()
                        .map(|(i, value)| {
                            Value::List(vec![(i as f64).into(), value.clone()].into())
                        })
                        .collect(),
                ));
            }
            "range" => {
                let count = numeric(
                    &self.resolve(&expr["count"], scope, &format!("{path}/count"), depth + 1)?,
                    path,
                )?;
                let start = numeric_value(
                    &self.resolve(&expr["start"], scope, &format!("{path}/start"), depth + 1)?,
                    path,
                )?;
                let step = numeric_value(
                    &self.resolve(&expr["step"], scope, &format!("{path}/step"), depth + 1)?,
                    path,
                )?;
                units::equal(start, step, path)?;
                if count.fract() != 0.0 || !(0.0..=256.0).contains(&count) {
                    return Err(Error::new(
                        "invalid_count",
                        path,
                        "Range count must be an integer from 0 to 256.",
                    ));
                }
                self.allocate(count as usize, path)?;
                let mut output = Vec::with_capacity(count as usize);
                for i in 0..count as usize {
                    output.push(
                        units::binary(
                            "add",
                            start,
                            units::binary("multiply", (i as f64).into(), step, path)?,
                            path,
                        )?
                        .into(),
                    );
                }
                return Ok(Value::List(output.into()));
            }
            "length" => {
                let value =
                    self.resolve(&expr["input"], scope, &format!("{path}/input"), depth + 1)?;
                return Ok((sequence(&value, path)?.len() as f64).into());
            }
            "at" => {
                let value =
                    self.resolve(&expr["input"], scope, &format!("{path}/input"), depth + 1)?;
                let items = sequence(&value, path)?;
                let index = numeric(
                    &self.resolve(&expr["index"], scope, &format!("{path}/index"), depth + 1)?,
                    path,
                )?;
                if index.fract() != 0.0 || index < 0.0 || index >= items.len() as f64 {
                    return Err(Error::new(
                        "invalid_index",
                        path,
                        "List index is out of bounds.",
                    ));
                }
                return Ok(items[index as usize].clone());
            }
            "map" | "filter" | "flatmap" | "reduce" => {
                let value =
                    self.resolve(&expr["input"], scope, &format!("{path}/input"), depth + 1)?;
                let items = sequence(&value, path)?;
                let function = self.resolve(
                    &expr["function"],
                    scope,
                    &format!("{path}/function"),
                    depth + 1,
                )?;
                self.allocate(items.len(), path)?;
                if op == "reduce" {
                    let initial = expr.get("initial").ok_or_else(|| {
                        Error::new("type_error", path, "Expected reduce initial value.")
                    })?;
                    let mut accumulator =
                        self.resolve(initial, scope, &format!("{path}/initial"), depth + 1)?;
                    for (i, item) in items.iter().enumerate() {
                        accumulator = self.invoke(
                            &function,
                            vec![accumulator, item.clone()],
                            &format!("{path}[{i}]"),
                            depth + 1,
                        )?;
                    }
                    return Ok(accumulator);
                }
                let mut output = Vec::with_capacity(items.len());
                for (i, item) in items.iter().enumerate() {
                    let value = self.invoke(
                        &function,
                        vec![item.clone()],
                        &format!("{path}[{i}]"),
                        depth + 1,
                    )?;
                    match op {
                        "map" => output.push(value),
                        "filter" => {
                            if numeric(&value, path)? != 0.0 {
                                output.push(item.clone());
                            }
                        }
                        _ => {
                            let batch = sequence(&value, path)?;
                            self.allocate(batch.len(), path)?;
                            if output.len() + batch.len() > 256 {
                                return Err(Error::new(
                                    "invalid_count",
                                    path,
                                    "Generated sequence exceeds 256 values.",
                                ));
                            }
                            output.extend_from_slice(batch);
                        }
                    }
                }
                return Ok(Value::List(output.into()));
            }
            "checked" => {
                for (i, check) in array(&expr["checks"]).iter().enumerate() {
                    self.resolve(check, scope, &format!("{path}/checks/{i}"), depth + 1)?;
                }
                return self.resolve(&expr["value"], scope, &format!("{path}/value"), depth + 1);
            }
            "typed" => {
                return Ok(units::check_type(
                    numeric_value(
                        &self.resolve(
                            &expr["value"],
                            scope,
                            &format!("{path}/value"),
                            depth + 1,
                        )?,
                        path,
                    )?,
                    string(&expr["type"]),
                    path,
                )?
                .into())
            }
            "if" => {
                let field = if self.evaluate(
                    &expr["condition"],
                    scope,
                    &format!("{path}/condition"),
                    depth + 1,
                )? != 0.0
                {
                    "then"
                } else {
                    "else"
                };
                return self.resolve(&expr[field], scope, path, depth + 1);
            }
            "let" => {
                let value =
                    self.resolve(&expr["value"], scope, &format!("{path}/value"), depth + 1)?;
                let mut bound = (**scope).clone();
                bound.insert(string(&expr["name"]).to_owned(), value);
                return self.resolve(
                    &expr["body"],
                    &Rc::new(bound),
                    &format!("{path}/body"),
                    depth + 1,
                );
            }
            "call" => {
                let name = string(&expr["function"]);
                let bound = self.bind(name, &expr["args"], scope, path, depth)?;
                if bound.function["kind"] == "geometry" {
                    return Err(Error::new(
                        "type_error",
                        path,
                        "Expected a scalar function.",
                    ));
                }
                let value = self.resolve(
                    &bound.function["body"],
                    &bound.scope,
                    &format!("{path}/call:{name}"),
                    depth + 1,
                )?;
                if bound.function["kind"] == "scalar" {
                    return Ok(numeric_value(&value, path)?.into());
                }
                return Ok(value);
            }
            _ => (),
        }
        if let Some(args) = expr.get("args") {
            let a = numeric_value(
                &self.resolve(&args[0], scope, &format!("{path}/args/0"), depth + 1)?,
                path,
            )?;
            if op == "and" && units::scalar(a, path)? == 0.0 {
                return Ok(0.0.into());
            }
            if op == "or" && units::scalar(a, path)? != 0.0 {
                return Ok(1.0.into());
            }
            let b = numeric_value(
                &self.resolve(&args[1], scope, &format!("{path}/args/1"), depth + 1)?,
                path,
            )?;
            return Ok(units::binary(op, a, b, path)?.into());
        }
        if let Some(value) = expr.get("value") {
            let value = numeric_value(
                &self.resolve(value, scope, &format!("{path}/value"), depth + 1)?,
                path,
            )?;
            return Ok(units::unary(op, value, path, self.strict)?.into());
        }
        Err(Error::new("type_error", path, "Invalid expression."))
    }
    pub fn validate_checks(&mut self) -> Result<Checks> {
        let (constraint_report, geometry_assertions) = self.evaluate_checks(self.document)?;
        Ok(Checks {
            constraint_report,
            geometry_assertions,
        })
    }
    pub fn evaluate_checks(&mut self, document: &'a Json) -> Result<(Json, Json)> {
        let scope = Scope::default();
        for (i, assertion) in array(&document["assertions"]).iter().enumerate() {
            if self.evaluate(
                &assertion["condition"],
                &scope,
                &format!("/assertions/{i}/condition"),
                0,
            )? == 0.0
            {
                return Err(Error::new(
                    "assertion_failed",
                    format!("/assertions/{i}"),
                    string(&assertion["message"]),
                ));
            }
        }
        let mut ids = HashSet::new();
        let mut report = Vec::with_capacity(array(&document["constraints"]).len());
        for (i, constraint) in array(&document["constraints"]).iter().enumerate() {
            let path = format!("/constraints/{i}");
            if !ids.insert(string(&constraint["id"])) {
                return Err(Error::new(
                    "duplicate_id",
                    &path,
                    "Constraint IDs must be unique.",
                ));
            }
            let left = numeric_value(
                &self.resolve(&constraint["left"], &scope, &format!("{path}/left"), 0)?,
                &path,
            )?;
            let right = numeric_value(
                &self.resolve(&constraint["right"], &scope, &format!("{path}/right"), 0)?,
                &path,
            )?;
            units::equal(left, right, &path)?;
            let relation = string(&constraint["relation"]);
            if matches!(relation, "lt" | "gt") && constraint.get("tolerance").is_some() {
                return Err(Error::new(
                    "invalid_tolerance",
                    &path,
                    "Strict comparisons do not accept tolerance.",
                ));
            }
            let mut tolerance = 0.0;
            if let Some(value) = constraint.get("tolerance") {
                let value = numeric_value(
                    &self.resolve(value, &scope, &format!("{path}/tolerance"), 0)?,
                    &path,
                )?;
                units::equal(left, value, &path)?;
                tolerance = value.value;
                if tolerance < 0.0 {
                    return Err(Error::new(
                        "invalid_tolerance",
                        &path,
                        "Tolerance must be nonnegative.",
                    ));
                }
            }
            let (a, b) = (left.value, right.value);
            let passed = match relation {
                "lt" => a < b,
                "gt" => a > b,
                "eq" => (a - b).abs() <= tolerance,
                "le" => a <= b + tolerance,
                _ => a >= b - tolerance,
            };
            report.push(json!({"id":constraint["id"],"path":path,"passed":passed,"status":if passed {"passed"} else {"failed"},"actual":a,"expected":b,"relation":relation,"tolerance":tolerance,"dimension":left.dimension,"message":constraint["message"]}));
        }
        if report.iter().any(|value| value["passed"] == false) {
            let failures = report
                .iter()
                .filter(|value| value["passed"] == false)
                .map(|value| {
                    let relation = match value["relation"].as_str().unwrap_or("") {
                        "le" => "<=",
                        "ge" => ">=",
                        "eq" => "==",
                        "lt" => "<",
                        "gt" => ">",
                        _ => "?",
                    };
                    format!(
                        "{} (actual: {}; required: {} {}; tolerance: {})",
                        value["message"].as_str().unwrap_or("Constraint failed"),
                        value["actual"],
                        relation,
                        value["expected"],
                        value["tolerance"]
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            return Err(Error::new("constraint_failed", "/constraints", &failures)
                .with_details(Json::Array(report)));
        }
        let mut geometry = Vec::with_capacity(array(&document["geometry_assertions"]).len());
        ids.clear();
        for (i, check) in array(&document["geometry_assertions"]).iter().enumerate() {
            let path = format!("/geometry_assertions/{i}");
            if check["target"] != document["root"] {
                return Err(Error::new(
                    "unsupported_assertion_target",
                    &path,
                    "Geometry checks currently target the shown root only.",
                ));
            }
            if !ids.insert(string(&check["id"])) {
                return Err(Error::new(
                    "duplicate_id",
                    &path,
                    "Check IDs must be unique.",
                ));
            }
            let kind = string(&check["check"]);
            let measured = matches!(kind, "height" | "width" | "depth");
            let expected = if let Some(value) = check.get("expected") {
                let value = numeric_value(&self.resolve(value, &scope, &path, 0)?, &path)?;
                if measured {
                    units::equal(value, units::quantity(1.0, "mm", &path)?, &path)?;
                } else {
                    units::scalar(value, &path)?;
                }
                value.value
            } else if kind == "hasBodies" || measured {
                return Err(Error::new(
                    "missing_expected",
                    &path,
                    "Expected value required.",
                ));
            } else {
                0.0
            };
            if expected < 0.0 || (kind == "hasBodies" && expected.fract() != 0.0) {
                return Err(Error::new(
                    "invalid_expected",
                    &path,
                    "Expected value must be nonnegative; body count must be integral.",
                ));
            }
            let mut tolerance = 0.0;
            if let Some(value) = check.get("tolerance") {
                let value = numeric_value(&self.resolve(value, &scope, &path, 0)?, &path)?;
                if !measured {
                    return Err(Error::new(
                        "invalid_tolerance",
                        &path,
                        "Tolerance applies only to measurements.",
                    ));
                }
                units::equal(value, units::quantity(1.0, "mm", &path)?, &path)?;
                tolerance = value.value;
                if tolerance < 0.0 {
                    return Err(Error::new(
                        "invalid_tolerance",
                        &path,
                        "Tolerance must be nonnegative.",
                    ));
                }
            }
            if !measured && kind != "hasBodies" && check.get("expected").is_some() {
                return Err(Error::new(
                    "invalid_expected",
                    &path,
                    "Topology checks take no expected argument.",
                ));
            }
            let mut result = check.clone();
            result["expected"] = json!(expected);
            result["tolerance"] = json!(tolerance);
            geometry.push(result);
        }
        Ok((Json::Array(report), Json::Array(geometry)))
    }
}
fn array(value: &Json) -> &[Json] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn string(value: &Json) -> &str {
    value.as_str().unwrap_or("")
}
fn number(value: &Json) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}
fn unique(items: &[Json]) -> bool {
    let mut seen = HashSet::with_capacity(items.len());
    items.iter().all(|item| seen.insert(string(item)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Json {
        json!({"parameters":[],"functions":[],"root":"Root"})
    }
    fn evaluate(expression: &Json) -> Result<Value<'_>> {
        // Results borrow the expression; the leaked empty document is avoided by
        // placing it in static storage for these expression-only cases.
        static DOCUMENT: std::sync::OnceLock<Json> = std::sync::OnceLock::new();
        Evaluator::new(DOCUMENT.get_or_init(document))?.resolve(
            expression,
            &Scope::default(),
            "/test",
            0,
        )
    }
    fn scalars(value: &Value<'_>) -> Vec<f64> {
        sequence(value, "/")
            .unwrap()
            .iter()
            .map(|item| numeric(item, "/").unwrap())
            .collect()
    }

    #[test]
    fn arithmetic_units_and_lazy_branches_match_contract() {
        let q = json!({"op":"add","args":[{"op":"quantity","value":2,"unit":"cm"},{"op":"quantity","value":3,"unit":"mm"}]});
        assert_eq!(
            numeric_value(&evaluate(&q).unwrap(), "/").unwrap(),
            Numeric {
                value: 23.0,
                dimension: units::LENGTH
            }
        );
        let mismatch = json!({"op":"add","args":[1,{"op":"quantity","value":3,"unit":"mm"}]});
        let error = evaluate(&mismatch).unwrap_err();
        assert_eq!(
            (error.code.as_str(), error.path.as_str()),
            ("unit_mismatch", "/test")
        );
        for expr in [
            json!({"op":"if","condition":1,"then":7,"else":{"local":"missing"}}),
            json!({"op":"or","args":[1,{"local":"missing"}]}),
            json!({"op":"and","args":[0,{"local":"missing"}]}),
        ] {
            assert!(evaluate(&expr).is_ok());
        }
        let squared = json!({"op":"sqrt","value":{"op":"multiply","args":[{"op":"quantity","value":3,"unit":"mm"},{"op":"quantity","value":3,"unit":"mm"}]}});
        assert_eq!(
            numeric_value(&evaluate(&squared).unwrap(), "/").unwrap(),
            Numeric {
                value: 3.0,
                dimension: units::LENGTH
            }
        );
        let invalid_dimension =
            json!({"op":"sqrt","value":{"op":"quantity","value":4,"unit":"mm"}});
        assert_eq!(
            evaluate(&invalid_dimension).unwrap_err().code,
            "dimension_limit"
        );
    }

    #[test]
    fn closures_capture_lexical_scope_and_shadow_safely() {
        let expr = json!({"op":"let","name":"x","value":10,"body":{"op":"let","name":"f","value":{"op":"lambda","parameters":["n"],"body":{"op":"add","args":[{"local":"x"},{"local":"n"}]}},"body":{"op":"let","name":"x","value":20,"body":{"op":"apply","function":{"local":"f"},"args":[3]}}}});
        assert_eq!(numeric(&evaluate(&expr).unwrap(), "/").unwrap(), 13.0);
        let captured =
            json!({"op":"let","name":"x","value":{"op":"list","items":[1,2]},"body":{"local":"x"}});
        assert_eq!(scalars(&evaluate(&captured).unwrap()), vec![1.0, 2.0]);
    }

    #[test]
    fn sequences_support_flatten_filter_reduce_and_exact_endpoints() {
        let interval = json!({"op":"interval","start":0,"end":0.3,"step":0.1,"inclusive":true});
        let list = evaluate(&interval).unwrap();
        let samples = scalars(&list);
        assert_eq!(samples.len(), 4);
        assert!((samples[3] - 0.3).abs() < 1e-15);
        let counted = json!({"op":"interval","start":0,"end":1,"count":4,"inclusive":true});
        assert_eq!(scalars(&evaluate(&counted).unwrap())[3], 1.0);
        let expr = json!({"op":"reduce","initial":0,"input":{"op":"filter","input":{"op":"flatmap","input":{"op":"range","count":3,"start":1,"step":1},"function":{"op":"lambda","parameters":["x"],"body":{"op":"list","items":[{"local":"x"},{"local":"x"}]}}},"function":{"op":"lambda","parameters":["x"],"body":{"op":"lt","args":[1,{"local":"x"}]}}},"function":{"op":"lambda","parameters":["sum","x"],"body":{"op":"add","args":[{"local":"sum"},{"local":"x"}]}}});
        assert_eq!(numeric(&evaluate(&expr).unwrap(), "/").unwrap(), 10.0);
        let zip =
            json!({"op":"zip","inputs":[{"op":"list","items":[1,2]},{"op":"list","items":[3]}]});
        assert_eq!(evaluate(&zip).unwrap_err().code, "length_mismatch");
    }

    #[test]
    fn budget_checks_count_logical_allocations_despite_shared_storage() {
        let inner = json!({"op":"range","count":256,"start":0,"step":1});
        let expr = json!({"op":"map","input":inner,"function":{"op":"lambda","parameters":["x"],"body":{"op":"range","count":256,"start":0,"step":1}}});
        let error = evaluate(&expr).unwrap_err();
        assert_eq!(error.code, "allocation_limit");
        assert_eq!(error.path, "/test[62]/apply");
        let mut deep = json!(1);
        for _ in 0..33 {
            deep = json!({"op":"negate","value":deep});
        }
        assert_eq!(evaluate(&deep).unwrap_err().code, "evaluation_limit");
        let doc = document();
        let literal = json!(1);
        let mut evaluator = Evaluator::new(&doc).unwrap();
        for _ in 0..100_000 {
            evaluator
                .resolve(&literal, &Scope::default(), "/test", 0)
                .unwrap();
        }
        assert_eq!(
            evaluator
                .resolve(&literal, &Scope::default(), "/test", 0)
                .unwrap_err()
                .code,
            "evaluation_limit"
        );
    }

    #[test]
    fn typed_values_and_strict_fields_are_not_coerced() {
        let rounded = json!({"op":"typed","type":"f32","value":0.1});
        assert_eq!(
            numeric(&evaluate(&rounded).unwrap(), "/").unwrap(),
            0.1f32 as f64
        );
        let fractional = json!({"op":"typed","type":"int","value":1.1});
        assert_eq!(
            evaluate(&fractional).unwrap_err().message,
            "Expected int (signed 32-bit integer)"
        );
        assert_eq!(
            units::field(0.0.into(), units::LENGTH, "/", true).unwrap(),
            0.0
        );
        assert_eq!(
            units::field(1.0.into(), units::LENGTH, "/", true)
                .unwrap_err()
                .code,
            "unit_mismatch"
        );
        assert_eq!(
            units::unary("sin", 90.0.into(), "/", true)
                .unwrap_err()
                .code,
            "unit_mismatch"
        );
    }

    #[test]
    fn function_arguments_bind_without_caller_locals_and_geometry_is_deferred() {
        let doc = json!({"parameters":[{"id":"Width","value":4}],"functions":[{"id":"Add","kind":"scalar","parameters":["x"],"body":{"op":"add","args":[{"local":"x"},{"param":"Width"}]}},{"id":"Shape","kind":"geometry","parameters":["r"],"nodes":[],"root":"Sphere"}]});
        let call = json!({"op":"call","function":"Add","args":{"x":2}});
        let shape = json!({"op":"geometry","function":"Shape","args":{"r":2}});
        let wrong = json!({"op":"call","function":"Add","args":{"z":2}});
        let mut evaluator = Evaluator::new(&doc).unwrap();
        assert_eq!(
            evaluator
                .evaluate(&call, &Scope::default(), "/", 0)
                .unwrap(),
            6.0
        );
        assert!(matches!(
            evaluator
                .resolve(&shape, &Scope::default(), "/", 0)
                .unwrap(),
            Value::Geometry {
                function: "Shape",
                ..
            }
        ));
        assert_eq!(
            evaluator
                .resolve(&wrong, &Scope::default(), "/", 0)
                .unwrap_err()
                .code,
            "invalid_arguments"
        );
    }

    #[test]
    fn reports_retain_paths_dimensions_and_all_failed_constraints() {
        let doc = json!({"parameters":[],"constraints":[{"id":"A","left":1,"relation":"eq","right":2,"message":"A failed"},{"id":"B","left":3,"relation":"lt","right":2,"message":"B failed"}]});
        let error = Evaluator::new(&doc)
            .unwrap()
            .validate_checks()
            .err()
            .unwrap();
        assert_eq!(error.code, "constraint_failed");
        assert!(error
            .message
            .contains("A failed (actual: 1.0; required: == 2.0"));
        assert!(error
            .message
            .contains("B failed (actual: 3.0; required: < 2.0"));
        let report = error.details.unwrap();
        assert_eq!(report.as_array().unwrap().len(), 2);
        assert_eq!(report[1]["path"], "/constraints/1");
        assert_eq!(report[0]["dimension"], json!([0, 0]));
        let doc = json!({"parameters":[],"root":"Root","geometry_assertions":[{"id":"Size","target":"Root","check":"width","expected":{"op":"quantity","value":2,"unit":"cm"},"message":"Width"}]});
        let checks = Evaluator::new(&doc).unwrap().validate_checks().unwrap();
        assert_eq!(checks.geometry_assertions[0]["expected"], 20.0);
        assert_eq!(checks.geometry_assertions[0]["tolerance"], 0.0);
    }
}
