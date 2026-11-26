use super::*;
use crate::{
    distance_bounds::Interval,
    formula::{self, Token},
    function_surface::{self, Request},
};

fn enclose(tokens: &[Token], u: Interval, v: Interval) -> Result<[f64; 2]> {
    let mut stack = Vec::with_capacity(tokens.len());
    let pop = |stack: &mut Vec<Interval>| {
        stack
            .pop()
            .ok_or_else(|| crate::input("Formula is missing an operand"))
    };
    for token in tokens {
        let next = match *token {
            Token::Constant(x) => Interval::point(x),
            Token::U => u,
            Token::V => v,
            Token::T => return Err(crate::input("Use u and v in a surface formula")),
            Token::Negate => Interval::point(0.).sub(pop(&mut stack)?)?,
            Token::Add | Token::Subtract | Token::Multiply | Token::Divide => {
                let right = pop(&mut stack)?;
                let left = pop(&mut stack)?;
                match token {
                    Token::Add => left.add(right)?,
                    Token::Subtract => left.sub(right)?,
                    Token::Multiply => left.mul(right)?,
                    _ => left.div(right)?,
                }
            }
        };
        stack.push(next);
    }
    check(
        stack.len() == 1,
        "Expected one result per coordinate formula",
    )?;
    let result = stack[0];
    Ok([result.lo, result.hi])
}

pub(super) fn build(v: &Value) -> Result<Value> {
    let [u_min, u_max, v_min, v_max] = field(v, "domain")?;
    let expressions = formula::tokens(&field::<Vec<Value>>(v, "expressions")?, true)?;
    let u_domain = Interval::new(u_min, u_max)?;
    let v_domain = Interval::new(v_min, v_max)?;
    for expression in &expressions {
        enclose(expression, u_domain, v_domain)?;
    }
    let outcome = function_surface::approximate_detailed(
        Request {
            domain: [[u_min, u_max], [v_min, v_max]],
            second_derivative_bounds: field(v, "second_derivative_bounds")?,
            tolerance: field(v, "tolerance_mm")?,
            max_cells: field(v, "max_cells")?,
        },
        |u, v| {
            Ok([
                enclose(&expressions[0], Interval::point(u), Interval::point(v))?,
                enclose(&expressions[1], Interval::point(u), Interval::point(v))?,
                enclose(&expressions[2], Interval::point(u), Interval::point(v))?,
            ])
        },
    )?;
    let r = outcome.report;
    Ok(json!({
        "surface":encode(r.surface)?,
        "interpolationErrorUpperMm":r.interpolation_error_upper,
        "sampleErrorUpperMm":r.sample_error_upper,
        "errorUpperMm":r.error_upper,
        "withinTolerance":r.within_tolerance,
        "cells":r.cells,
        "evaluations":r.evaluations,
        "gridStop":format!("{:?}", outcome.grid_stop),
        "certificateOutcome":format!("{:?}", outcome.certificate),
        "requiresAuthoredGlobalDerivativeBounds":true,
        "solidTopologyCertified":false
    }))
}
