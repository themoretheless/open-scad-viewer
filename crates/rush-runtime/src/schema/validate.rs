use super::*;

#[derive(Clone, Copy)]
pub(super) enum Rule {
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
pub(super) enum Presence {
    Required,
    Optional,
    Number(f64),
    Bool(bool),
}

#[derive(Clone, Copy)]
pub(super) struct Field {
    pub(super) name: &'static str,
    pub(super) rule: Rule,
    pub(super) presence: Presence,
}

pub(super) const fn required(name: &'static str, rule: Rule) -> Field {
    Field {
        name,
        rule,
        presence: Presence::Required,
    }
}
pub(super) const fn optional(name: &'static str, rule: Rule) -> Field {
    Field {
        name,
        rule,
        presence: Presence::Optional,
    }
}
pub(super) const fn number_default(name: &'static str, value: f64) -> Field {
    Field {
        name,
        rule: Rule::Expr,
        presence: Presence::Number(value),
    }
}
pub(super) const fn bool_default(name: &'static str, value: bool) -> Field {
    Field {
        name,
        rule: Rule::Bool,
        presence: Presence::Bool(value),
    }
}
pub(super) const ID: Field = required("id", Rule::Id);
pub(super) const OP: Field = required("op", Rule::Enum(&[]));
pub(super) const KIND: Field = required("kind", Rule::Enum(&[]));
pub(super) const INPUT: Field = required("input", Rule::Id);
pub(super) const RADIUS: Field = required("radius", Rule::Expr);
pub(super) const HEIGHT: Field = required("height", Rule::Expr);
pub(super) const CENTER: Field = bool_default("center", false);
pub(super) const PARAMETERS: Field = required("parameters", Rule::Array(&Rule::Id, 0, 32));
pub(super) const ROOT: Field = required("root", Rule::Id);
pub(super) const FUNCTION: Field = required("function", Rule::Id);
pub(super) const ARGS: Field = required("args", Rule::ExprRecord);
pub(super) const UNITS: &[&str] = &["mm", "cm", "m", "in", "deg", "rad"];

pub(super) fn invalid(path: &str, message: impl Into<String>) -> Error {
    Error::new("invalid_document", path, message.into())
}
pub(super) fn child_path(path: &str, key: impl std::fmt::Display) -> String {
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
            required("language", Rule::Literal("rush/ir-1")),
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

pub(super) fn object(value: &mut Value, path: &str, fields: &[Field]) -> Result<()> {
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
                    ));
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

pub(super) fn discriminator<'a>(value: &'a Value, path: &str, key: &str) -> Result<&'a str> {
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

pub(super) fn valid_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=32).contains(&bytes.len())
        && bytes[0].is_ascii_alphabetic()
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}

pub(super) fn rule(value: &mut Value, path: &str, expected: Rule) -> Result<()> {
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
                .is_some_and(|text| text.chars().count() <= 4096)
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
                .is_some_and(|text| (1..=256).contains(&text.chars().count()))
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
        Rule::GuardedStep => object(
            value,
            path,
            &[
                required("kind", Rule::Enum(&["assert", "where", "while"])),
                required("value", Rule::Expr),
            ],
        )?,
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

pub(super) fn array(value: &mut Value, path: &str, element: Rule, min: usize, max: usize) -> Result<()> {
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
