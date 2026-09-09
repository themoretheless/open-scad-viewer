//! Strict ModelGraph/1 validation and default insertion.
//!
//! Dispatch by `op`/`kind` once per object instead of probing a recursive union.
//! The input is normalized in place; expression subtrees are never cloned.
use crate::{Error, Result};
use value_codec::Value;

#[derive(Clone, Copy)]
enum Rule {
    Id,
    Number,
    Integer(i64, i64),
    Bool,
    Enum(&'static [&'static str]),
    Literal(&'static str),
    Message,
    Text,
    Expr,
    Node,
    Vector(usize),
    GeometryVector(usize),
    TypeDesc,
    TypeDescShape,
    TypeDescFields,
    Array(&'static Rule, usize, usize),
    ExprRecord,
    SortKey,
    Pattern,
    PatternShape,
    PatternFields,
    PatternLiteral,
    RestBinding,
    MatchArm(bool),
    Frame,
    LoftSection,
    Component,
    Anchor,
    Mate,
    Joint,
    SketchPoint,
    SketchConstraint,
    Parameter,
    Constraint,
    Function,
    Assertion,
    GeometryAssertion,
    GuardedStep,
}

#[derive(Clone, Copy)]
enum Presence {
    Required,
    Optional,
    Number(f64),
    Bool(bool),
}

#[derive(Clone, Copy)]
struct Field {
    name: &'static str,
    rule: Rule,
    presence: Presence,
}

const fn required(name: &'static str, rule: Rule) -> Field {
    Field {
        name,
        rule,
        presence: Presence::Required,
    }
}
const fn optional(name: &'static str, rule: Rule) -> Field {
    Field {
        name,
        rule,
        presence: Presence::Optional,
    }
}
const fn number_default(name: &'static str, value: f64) -> Field {
    Field {
        name,
        rule: Rule::Expr,
        presence: Presence::Number(value),
    }
}
const fn bool_default(name: &'static str, value: bool) -> Field {
    Field {
        name,
        rule: Rule::Bool,
        presence: Presence::Bool(value),
    }
}
const ID: Field = required("id", Rule::Id);
const OP: Field = required("op", Rule::Enum(&[]));
const KIND: Field = required("kind", Rule::Enum(&[]));
const INPUT: Field = required("input", Rule::Id);
const RADIUS: Field = required("radius", Rule::Expr);
const HEIGHT: Field = required("height", Rule::Expr);
const CENTER: Field = bool_default("center", false);
const PARAMETERS: Field = required("parameters", Rule::Array(&Rule::Id, 0, 32));
const ROOT: Field = required("root", Rule::Id);
const FUNCTION: Field = required("function", Rule::Id);
const ARGS: Field = required("args", Rule::ExprRecord);
const UNITS: &[&str] = &["mm", "cm", "m", "in", "deg", "rad"];

fn invalid(path: &str, message: impl Into<String>) -> Error {
    Error::new("invalid_document", path, message.into())
}
fn child_path(path: &str, key: impl std::fmt::Display) -> String {
    if path == "/" {
        format!("/{key}")
    } else {
        format!("{path}/{key}")
    }
}

/// Validate the complete input before recursively traversing its schema.
/// Counts primitives and container values alike, matching the former runtime.
pub fn validate(mut value: Value) -> Result<Value> {
    let mut pending = vec![(&value, 0usize)];
    let mut count = 0usize;
    while let Some((item, depth)) = pending.pop() {
        count += 1;
        if count > 20_000 || depth > 64 {
            return Err(Error::new(
                "input_limit",
                "/",
                "Document nesting or size limit exceeded.",
            ));
        }
        let children = match item {
            Value::Object(object) => object.len(),
            Value::Array(array) => array.len(),
            _ => 0,
        };
        // Do not allocate an unbounded work list for a wide invalid document.
        if children > 20_000 - count - pending.len() {
            return Err(Error::new(
                "input_limit",
                "/",
                "Document nesting or size limit exceeded.",
            ));
        }
        match item {
            Value::Object(object) => {
                pending.extend(object.values().map(|value| (value, depth + 1)))
            }
            Value::Array(array) => pending.extend(array.iter().map(|value| (value, depth + 1))),
            _ => {}
        }
    }
    object(
        &mut value,
        "/",
        &[
            required("language", Rule::Literal("modelgraph/1")),
            required("units", Rule::Literal("mm")),
            optional("type_policy", Rule::Enum(&["legacy", "strict"])),
            optional("constraints", Rule::Array(&Rule::Constraint, 0, 64)),
            required("parameters", Rule::Array(&Rule::Parameter, 0, 64)),
            required("nodes", Rule::Array(&Rule::Node, 1, 128)),
            ROOT,
            optional("functions", Rule::Array(&Rule::Function, 0, 32)),
            optional(
                "geometry_assertions",
                Rule::Array(&Rule::GeometryAssertion, 0, 64),
            ),
            optional("assertions", Rule::Array(&Rule::Assertion, 0, 64)),
            Field {
                name: "segments",
                rule: Rule::Integer(12, 128),
                presence: Presence::Number(48.0),
            },
        ],
    )?;
    Ok(value)
}

fn object(value: &mut Value, path: &str, fields: &[Field]) -> Result<()> {
    let map = value
        .as_object_mut()
        .ok_or_else(|| invalid(path, "Expected an object."))?;
    for field in fields {
        if !map.contains_key(field.name) {
            match field.presence {
                Presence::Required => {
                    return Err(invalid(
                        &child_path(path, field.name),
                        "Required field is missing.",
                    ))
                }
                Presence::Optional => continue,
                Presence::Number(number) => {
                    map.insert(
                        field.name.into(),
                        if number.fract() == 0.0 {
                            Value::from(number as i64)
                        } else {
                            Value::from(number)
                        },
                    );
                }
                Presence::Bool(boolean) => {
                    map.insert(field.name.into(), Value::Bool(boolean));
                }
            }
        }
        // Discriminators have already been checked by direct dispatch.
        if matches!(field.rule, Rule::Enum(values) if values.is_empty()) {
            continue;
        }
        rule(
            map.get_mut(field.name).unwrap(),
            &child_path(path, field.name),
            field.rule,
        )?;
    }
    if let Some(key) = map
        .keys()
        .find(|key| !fields.iter().any(|field| field.name == key.as_str()))
    {
        return Err(invalid(path, format!("Unrecognized key: {key}")));
    }
    Ok(())
}

fn discriminator<'a>(value: &'a Value, path: &str, key: &str) -> Result<&'a str> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(path, "Expected an object."))?;
    object.get(key).and_then(Value::as_str).ok_or_else(|| {
        invalid(
            &child_path(path, key),
            format!("Expected {key} discriminator."),
        )
    })
}

fn valid_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=32).contains(&bytes.len())
        && bytes[0].is_ascii_alphabetic()
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}

fn rule(value: &mut Value, path: &str, expected: Rule) -> Result<()> {
    match expected {
        Rule::Id => {
            if !value.as_str().is_some_and(valid_id) {
                return Err(invalid(
                    path,
                    "Expected an identifier matching [A-Za-z][A-Za-z0-9_]{0,31}.",
                ));
            }
        }
        Rule::Number => {
            if !value.as_f64().is_some_and(|number| {
                number.is_finite() && (-1_000_000.0..=1_000_000.0).contains(&number)
            }) {
                return Err(invalid(
                    path,
                    "Expected a finite number between -1000000 and 1000000.",
                ));
            }
        }
        Rule::Integer(min, max) => {
            if !value.as_f64().is_some_and(|number| {
                number.is_finite()
                    && number.fract() == 0.0
                    && number >= min as f64
                    && number <= max as f64
            }) {
                return Err(invalid(
                    path,
                    format!("Expected an integer between {min} and {max}."),
                ));
            }
        }
        Rule::Bool => {
            if !value.is_boolean() {
                return Err(invalid(path, "Expected a boolean."));
            }
        }
        Rule::Enum(options) => {
            if !value.as_str().is_some_and(|text| options.contains(&text)) {
                return Err(invalid(
                    path,
                    format!("Expected one of {}.", options.join(", ")),
                ));
            }
        }
        Rule::Literal(expected) => {
            if value.as_str() != Some(expected) {
                return Err(invalid(path, format!("Expected {expected}.")));
            }
        }
        Rule::Text => {
            if !value
                .as_str()
                .is_some_and(|text| text.encode_utf16().count() <= 4096)
            {
                return Err(Error::new(
                    "schema_error",
                    path,
                    "Expected a string of at most 4096 characters",
                ));
            }
        }
        Rule::Message => {
            if !value
                .as_str()
                .is_some_and(|text| (1..=256).contains(&text.encode_utf16().count()))
            {
                return Err(invalid(
                    path,
                    "Expected a message between 1 and 256 characters.",
                ));
            }
        }
        Rule::SortKey => object(
            value,
            path,
            &[
                required("function", Rule::Expr),
                required("descending", Rule::Bool),
            ],
        )?,
        Rule::Pattern => {
            pattern_shape(value, path)?;
            pattern_bindings(value, path, 0, &mut 0)?;
        }
        Rule::PatternShape => pattern_shape(value, path)?,
        Rule::PatternFields => {
            let fields = value
                .as_object_mut()
                .ok_or_else(|| invalid(path, "Expected pattern fields."))?;
            if fields.len() > 256 {
                return Err(invalid(path, "Pattern fields exceed 256 entries."));
            }
            for (key, pattern) in fields {
                let child = child_path(path, key);
                if !valid_id(key) {
                    return Err(invalid(&child, "Invalid pattern field name."));
                }
                pattern_shape(pattern, &child)?;
            }
        }
        Rule::PatternLiteral => {
            if !pattern_literal(value) {
                return Err(invalid(
                    path,
                    "Pattern literal must be a number, quantity or string constant.",
                ));
            }
            expression(value, path)?;
        }
        Rule::RestBinding => {
            if value != "_" {
                rule(value, path, Rule::Id)?;
            }
        }
        Rule::MatchArm(geometry) => object(
            value,
            path,
            &[
                required("pattern", Rule::Pattern),
                optional("guard", Rule::Expr),
                if geometry {
                    INPUT
                } else {
                    required("body", Rule::Expr)
                },
            ],
        )?,
        Rule::Expr => expression(value, path)?,
        Rule::Node => node(value, path)?,
        Rule::Vector(size) => array(value, path, Rule::Expr, size, size)?,
        Rule::GeometryVector(size) => {
            if value.is_array() {
                array(value, path, Rule::Expr, size, size)?;
            } else {
                expression(value, path)?;
            }
        }
        Rule::TypeDesc => {
            type_descriptor(value, path)?;
            type_descriptor_depth(value, path, 0)?;
        }
        Rule::TypeDescShape => type_descriptor(value, path)?,
        Rule::TypeDescFields => {
            let fields = value
                .as_object_mut()
                .ok_or_else(|| invalid(path, "Expected type descriptor fields."))?;
            if fields.len() > 32 {
                return Err(invalid(path, "Type descriptor exceeds 32 fields."));
            }
            for (name, descriptor) in fields {
                let child = child_path(path, name);
                if !valid_id(name) {
                    return Err(invalid(&child, "Invalid field name."));
                }
                type_descriptor(descriptor, &child)?;
            }
        }
        Rule::Array(element, min, max) => array(value, path, *element, min, max)?,
        Rule::ExprRecord => {
            let map = value
                .as_object_mut()
                .ok_or_else(|| invalid(path, "Expected an argument object."))?;
            for (key, value) in map {
                let key_path = child_path(path, key);
                if !valid_id(key) {
                    return Err(invalid(&key_path, "Invalid argument identifier."));
                }
                expression(value, &key_path)?;
            }
        }
        Rule::Frame => object(
            value,
            path,
            &[
                required("origin", Rule::Vector(3)),
                required("rotation", Rule::Vector(3)),
            ],
        )?,
        Rule::LoftSection => object(
            value,
            path,
            &[
                required("z", Rule::Expr),
                required("scale", Rule::Vector(2)),
                required("offset", Rule::Vector(2)),
            ],
        )?,
        Rule::Component => object(
            value,
            path,
            &[
                ID,
                INPUT,
                required("anchors", Rule::Array(&Rule::Anchor, 0, 32)),
                optional("placement", Rule::Frame),
                optional("mate", Rule::Mate),
            ],
        )?,
        Rule::Anchor => object(
            value,
            path,
            &[
                ID,
                required("origin", Rule::Vector(3)),
                required("rotation", Rule::Vector(3)),
            ],
        )?,
        Rule::Mate => object(
            value,
            path,
            &[
                required("component", Rule::Id),
                required("anchor", Rule::Id),
                required("own_anchor", Rule::Id),
                required("gap", Rule::Expr),
                required("rotation", Rule::Vector(3)),
                optional("joint", Rule::Joint),
            ],
        )?,
        Rule::Joint => object(
            value,
            path,
            &[
                required("kind", Rule::Enum(&["revolute", "slider"])),
                required("position", Rule::Expr),
                required("min", Rule::Expr),
                required("max", Rule::Expr),
            ],
        )?,
        Rule::SketchPoint => object(value, path, &[ID, required("position", Rule::Vector(2))])?,
        Rule::SketchConstraint => sketch_constraint(value, path)?,
        Rule::Parameter => object(
            value,
            path,
            &[
                ID,
                required("value", Rule::Number),
                optional("unit", Rule::Enum(UNITS)),
                optional("min", Rule::Number),
                optional("max", Rule::Number),
                optional("integer", Rule::Bool),
            ],
        )?,
        Rule::Constraint => object(
            value,
            path,
            &[
                ID,
                required("left", Rule::Expr),
                required("relation", Rule::Enum(&["le", "ge", "eq", "lt", "gt"])),
                required("right", Rule::Expr),
                optional("tolerance", Rule::Expr),
                required("message", Rule::Message),
            ],
        )?,
        Rule::Function => function(value, path)?,
        Rule::Assertion => object(
            value,
            path,
            &[
                required("condition", Rule::Expr),
                required("message", Rule::Message),
            ],
        )?,
        Rule::GuardedStep => object(value, path, &[
            required("kind", Rule::Enum(&["assert", "where", "while"])),
            required("value", Rule::Expr),
        ])?,
        Rule::GeometryAssertion => object(
            value,
            path,
            &[
                ID,
                required("target", Rule::Id),
                required(
                    "check",
                    Rule::Enum(&[
                        "hasBodies",
                        "isWatertight",
                        "hasNoDegenerateTriangles",
                        "height",
                        "width",
                        "depth",
                    ]),
                ),
                optional("expected", Rule::Expr),
                optional("tolerance", Rule::Expr),
                required("message", Rule::Message),
            ],
        )?,
    }
    Ok(())
}

fn array(value: &mut Value, path: &str, element: Rule, min: usize, max: usize) -> Result<()> {
    let values = value
        .as_array_mut()
        .ok_or_else(|| invalid(path, "Expected an array."))?;
    if !(min..=max).contains(&values.len()) {
        return Err(invalid(
            path,
            format!("Expected between {min} and {max} items."),
        ));
    }
    for (index, value) in values.iter_mut().enumerate() {
        rule(value, &child_path(path, index), element)?;
    }
    Ok(())
}

fn expression(value: &mut Value, path: &str) -> Result<()> {
    if value.is_number() {
        return rule(value, path, Rule::Number);
    }
    let map = value
        .as_object()
        .ok_or_else(|| invalid(path, "Expected a number or expression object."))?;
    if map.contains_key("param") {
        return object(value, path, &[required("param", Rule::Id)]);
    }
    if map.contains_key("local") {
        return object(value, path, &[required("local", Rule::Id)]);
    }
    if value["op"] == "query" {
        let method = value["method"].as_str().unwrap_or("").to_owned();
        if ["join", "groupJoin"].contains(&method.as_str()) {
            return object(
                value,
                path,
                &[
                    OP,
                    required("method", Rule::Id),
                    required("input", Rule::Expr),
                    required("argument", Rule::Expr),
                    required("functions", Rule::Array(&Rule::Expr, 3, 3)),
                ],
            );
        }
        if method == "scan" {
            return object(
                value,
                path,
                &[
                    OP,
                    required("method", Rule::Id),
                    required("input", Rule::Expr),
                    required("argument", Rule::Expr),
                    required("function", Rule::Expr),
                ],
            );
        }
        let callback =
            ["takeWhile", "skipWhile", "groupBy", "distinctBy", "all"].contains(&method.as_str());
        let optional_callback = [
            "count", "any", "first", "last", "single", "sum", "min", "max", "average",
        ]
        .contains(&method.as_str());
        let argument = [
            "take",
            "skip",
            "chunk",
            "window",
            "concat",
            "append",
            "prepend",
            "contains",
            "except",
            "intersect",
            "union",
        ]
        .contains(&method.as_str());
        let optional_argument =
            ["firstOrDefault", "lastOrDefault", "defaultIfEmpty"].contains(&method.as_str());
        let plain = ["reverse", "distinct", "flatten", "length"].contains(&method.as_str());
        if method == "orderBy" {
            return object(
                value,
                path,
                &[
                    OP,
                    required("method", Rule::Enum(&["orderBy"])),
                    required("input", Rule::Expr),
                    required("keys", Rule::Array(&Rule::SortKey, 1, 8)),
                ],
            );
        }
        if !(callback || optional_callback || argument || optional_argument || plain) {
            return Err(Error::new(
                "schema_error",
                path,
                "Unknown sequence query method",
            ));
        }
        let mut fields = vec![
            OP,
            required("method", Rule::Id),
            required("input", Rule::Expr),
        ];
        if callback {
            fields.push(required("function", Rule::Expr));
        }
        if optional_callback {
            fields.push(optional("function", Rule::Expr));
        }
        if argument {
            fields.push(required("argument", Rule::Expr));
        }
        if optional_argument {
            fields.push(optional("argument", Rule::Expr));
        }
        return object(value, path, &fields);
    }
    let fields: &[Field] = match discriminator(value, path, "op")? {
        "match" => &[
            OP,
            required("input", Rule::Expr),
            required("arms", Rule::Array(&Rule::MatchArm(false), 1, 32)),
        ],
        "text" => &[OP, required("value", Rule::Text)],
        "record" => &[OP, required("fields", Rule::ExprRecord)],
        "field" => &[
            OP,
            required("input", Rule::Expr),
            required("name", Rule::Id),
        ],
        "memo" => &[OP, ID, required("value", Rule::Expr)],
        "geometry_effects" => &[OP, INPUT, required("value", Rule::Expr)],
        "guarded" => &[
            OP,
            required("input", Rule::Expr),
            required("binding", Rule::Id),
            required("steps", Rule::Array(&Rule::GuardedStep, 0, 64)),
        ],
        "assert_value" => &[
            OP,
            optional("constraints", Rule::Array(&Rule::Constraint, 0, 64)),
            optional("geometry_assertions", Rule::Array(&Rule::GeometryAssertion, 0, 64)),
            required("value", Rule::Expr),
        ],
        "checked" => &[
            OP,
            required("checks", Rule::Array(&Rule::Expr, 0, 256)),
            required("value", Rule::Expr),
        ],
        "typed_value" => &[
            OP,
            required("value", Rule::Expr),
            required("type", Rule::TypeDesc),
        ],
        "typed" => &[
            OP,
            required(
                "type",
                Rule::Enum(&["int", "f32", "f64", "length", "angle"]),
            ),
            required("value", Rule::Expr),
        ],
        "quantity" => &[
            OP,
            required("value", Rule::Number),
            required("unit", Rule::Enum(UNITS)),
        ],
        "geometry" | "call" => &[OP, FUNCTION, ARGS],
        "lambda" => &[OP, PARAMETERS, required("body", Rule::Expr)],
        "apply" => &[
            OP,
            required("function", Rule::Expr),
            required("args", Rule::Array(&Rule::Expr, 0, 32)),
        ],
        "list" => &[OP, required("items", Rule::Array(&Rule::Expr, 0, 256))],
        "range" => &[
            OP,
            required("count", Rule::Expr),
            required("start", Rule::Expr),
            required("step", Rule::Expr),
        ],
        "interval" => &[
            OP,
            required("start", Rule::Expr),
            required("end", Rule::Expr),
            required("inclusive", Rule::Bool),
            optional("count", Rule::Expr),
            optional("step", Rule::Expr),
        ],
        "zip" => &[OP, required("inputs", Rule::Array(&Rule::Expr, 2, 8))],
        "enumerate" | "length" => &[OP, required("input", Rule::Expr)],
        "map" | "filter" | "flatmap" => &[
            OP,
            required("input", Rule::Expr),
            required("function", Rule::Expr),
        ],
        "reduce" => &[
            OP,
            required("input", Rule::Expr),
            required("function", Rule::Expr),
            required("initial", Rule::Expr),
        ],
        "at" => &[
            OP,
            required("input", Rule::Expr),
            required("index", Rule::Expr),
        ],
        "add" | "subtract" | "multiply" | "divide" | "min" | "max" | "pow" | "mod" | "lt"
        | "le" | "eq" | "and" | "or" => &[OP, required("args", Rule::Vector(2))],
        "negate" | "abs" | "sqrt" | "sin" | "cos" | "floor" | "ceil" | "not" => {
            &[OP, required("value", Rule::Expr)]
        }
        "if" => &[
            OP,
            required("condition", Rule::Expr),
            required("then", Rule::Expr),
            required("else", Rule::Expr),
        ],
        "let" => &[
            OP,
            required("name", Rule::Id),
            required("value", Rule::Expr),
            required("body", Rule::Expr),
        ],
        op => {
            return Err(invalid(
                &child_path(path, "op"),
                format!("Unknown expression operation: {op}"),
            ))
        }
    };
    object(value, path, fields)
}

fn type_descriptor(value: &mut Value, path: &str) -> Result<()> {
    object(
        value,
        path,
        &[
            required("name", Rule::Id),
            optional("args", Rule::Array(&Rule::TypeDescShape, 0, 32)),
            optional("fields", Rule::TypeDescFields),
        ],
    )?;
    let name = value["name"].as_str().unwrap();
    let count = value
        .get("args")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if name == "Vec" {
        if count != 1 || value.get("fields").is_some() {
            return Err(invalid(
                path,
                "Vec requires one type argument and no fields.",
            ));
        }
    } else if ["int", "f32", "f64", "length", "angle", "str"].contains(&name) {
        if count != 0 || value.get("fields").is_some() {
            return Err(invalid(
                path,
                "Primitive types do not accept arguments or fields.",
            ));
        }
    } else if value.get("fields").is_none() {
        return Err(invalid(
            path,
            "Named record types require a fields descriptor.",
        ));
    }
    Ok(())
}
fn type_descriptor_depth(value: &Value, path: &str, depth: usize) -> Result<()> {
    if depth > 16 {
        return Err(invalid(path, "Type descriptor depth 16 exceeded."));
    }
    if let Some(args) = value.get("args").and_then(Value::as_array) {
        for (index, arg) in args.iter().enumerate() {
            type_descriptor_depth(
                arg,
                &child_path(&child_path(path, "args"), index),
                depth + 1,
            )?;
        }
    }
    if let Some(fields) = value.get("fields").and_then(Value::as_object) {
        for (name, field) in fields {
            type_descriptor_depth(
                field,
                &child_path(&child_path(path, "fields"), name),
                depth + 1,
            )?;
        }
    }
    Ok(())
}

fn pattern_literal(value: &Value) -> bool {
    value.is_number()
        || matches!(value["op"].as_str(), Some("text" | "quantity"))
        || (value["op"] == "negate"
            && pattern_literal(&value["value"])
            && value["value"]["op"] != "text")
}

fn pattern_shape(value: &mut Value, path: &str) -> Result<()> {
    let fields: &[Field] = match discriminator(value, path, "kind")? {
        "wildcard" => &[KIND],
        "bind" => &[KIND, required("name", Rule::Id)],
        "literal" => &[KIND, required("value", Rule::PatternLiteral)],
        "as" => &[
            KIND,
            required("name", Rule::Id),
            required("pattern", Rule::PatternShape),
        ],
        "or" => &[
            KIND,
            required("patterns", Rule::Array(&Rule::PatternShape, 2, 32)),
        ],
        "range" => &[
            KIND,
            required("start", Rule::Expr),
            required("end", Rule::Expr),
            required("inclusive", Rule::Bool),
        ],
        "list" => &[
            KIND,
            required("prefix", Rule::Array(&Rule::PatternShape, 0, 256)),
            required("suffix", Rule::Array(&Rule::PatternShape, 0, 256)),
            optional("rest", Rule::RestBinding),
        ],
        "record" => &[
            KIND,
            required("fields", Rule::PatternFields),
            required("exact", Rule::Bool),
        ],
        "type" => &[
            KIND,
            required(
                "name",
                Rule::Enum(&[
                    "int", "f32", "f64", "str", "length", "angle", "list", "record",
                ]),
            ),
            required("pattern", Rule::PatternShape),
        ],
        kind => return Err(invalid(path, format!("Unknown pattern kind {kind}."))),
    };
    object(value, path, fields)
}

fn pattern_bindings(
    pattern: &Value,
    path: &str,
    depth: usize,
    count: &mut usize,
) -> Result<std::collections::HashSet<String>> {
    use std::collections::HashSet;
    *count += 1;
    if depth > 16 || *count > 256 {
        return Err(invalid(path, "Pattern depth 16 or size 256 exceeded."));
    }
    let mut bindings = HashSet::new();
    let mut merge = |names: HashSet<String>| -> Result<()> {
        for name in names {
            if !bindings.insert(name.clone()) {
                return Err(invalid(path, format!("Duplicate pattern binding {name}.")));
            }
        }
        Ok(())
    };
    match pattern["kind"].as_str().unwrap_or("") {
        "bind" => {
            merge(HashSet::from([pattern["name"]
                .as_str()
                .unwrap()
                .to_owned()]))?;
        }
        "as" | "type" => {
            merge(pattern_bindings(
                &pattern["pattern"],
                &child_path(path, "pattern"),
                depth + 1,
                count,
            )?)?;
            if pattern["kind"] == "as" {
                merge(HashSet::from([pattern["name"]
                    .as_str()
                    .unwrap()
                    .to_owned()]))?;
            }
        }
        "or" => {
            let mut previous = None;
            for (index, child) in pattern["patterns"].as_array().unwrap().iter().enumerate() {
                let child = pattern_bindings(child, &child_path(path, index), depth + 1, count)?;
                if previous.as_ref().is_some_and(|names| names != &child) {
                    return Err(invalid(path, "Every alternative must bind the same names."));
                }
                previous = Some(child);
            }
            merge(previous.unwrap_or_default())?;
        }
        "list" => {
            for field in ["prefix", "suffix"] {
                for (index, child) in pattern[field].as_array().unwrap().iter().enumerate() {
                    merge(pattern_bindings(
                        child,
                        &child_path(&child_path(path, field), index),
                        depth + 1,
                        count,
                    )?)?;
                }
            }
            if let Some(name) = pattern
                .get("rest")
                .and_then(Value::as_str)
                .filter(|name| *name != "_")
            {
                merge(HashSet::from([name.to_owned()]))?;
            }
        }
        "record" => {
            for (name, child) in pattern["fields"].as_object().unwrap() {
                merge(pattern_bindings(
                    child,
                    &child_path(path, name),
                    depth + 1,
                    count,
                )?)?;
            }
        }
        _ => {}
    }
    Ok(bindings)
}

fn function(value: &mut Value, path: &str) -> Result<()> {
    let fields: &[Field] = match discriminator(value, path, "kind")? {
        "scalar" | "value" => &[ID, KIND, PARAMETERS, required("body", Rule::Expr)],
        "geometry" => &[
            ID,
            KIND,
            PARAMETERS,
            required("nodes", Rule::Array(&Rule::Node, 1, 128)),
            ROOT,
        ],
        kind => {
            return Err(invalid(
                &child_path(path, "kind"),
                format!("Unknown function kind: {kind}"),
            ))
        }
    };
    object(value, path, fields)
}

fn sketch_constraint(value: &mut Value, path: &str) -> Result<()> {
    const A: Field = required("a", Rule::Id);
    const B: Field = required("b", Rule::Id);
    let fields: &[Field] = match discriminator(value, path, "kind")? {
        "fix" => &[
            ID,
            KIND,
            required("point", Rule::Id),
            required("at", Rule::Vector(2)),
        ],
        "horizontal" | "vertical" | "coincident" => &[ID, KIND, A, B],
        "distance" => &[ID, KIND, A, B, required("value", Rule::Expr)],
        "parallel" | "perpendicular" | "equal_length" => &[
            ID,
            KIND,
            A,
            B,
            required("c", Rule::Id),
            required("d", Rule::Id),
        ],
        kind => {
            return Err(invalid(
                &child_path(path, "kind"),
                format!("Unknown sketch constraint kind: {kind}"),
            ))
        }
    };
    object(value, path, fields)
}

fn node(value: &mut Value, path: &str) -> Result<()> {
    let fields: &[Field] = match discriminator(value, path, "op")? {
        "match" => &[
            ID,
            OP,
            required("value", Rule::Expr),
            required("arms", Rule::Array(&Rule::MatchArm(true), 1, 32)),
        ],
        "gear" => &[
            ID,
            OP,
            required("teeth", Rule::Expr),
            required("module", Rule::Expr),
            number_default("pressure_angle", 20.0),
            required("thickness", Rule::Expr),
            number_default("bore", 0.0),
            number_default("backlash", 0.15),
            number_default("clearance", 0.5),
            bool_default("internal", false),
            number_default("rim_width", 6.0),
            number_default("flank_segments", 6.0),
        ],
        "planetary_spinner" => &[
            ID,
            OP,
            required("inner_radius", Rule::Expr),
            required("outer_radius", Rule::Expr),
            required("bore", Rule::Expr),
            required("gap", Rule::Expr),
            HEIGHT,
            required("helix_angle", Rule::Expr),
        ],
        "planetary_gears" => &[
            ID,
            OP,
            required("sun_teeth", Rule::Expr),
            required("planet_teeth", Rule::Expr),
            required("planet_count", Rule::Expr),
            required("module", Rule::Expr),
            number_default("pressure_angle", 20.0),
            required("thickness", Rule::Expr),
            number_default("bore", 0.0),
            number_default("backlash", 0.15),
            number_default("clearance", 0.5),
            number_default("rim_width", 6.0),
            number_default("flank_segments", 6.0),
            number_default("carrier_angle", 0.0),
        ],
        "thread" => &[
            ID,
            OP,
            required("diameter", Rule::Expr),
            required("pitch", Rule::Expr),
            required("length", Rule::Expr),
            bool_default("internal", false),
            number_default("wall", 3.0),
            number_default("clearance", 0.2),
            number_default("starts", 1.0),
            bool_default("left_handed", false),
            number_default("segments_per_turn", 32.0),
        ],
        "affine" => &[
            ID,
            OP,
            INPUT,
            required("rows", Rule::Array(&Rule::Vector(4), 3, 3)),
        ],
        "hull" | "union" | "intersection" => {
            &[ID, OP, required("inputs", Rule::Array(&Rule::Id, 1, 32))]
        }
        "mirror" => &[ID, OP, required("normal", Rule::Vector(3)), INPUT],
        "offset" => &[
            ID,
            OP,
            required("distance", Rule::Expr),
            INPUT,
            optional("mode", Rule::Enum(&["radius", "delta"])),
        ],
        "projection" => &[ID, OP, INPUT],
        "section" => &[ID, OP, HEIGHT, INPUT],
        "advanced_extrude" => &[
            ID,
            OP,
            INPUT,
            HEIGHT,
            required("twist", Rule::Expr),
            required("top_scale", Rule::Vector(2)),
            required("slices", Rule::Integer(1, 64)),
            CENTER,
        ],
        "cone" => &[
            ID,
            OP,
            required("radius_bottom", Rule::Expr),
            required("radius_top", Rule::Expr),
            HEIGHT,
            CENTER,
        ],
        "torus" => &[
            ID,
            OP,
            required("major_radius", Rule::Expr),
            required("minor_radius", Rule::Expr),
        ],
        "linear_pattern" => &[
            ID,
            OP,
            INPUT,
            required("count", Rule::Expr),
            required("step", Rule::Vector(3)),
        ],
        "circular_pattern" => &[
            ID,
            OP,
            INPUT,
            required("count", Rule::Expr),
            required("angle_step", Rule::Expr),
        ],
        "loft" => &[
            ID,
            OP,
            required("profile", Rule::Array(&Rule::Vector(2), 3, 64)),
            required("sections", Rule::Array(&Rule::LoftSection, 2, 32)),
        ],
        "assembly" => &[
            ID,
            OP,
            required("components", Rule::Array(&Rule::Component, 1, 32)),
        ],
        "sketch" => &[
            ID,
            OP,
            required("points", Rule::Array(&Rule::SketchPoint, 3, 16)),
            required("boundary", Rule::Array(&Rule::Id, 3, 16)),
            required("constraints", Rule::Array(&Rule::SketchConstraint, 0, 48)),
            bool_default("allow_underconstrained", false),
        ],
        "rectangle" => &[ID, OP, required("size", Rule::GeometryVector(2)), CENTER],
        "circle" | "sphere" => &[ID, OP, RADIUS],
        "polygon" => &[
            ID,
            OP,
            required("points", Rule::Array(&Rule::Vector(2), 3, 256)),
        ],
        "extrude" => &[ID, OP, INPUT, HEIGHT, CENTER],
        "revolve" => &[ID, OP, INPUT, required("angle", Rule::Expr)],
        "assert" => &[
            ID, OP, INPUT,
            optional("constraints", Rule::Array(&Rule::Constraint, 0, 64)),
            optional("geometry_assertions", Rule::Array(&Rule::GeometryAssertion, 0, 64)),
        ],
        "evaluate" => &[ID, OP, required("value", Rule::Expr)],
        "call" => &[ID, OP, FUNCTION, ARGS],
        "if" => &[
            ID,
            OP,
            required("condition", Rule::Expr),
            required("then", Rule::Id),
            required("else", Rule::Id),
        ],
        "collect" => &[
            ID,
            OP,
            required("values", Rule::Expr),
            required("binding", Rule::Id),
            INPUT,
        ],
        "group" => &[ID, OP, required("inputs", Rule::Array(&Rule::Id, 0, 256))],
        "map" => &[
            ID,
            OP,
            required("count", Rule::Expr),
            required("index", Rule::Id),
            INPUT,
        ],
        "box" => &[ID, OP, required("size", Rule::GeometryVector(3)), CENTER],
        "cylinder" => &[ID, OP, RADIUS, HEIGHT, CENTER],
        "translate" | "rotate" | "scale" => {
            &[ID, OP, required("vector", Rule::GeometryVector(3)), INPUT]
        }
        "difference" => &[
            ID,
            OP,
            required("base", Rule::Id),
            required("subtract", Rule::Array(&Rule::Id, 1, 32)),
        ],
        op => {
            return Err(invalid(
                &child_path(path, "op"),
                format!("Unknown node operation: {op}"),
            ))
        }
    };
    object(value, path, fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;

    fn document(node: Value) -> Value {
        json!({"language":"modelgraph/1","units":"mm","parameters":[],"nodes":[node],"root":"shape"})
    }
    #[test]
    fn matches_original_zod_schema_corpus() {
        let corpus: Value =
            value_codec::from_str(include_str!("../tests/fixtures/schema-parity.json")).unwrap();
        for case in corpus["cases"].as_array().unwrap() {
            let result = validate(case["input"].clone());
            if case["error"] == true {
                assert!(result.is_err(), "{} unexpectedly accepted", case["name"]);
            } else {
                assert_eq!(
                    result.unwrap_or_else(|error| panic!("{}: {:?}", case["name"], error)),
                    case["expected"],
                    "{} defaults differ",
                    case["name"]
                );
            }
        }
    }
    #[test]
    fn reports_exact_nested_paths_and_budget_error_codes() {
        let error = validate(document(
            json!({"id":"shape","op":"box","size":[1,{"param":"bad-id"},3]}),
        ))
        .unwrap_err();
        assert_eq!(error.code, "invalid_document");
        assert_eq!(error.path, "/nodes/0/size/1/param");
        let error = validate(Value::Array(vec![Value::Null; 20_000])).unwrap_err();
        assert_eq!(error.code, "input_limit");
        assert_eq!(error.path, "/");
    }
    #[test]
    fn inserts_defaults_without_changing_expressions() {
        let input = document(
            json!({"id":"shape","op":"gear","teeth":{"param":"teeth"},"module":1,"thickness":5}),
        );
        let output = validate(input).unwrap();
        assert_eq!(output["segments"], json!(48));
        assert_eq!(output["nodes"][0]["teeth"], json!({"param":"teeth"}));
        assert_eq!(output["nodes"][0]["pressure_angle"], json!(20));
        assert_eq!(output["nodes"][0]["internal"], json!(false));
        assert_eq!(output["nodes"][0]["backlash"], json!(0.15));
    }
    #[test]
    fn rejects_unknown_keys_in_expressions_and_components() {
        let mut input = document(
            json!({"id":"shape","op":"sphere","radius":{"op":"add","args":[1,2],"extra":3}}),
        );
        assert!(validate(input.clone()).is_err());
        input["nodes"][0]["radius"] = json!({"param":"r","local":"r"});
        assert!(validate(input).is_err());
        let input = document(
            json!({"id":"shape","op":"assembly","components":[{"id":"part","input":"other","anchors":[],"placement":{"origin":[0,0,0],"rotation":[0,0,0],"extra":0}}]}),
        );
        assert!(validate(input).is_err());
    }
    #[test]
    fn validates_nested_function_and_sequence_variants() {
        let mut input = document(
            json!({"id":"shape","op":"evaluate","value":{"op":"reduce","input":{"op":"zip","inputs":[{"op":"interval","start":0,"end":5,"inclusive":false},{"op":"range","start":1,"step":2,"count":5}]},"function":{"op":"lambda","parameters":["acc","x"],"body":{"op":"add","args":[{"local":"acc"},{"op":"at","input":{"local":"x"},"index":0}]}},"initial":0}}),
        );
        input["functions"] = json!([{"id":"make","kind":"geometry","parameters":[],"nodes":[{"id":"box","op":"box","size":[1,2,3]}],"root":"box"}]);
        let output = validate(input).unwrap();
        assert_eq!(output["functions"][0]["nodes"][0]["center"], false);
    }
    #[test]
    fn enforces_input_budgets_before_schema_errors() {
        let mut value = Value::Null;
        for _ in 0..66 {
            value = Value::Array(vec![value]);
        }
        assert!(validate(value).is_err());
        assert!(validate(Value::Array(vec![Value::Null; 20_000])).is_err());
    }
    #[test]
    fn rejects_out_of_range_and_invalid_shapes() {
        for radius in [
            json!(1_000_001),
            json!(true),
            json!({"op":"bogus"}),
            json!({"local":"bad-id"}),
        ] {
            assert!(validate(document(
                json!({"id":"shape","op":"sphere","radius":radius})
            ))
            .is_err());
        }
        assert!(validate(document(json!({"id":"shape","op":"box","size":[1,2]}))).is_err());
        let mut input = document(json!({"id":"shape","op":"sphere","radius":1}));
        input["segments"] = json!(12.5);
        assert!(validate(input).is_err());
    }
    #[test]
    fn rejects_null_optional_values_and_checks_utf16_message_length() {
        let mut input = document(json!({"id":"shape","op":"sphere","radius":1}));
        input["type_policy"] = Value::Null;
        assert!(validate(input.clone()).is_err());
        input.as_object_mut().unwrap().remove("type_policy");
        input["assertions"] = json!([{"condition":1,"message":"😀".repeat(129)}]);
        assert!(validate(input).is_err());
    }
    #[test]
    fn pattern_schema_rejects_ambiguous_bindings_and_unknown_keys() {
        for pattern in [
            json!({"kind":"list","prefix":[{"kind":"bind","name":"x"},{"kind":"bind","name":"x"}],"suffix":[]}),
            json!({"kind":"as","name":"x","pattern":{"kind":"bind","name":"x"}}),
            json!({"kind":"or","patterns":[{"kind":"bind","name":"x"},{"kind":"wildcard"}]}),
            json!({"kind":"wildcard","ignored":1}),
            json!({"kind":"literal","value":{"local":"x"}}),
            json!({"kind":"type","name":"unknown","pattern":{"kind":"wildcard"}}),
            json!({"kind":"list","prefix":[],"suffix":[],"rest":"bad-name"}),
        ] {
            let input = document(
                json!({"id":"shape","op":"sphere","radius":{"op":"match","input":1,"arms":[{"pattern":pattern,"body":2}]}}),
            );
            assert!(validate(input).is_err());
        }
    }

    #[test]
    fn pattern_schema_enforces_arms_nesting_and_total_size_limits() {
        let valid = json!({"pattern":{"kind":"wildcard"},"body":1});
        let input = document(
            json!({"id":"shape","op":"sphere","radius":{"op":"match","input":1,"arms":vec![valid;33]}}),
        );
        assert!(validate(input).is_err());
        let mut deep = json!({"kind":"wildcard"});
        for _ in 0..18 {
            deep = json!({"kind":"type","name":"int","pattern":deep});
        }
        let wide = json!({"kind":"list","prefix":vec![json!({"kind":"wildcard"});256],"suffix":[]});
        for pattern in [deep, wide] {
            let input = document(
                json!({"id":"shape","op":"sphere","radius":{"op":"match","input":1,"arms":[{"pattern":pattern,"body":2}]}}),
            );
            assert!(validate(input).is_err());
        }
    }
    #[test]
    fn type_descriptors_are_strict_and_bounded() {
        let mut deep = json!({"name":"int"});
        for _ in 0..18 {
            deep = json!({"name":"Vec","args":[deep]});
        }
        let mut wide = json!({"name":"Wide","fields":{}});
        for index in 0..33 {
            wide["fields"][&format!("f{index}")] = json!({"name":"int"});
        }
        for descriptor in [
            deep,
            wide,
            json!({"name":"Vec"}),
            json!({"name":"Unknown"}),
            json!({"name":"str","fields":{}}),
            json!({"name":"int","args":[{"name":"int"}]}),
            json!({"name":"int","unknown":1}),
            json!({"name":"Point","fields":{"bad-name":{"name":"int"}}}),
        ] {
            let input = document(
                json!({"id":"shape","op":"sphere","radius":{"op":"typed_value","value":1,"type":descriptor}}),
            );
            assert!(validate(input).is_err());
        }
    }

    #[test]
    fn dynamic_vectors_do_not_relax_numeric_or_other_surface_shapes() {
        for node in [
            json!({"id":"shape","op":"sphere","radius":{"op":"add","args":{"op":"list","items":[1,2]}}}),
            json!({"id":"shape","op":"mirror","input":"other","normal":{"op":"list","items":[1,0,0]}}),
            json!({"id":"shape","op":"polygon","points":{"op":"list","items":[]}}),
        ] {
            assert!(validate(document(node)).is_err());
        }
    }
}
