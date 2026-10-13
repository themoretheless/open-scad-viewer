use super::*;

pub(super) fn quoted_expression_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        out.push_str(match ch {
            '\t' => "\\t",
            '\n' => "\\n",
            '\r' => "\\r",
            '"' => "\\\"",
            '\\' => "\\\\",
            _ => {
                out.push(ch);
                continue;
            }
        });
    }
    out.push('"');
    out
}

pub(super) fn binary_token(operator: TT) -> &'static str {
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

pub(super) fn format_expression_arguments(args: &[ExpressionArgument]) -> String {
    args.iter()
        .map(|argument| match &argument.name {
            None => format_expression(&argument.value),
            Some(name) => format!("{name} = {}", format_expression(&argument.value)),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn format_params(params: &[ModuleParam]) -> String {
    params
        .iter()
        .map(|parameter| match &parameter.default_value {
            None => parameter.name.clone(),
            Some(default) => format!("{} = {}", parameter.name, format_expression(default)),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Canonical source-shaped expression rendering used by `str(function-value)`.
pub fn format_expression(expression: &Expr) -> String {
    match expression {
        Expr::Literal { value, .. } => match value {
            crate::ast::LiteralValue::String(s) => quoted_expression_string(s),
            crate::ast::LiteralValue::Undef => "undef".to_string(),
            crate::ast::LiteralValue::Bool(v) => if *v { "true" } else { "false" }.to_string(),
            crate::ast::LiteralValue::Number(v) => format_number(*v),
        },
        Expr::Identifier { name, .. } => name.clone(),
        Expr::Vector { items, .. } => format!(
            "[{}]",
            items
                .iter()
                .map(format_expression)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Range {
            start, step, end, ..
        } => format!(
            "[{} : {}{}]",
            format_expression(start),
            match step {
                None => String::new(),
                Some(step) => format!("{} : ", format_expression(step)),
            },
            format_expression(end)
        ),
        Expr::Unary { op, value, .. } => format!(
            "{}{}",
            match op {
                TT::Not => "!",
                TT::Minus => "-",
                _ => "+",
            },
            format_expression(value)
        ),
        Expr::Binary {
            op, left, right, ..
        } => format!(
            "({} {} {})",
            format_expression(left),
            binary_token(*op),
            format_expression(right)
        ),
        Expr::Ternary { test, yes, no, .. } => format!(
            "({} ? {} : {})",
            format_expression(test),
            format_expression(yes),
            format_expression(no)
        ),
        Expr::Function { params, body, .. } => {
            format!(
                "function({}) {}",
                format_params(params),
                format_expression(body)
            )
        }
        Expr::Call { callee, args, .. } => {
            let rendered = format_expression(callee);
            let callee_text = if matches!(**callee, Expr::Function { .. }) {
                format!("({rendered})")
            } else {
                rendered
            };
            format!("{callee_text}({})", format_expression_arguments(args))
        }
        Expr::Index { value, index, .. } => {
            format!("{}[{}]", format_expression(value), format_expression(index))
        }
        Expr::Member { value, name, .. } => format!("{}.{}", format_expression(value), name),
        Expr::Let { args, body, .. } => {
            format!(
                "let({}) {}",
                format_expression_arguments(args),
                format_expression(body)
            )
        }
        Expr::Assert { args, body, .. } => format!(
            "assert({}){}",
            format_expression_arguments(args),
            match body {
                None => String::new(),
                Some(body) => format!(" {}", format_expression(body)),
            }
        ),
        Expr::Echo { args, body, .. } => format!(
            "echo({}){}",
            format_expression_arguments(args),
            match body {
                None => String::new(),
                Some(body) => format!(" {}", format_expression(body)),
            }
        ),
        Expr::LcFor { args, body, .. } => format!(
            "for({}) ({})",
            format_expression_arguments(args),
            format_expression(body)
        ),
        Expr::LcForC {
            init,
            condition,
            update,
            body,
            ..
        } => format!(
            "for({}; {}; {}) ({})",
            format_expression_arguments(init),
            format_expression(condition),
            format_expression_arguments(update),
            format_expression(body)
        ),
        Expr::LcIf {
            condition, yes, no, ..
        } => format!(
            "if({}) ({}){}",
            format_expression(condition),
            format_expression(yes),
            match no {
                None => String::new(),
                Some(no) => format!(" else ({})", format_expression(no)),
            }
        ),
        Expr::LcLet { args, body, .. } => format!(
            "let({}) ({})",
            format_expression_arguments(args),
            format_expression(body)
        ),
        Expr::LcEach { value, .. } => format!("each ({})", format_expression(value)),
    }
}

pub(super) fn format_function(params: &[ModuleParam], body: &Expr) -> String {
    format!(
        "function({}) {}",
        format_params(params),
        format_expression(body)
    )
}

/// `formatValue` of the TS built-ins: bare strings at the top level, quoted
/// when nested, six-significant-digit numbers.
pub fn format_builtin_value(value: &Value, nested: bool) -> String {
    match value {
        Value::Undef => "undef".to_string(),
        Value::Bool(v) => if *v { "true" } else { "false" }.to_string(),
        Value::Number(v) => format_number(*v),
        Value::Str(s) => {
            if nested {
                format!("\"{s}\"")
            } else {
                s.to_string()
            }
        }
        Value::Vector(items) => format!(
            "[{}]",
            items
                .iter()
                .map(|item| format_builtin_value(item, true))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Range { start, step, end } => format!(
            "[{} : {} : {}]",
            format_number(*start),
            format_number(*step),
            format_number(*end)
        ),
        Value::Function(f) => format_function(f.params, f.body),
    }
}
