//! Structural serialization of the AST into MGV1 `Value`s, mirroring the
//! TypeScript AST shape from `src/services/openscadCompiler.ts` (object keys
//! match the TS field names; `undefined` optionals are omitted, `undef`
//! literals become null, non-finite numbers become `{"$number": ...}`).
use crate::Diagnostic;
use crate::ast::*;
use value_codec::{Number, Value, json};

fn number(v: f64) -> Value {
    match Number::from_f64(v) {
        Some(n) => Value::Number(n),
        None => {
            json!({"$number": if v.is_nan() { "NaN" } else if v > 0.0 { "Infinity" } else { "-Infinity" }})
        }
    }
}

fn literal(value: &LiteralValue) -> Value {
    match value {
        LiteralValue::Undef => Value::Null,
        LiteralValue::Bool(v) => Value::Bool(*v),
        LiteralValue::Number(v) => number(*v),
        LiteralValue::String(v) => Value::String(v.clone()),
    }
}

fn argument(arg: &ExpressionArgument) -> Value {
    let mut object = value_codec::Map::new();
    if let Some(name) = &arg.name {
        object.insert("name".into(), Value::String(name.clone()));
    }
    object.insert("value".into(), expr(&arg.value));
    object.insert("p".into(), json!(arg.p));
    object.insert("end".into(), json!(arg.end));
    Value::Object(object)
}

fn arguments(args: &[ExpressionArgument]) -> Value {
    Value::Array(args.iter().map(argument).collect())
}

fn params(params: &[ModuleParam]) -> Value {
    Value::Array(
        params
            .iter()
            .map(|param| {
                let mut object = value_codec::Map::new();
                object.insert("name".into(), Value::String(param.name.clone()));
                if let Some(default) = &param.default_value {
                    object.insert("defaultValue".into(), expr(default));
                }
                Value::Object(object)
            })
            .collect(),
    )
}

fn modifiers(modifiers: &[ViewportModifier]) -> Value {
    Value::Array(
        modifiers
            .iter()
            .map(|modifier| {
                json!({
                    "kind": modifier.kind,
                    "token": modifier.token.to_string(),
                    "span": {"start": modifier.start, "end": modifier.end},
                })
            })
            .collect(),
    )
}

fn statements(nodes: &[Statement]) -> Value {
    Value::Array(nodes.iter().map(statement).collect())
}

pub fn statement(node: &Statement) -> Value {
    match node {
        Statement::Call(call) => {
            let mut object = value_codec::Map::new();
            object.insert("type".into(), json!("call"));
            object.insert("name".into(), Value::String(call.name.clone()));
            object.insert("callArguments".into(), arguments(&call.call_arguments));
            object.insert(
                "args".into(),
                Value::Object(
                    call.args
                        .iter()
                        .map(|(k, v)| (k.clone(), expr(v)))
                        .collect(),
                ),
            );
            object.insert(
                "argKinds".into(),
                Value::Object(
                    call.arg_kinds
                        .iter()
                        .map(|(k, v)| (k.clone(), json!(*v)))
                        .collect(),
                ),
            );
            object.insert(
                "argSpans".into(),
                Value::Object(
                    call.arg_spans
                        .iter()
                        .map(|(k, (start, end))| (k.clone(), json!({"start": *start, "end": *end})))
                        .collect(),
                ),
            );
            object.insert("children".into(), statements(&call.children));
            object.insert("alternative".into(), statements(&call.alternative));
            object.insert("p".into(), json!(call.p));
            object.insert("end".into(), json!(call.end));
            if !call.viewport_modifiers.is_empty() {
                object.insert(
                    "viewportModifiers".into(),
                    modifiers(&call.viewport_modifiers),
                );
            }
            if let Some(id) = &call.operation_id {
                object.insert("operationId".into(), Value::String(id.clone()));
            }
            Value::Object(object)
        }
        Statement::Assign(assign) => json!({
            "type": "assign",
            "name": assign.name,
            "value": expr(&assign.value),
            "p": assign.p,
            "end": assign.end,
        }),
        Statement::Module(module) => json!({
            "type": "module",
            "name": module.name,
            "params": params(&module.params),
            "children": statements(&module.children),
            "p": module.p,
            "end": module.end,
        }),
        Statement::Function(function) => json!({
            "type": "function",
            "name": function.name,
            "params": params(&function.params),
            "body": expr(&function.body),
            "p": function.p,
            "end": function.end,
        }),
        Statement::Directive(directive) => {
            let mut object = value_codec::Map::new();
            object.insert("type".into(), json!("directive"));
            object.insert("directive".into(), json!(directive.directive));
            object.insert("path".into(), Value::String(directive.path.clone()));
            object.insert(
                "pathSpan".into(),
                json!({"start": directive.path_span.0, "end": directive.path_span.1}),
            );
            object.insert("p".into(), json!(directive.p));
            object.insert("end".into(), json!(directive.end));
            if !directive.viewport_modifiers.is_empty() {
                object.insert(
                    "viewportModifiers".into(),
                    modifiers(&directive.viewport_modifiers),
                );
            }
            Value::Object(object)
        }
    }
}

pub fn expr(node: &Expr) -> Value {
    let mut object = value_codec::Map::new();
    object.insert("kind".into(), json!(node.kind_name()));
    match node {
        Expr::Literal { value, p } => {
            object.insert("value".into(), literal(value));
            object.insert("p".into(), json!(*p));
        }
        Expr::Identifier { name, p } => {
            object.insert("name".into(), Value::String(name.clone()));
            object.insert("p".into(), json!(*p));
        }
        Expr::Vector { items, p } => {
            object.insert(
                "items".into(),
                Value::Array(items.iter().map(expr).collect()),
            );
            object.insert("p".into(), json!(*p));
        }
        Expr::Range {
            start,
            step,
            end,
            p,
        } => {
            object.insert("start".into(), expr(start));
            if let Some(step) = step {
                object.insert("step".into(), expr(step));
            }
            object.insert("end".into(), expr(end));
            object.insert("p".into(), json!(*p));
        }
        Expr::Unary { op, value, p } => {
            object.insert("op".into(), json!(op.name()));
            object.insert("value".into(), expr(value));
            object.insert("p".into(), json!(*p));
        }
        Expr::Binary { op, left, right, p } => {
            object.insert("op".into(), json!(op.name()));
            object.insert("left".into(), expr(left));
            object.insert("right".into(), expr(right));
            object.insert("p".into(), json!(*p));
        }
        Expr::Ternary { test, yes, no, p } => {
            object.insert("test".into(), expr(test));
            object.insert("yes".into(), expr(yes));
            object.insert("no".into(), expr(no));
            object.insert("p".into(), json!(*p));
        }
        Expr::Function {
            params: ps,
            body,
            p,
        } => {
            object.insert("params".into(), params(ps));
            object.insert("body".into(), expr(body));
            object.insert("p".into(), json!(*p));
        }
        Expr::Call {
            name,
            callee,
            args,
            p,
        } => {
            match name {
                Some(name) => object.insert("name".into(), Value::String(name.clone())),
                None => object.insert("name".into(), Value::Null),
            };
            object.insert("callee".into(), expr(callee));
            object.insert("args".into(), arguments(args));
            object.insert("p".into(), json!(*p));
        }
        Expr::Index { value, index, p } => {
            object.insert("value".into(), expr(value));
            object.insert("index".into(), expr(index));
            object.insert("p".into(), json!(*p));
        }
        Expr::Member { value, name, p } => {
            object.insert("value".into(), expr(value));
            object.insert("name".into(), Value::String(name.clone()));
            object.insert("p".into(), json!(*p));
        }
        Expr::Let { args, body, p } => {
            object.insert("args".into(), arguments(args));
            object.insert("body".into(), expr(body));
            object.insert("p".into(), json!(*p));
        }
        Expr::Assert { args, body, p } | Expr::Echo { args, body, p } => {
            object.insert("args".into(), arguments(args));
            if let Some(body) = body {
                object.insert("body".into(), expr(body));
            }
            object.insert("p".into(), json!(*p));
        }
        Expr::LcFor { args, body, p } => {
            object.insert("args".into(), arguments(args));
            object.insert("body".into(), expr(body));
            object.insert("p".into(), json!(*p));
        }
        Expr::LcForC {
            init,
            condition,
            update,
            body,
            p,
        } => {
            object.insert("init".into(), arguments(init));
            object.insert("condition".into(), expr(condition));
            object.insert("update".into(), arguments(update));
            object.insert("body".into(), expr(body));
            object.insert("p".into(), json!(*p));
        }
        Expr::LcIf {
            condition,
            yes,
            no,
            p,
        } => {
            object.insert("condition".into(), expr(condition));
            object.insert("yes".into(), expr(yes));
            if let Some(no) = no {
                object.insert("no".into(), expr(no));
            }
            object.insert("p".into(), json!(*p));
        }
        Expr::LcLet { args, body, p } => {
            object.insert("args".into(), arguments(args));
            object.insert("body".into(), expr(body));
            object.insert("p".into(), json!(*p));
        }
        Expr::LcEach { value, p } => {
            object.insert("value".into(), expr(value));
            object.insert("p".into(), json!(*p));
        }
    }
    Value::Object(object)
}

pub fn diagnostic(d: &Diagnostic) -> Value {
    json!({
        "code": d.code,
        "message": d.message,
        "start": d.start,
        "end": d.end,
        "line": d.line,
        "column": d.column,
    })
}

pub fn program(nodes: &[Statement]) -> Value {
    statements(nodes)
}
