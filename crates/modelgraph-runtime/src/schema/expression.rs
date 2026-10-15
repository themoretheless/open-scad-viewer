use super::*;

pub(super) fn expression(value: &mut Value, path: &str) -> Result<()> {
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
            optional(
                "geometry_assertions",
                Rule::Array(&Rule::GeometryAssertion, 0, 64),
            ),
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
            ));
        }
    };
    object(value, path, fields)
}

pub(super) fn type_descriptor(value: &mut Value, path: &str) -> Result<()> {
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
pub(super) fn type_descriptor_depth(value: &Value, path: &str, depth: usize) -> Result<()> {
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

pub(super) fn pattern_literal(value: &Value) -> bool {
    value.is_number()
        || matches!(value["op"].as_str(), Some("text" | "quantity"))
        || (value["op"] == "negate"
            && pattern_literal(&value["value"])
            && value["value"]["op"] != "text")
}

pub(super) fn pattern_shape(value: &mut Value, path: &str) -> Result<()> {
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

pub(super) fn pattern_bindings(
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

pub(super) fn function(value: &mut Value, path: &str) -> Result<()> {
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
            ));
        }
    };
    object(value, path, fields)
}

pub(super) fn sketch_constraint(value: &mut Value, path: &str) -> Result<()> {
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
            ));
        }
    };
    object(value, path, fields)
}
