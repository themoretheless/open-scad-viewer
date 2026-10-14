//! Bounded rational polynomial expressions in reverse Polish notation.
//! Algebraic basis conversion, not sampled fitting. Arithmetic uses binary64.
use crate::{Result, check, curve::Curve, polynomial, surface::Surface};
#[cfg(feature = "codec")]
mod serialization;
mod text;
#[cfg(feature = "codec")]
pub use serialization::tokens;

/// A bounded reverse Polish formula instruction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    Constant(f64),
    T,
    U,
    V,
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Parse bounded infix text directly into native instructions.
pub fn parse(source: &str, surface: bool) -> Result<Vec<Token>> {
    text::parse(source, surface)
}

type Poly = Vec<Vec<f64>>;
#[derive(Clone)]
struct Fraction {
    numerator: Poly,
    denominator: Poly,
}
fn constant(x: f64) -> Poly {
    vec![vec![x]]
}
fn coefficient(p: &Poly, i: usize, j: usize) -> f64 {
    p.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0.)
}

fn checked(mut p: Poly) -> Result<Poly> {
    while p.len() > 1 && p.last().unwrap().iter().all(|x| *x == 0.) {
        p.pop();
    }
    while p[0].len() > 1 && p.iter().all(|row| row.last() == Some(&0.)) {
        for row in &mut p {
            row.pop();
        }
    }
    check(
        p.len() <= 13 && p[0].len() <= 13,
        "Formula degree exceeds 12 per axis; simplify or split the expression",
    )?;
    check(
        p.iter().flatten().all(|c| c.is_finite()),
        "Formula coefficient overflow",
    )?;
    Ok(p)
}
fn add(a: &Poly, b: &Poly, sign: f64) -> Result<Poly> {
    checked(
        (0..a.len().max(b.len()))
            .map(|i| {
                (0..a[0].len().max(b[0].len()))
                    .map(|j| coefficient(a, i, j) + sign * coefficient(b, i, j))
                    .collect()
            })
            .collect(),
    )
}
fn multiply(a: &Poly, b: &Poly) -> Result<Poly> {
    if a == &constant(0.) || b == &constant(0.) {
        return Ok(constant(0.));
    }
    let nu = a.len() + b.len() - 1;
    let nv = a[0].len() + b[0].len() - 1;
    check(
        nu <= 13 && nv <= 13,
        "Formula degree exceeds 12 per axis; simplify or split the expression",
    )?;
    let mut p = vec![vec![0.; nv]; nu];
    for (i, row) in a.iter().enumerate() {
        for (j, x) in row.iter().enumerate() {
            for (k, row) in b.iter().enumerate() {
                for (l, y) in row.iter().enumerate() {
                    p[i + k][j + l] += x * y;
                }
            }
        }
    }
    checked(p)
}
fn expression(tokens: &[Token], surface: bool) -> Result<Fraction> {
    check(
        !tokens.is_empty() && tokens.len() <= 64,
        "Each formula requires 1..64 tokens",
    )?;
    let mut stack: Vec<Fraction> = Vec::new();
    for token in tokens {
        if let Token::Constant(value) = *token {
            check(value.is_finite(), "Formula constants must be finite")?;
            stack.push(Fraction {
                numerator: constant(value),
                denominator: constant(1.),
            });
            continue;
        }
        let op = *token;
        if matches!(
            (surface, op),
            (false, Token::T) | (true, Token::U | Token::V)
        ) {
            stack.push(Fraction {
                numerator: if op == Token::V {
                    vec![vec![0., 1.]]
                } else {
                    vec![vec![0.], vec![1.]]
                },
                denominator: constant(1.),
            });
            continue;
        }
        if op == Token::Negate {
            let a = stack
                .last_mut()
                .ok_or_else(|| crate::input("Formula neg requires one operand"))?;
            for x in a.numerator.iter_mut().flatten() {
                *x = -*x;
            }
            continue;
        }
        check(
            matches!(
                op,
                Token::Add | Token::Subtract | Token::Multiply | Token::Divide
            ),
            "Unknown formula operator or variable",
        )?;
        check(
            stack.len() >= 2,
            "Formula binary operator requires two operands",
        )?;
        let b = stack.pop().unwrap();
        let a = stack.pop().unwrap();
        let value = match op {
            Token::Add | Token::Subtract => {
                let sign = if op == Token::Add { 1. } else { -1. };
                if a.denominator == b.denominator {
                    Fraction {
                        numerator: add(&a.numerator, &b.numerator, sign)?,
                        denominator: a.denominator,
                    }
                } else {
                    Fraction {
                        numerator: add(
                            &multiply(&a.numerator, &b.denominator)?,
                            &multiply(&b.numerator, &a.denominator)?,
                            sign,
                        )?,
                        denominator: multiply(&a.denominator, &b.denominator)?,
                    }
                }
            }
            Token::Multiply => Fraction {
                numerator: multiply(&a.numerator, &b.numerator)?,
                denominator: multiply(&a.denominator, &b.denominator)?,
            },
            _ => {
                check(b.numerator != constant(0.), "Formula division by zero")?;
                Fraction {
                    numerator: multiply(&a.numerator, &b.denominator)?,
                    denominator: multiply(&a.denominator, &b.numerator)?,
                }
            }
        };
        stack.push(value);
    }
    check(stack.len() == 1, "Formula must leave exactly one result")?;
    Ok(stack.pop().unwrap())
}
fn homogeneous(expressions: &[Vec<Token>], surface: bool) -> Result<Vec<Vec<[f64; 4]>>> {
    check(
        expressions.len() == 3,
        "Spatial formula requires x, y and z expressions",
    )?;
    let fractions = expressions
        .iter()
        .map(|e| expression(e, surface))
        .collect::<Result<Vec<_>>>()?;
    let mut denominators: Vec<Poly> = Vec::new();
    let indices: Vec<usize> = fractions
        .iter()
        .map(|f| {
            if let Some(i) = denominators.iter().position(|d| d == &f.denominator) {
                i
            } else {
                denominators.push(f.denominator.clone());
                denominators.len() - 1
            }
        })
        .collect();
    let mut denominator = constant(1.);
    for d in &denominators {
        denominator = multiply(&denominator, d)?;
    }
    let mut numerators = Vec::new();
    for (axis, f) in fractions.iter().enumerate() {
        let mut n = f.numerator.clone();
        for (i, d) in denominators.iter().enumerate() {
            if i != indices[axis] {
                n = multiply(&n, d)?;
            }
        }
        numerators.push(n);
    }
    let nu = numerators
        .iter()
        .map(Vec::len)
        .chain([denominator.len()])
        .max()
        .unwrap();
    let nv = numerators
        .iter()
        .map(|p| p[0].len())
        .chain([denominator[0].len()])
        .max()
        .unwrap();
    Ok((0..nu)
        .map(|i| {
            (0..nv)
                .map(|j| {
                    [
                        coefficient(&numerators[0], i, j),
                        coefficient(&numerators[1], i, j),
                        coefficient(&numerators[2], i, j),
                        coefficient(&denominator, i, j),
                    ]
                })
                .collect()
        })
        .collect())
}
/// Three coordinate formulas, in model length units. `t` ranges over domain;
/// returned parameter ranges over [0,1]. Example `["t","t","*",2,"+"]`.
/// Each formula is bounded to 64 tokens; polynomial degrees must remain <=12.
/// Denominator cancellation is not inferred. Positive/common negative Bernstein
/// weights are required; a nonsingular expression may be conservatively rejected.
pub fn curve(domain: [f64; 2], expressions: &[Vec<Token>]) -> Result<Curve> {
    let coefficients = homogeneous(expressions, false)?
        .into_iter()
        .map(|row| row[0])
        .collect::<Vec<_>>();
    polynomial::rational_curve(domain, &coefficients)
}
/// Two-variable rational surface. `u` and `v` range over the supplied domain
/// [u0,u1,v0,v1], while output domains are [0,1]^2. Same token, degree and
/// denominator admission bounds as curve(). No regularity certificate is issued.
pub fn surface(domain: [f64; 4], expressions: &[Vec<Token>]) -> Result<Surface> {
    polynomial::rational_surface(domain, &homogeneous(expressions, true)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "codec")]
    use value_codec::{Value, json};

    #[cfg(feature = "codec")]
    fn expressions(v: Value) -> Vec<Vec<Token>> {
        value_codec::from_value(v).unwrap()
    }
    #[test]
    #[cfg(feature = "codec")]
    fn infix_text_matches_rpn_with_precedence_parentheses_and_reciprocals() {
        let p = curve(
            [-2., 3.],
            &tokens(&vec![json!("t"), json!("-t^2+2"), json!("3*(t+1)")], false).unwrap(),
        )
        .unwrap();
        let s = surface(
            [0., 1., 0., 1.],
            &tokens(
                &vec![
                    json!("u/(1+u*v)"),
                    json!("v*(1+u*v)^-1"),
                    json!(".5e1*(u^3-3*u*v^2)"),
                ],
                true,
            )
            .unwrap(),
        )
        .unwrap();
        for i in 0..=20 {
            let normalized_u = i as f64 / 20.;
            let t = -2. + 5. * normalized_u;
            let q = p.evaluate(normalized_u).unwrap().point;
            assert!(
                (q[0] - t).abs() < 1e-12
                    && (q[1] + t * t - 2.).abs() < 1e-12
                    && (q[2] - 3. * (t + 1.)).abs() < 1e-12
            );
            for j in 0..=20 {
                let v = j as f64 / 20.;
                let q = s.evaluate(normalized_u, v).unwrap().point;
                assert!(
                    (q[0] - normalized_u / (1. + normalized_u * v)).abs() < 1e-12
                        && (q[1] - v / (1. + normalized_u * v)).abs() < 1e-12
                        && (q[2] - 5. * (normalized_u.powi(3) - 3. * normalized_u * v * v)).abs()
                            < 1e-12
                );
            }
        }
    }
    #[test]
    #[cfg(feature = "codec")]
    fn text_parser_rejects_functions_invalid_powers_and_resource_abuse() {
        for bad in [
            "sin(t)", "t^0", "t^13", "t^1.5", "t^2^3", "1e999", "t**2", "2t", "(t", "t)", "t+",
            "u", "t+v", "t;0", "t×t",
        ] {
            assert!(
                tokens(&vec![json!(bad), json!(0), json!(0)], false).is_err(),
                "{bad}"
            );
        }
        let deep = format!("{}t{}", "(".repeat(30), ")".repeat(30));
        assert!(tokens(&vec![json!(deep), json!(0), json!(0)], false).is_err());
        assert!(
            tokens(
                &vec![json!("1+".repeat(40) + "1"), json!(0), json!(0)],
                false
            )
            .is_err()
        );
        assert!(tokens(&vec![json!(" ".repeat(1025)), json!(0), json!(0)], false).is_err());
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_dispatch_accepts_mixed_text_and_token_formulas() {
        let value = crate::dispatch(
            json!({"op":"surface_formula","domain":[0,1,0,1],"expressions":["u",["v"],"u*v"]}),
        )
        .unwrap();
        let surface: Surface = value_codec::from_value(value).unwrap();
        let q = surface.evaluate(0.3, 0.7).unwrap().point;
        assert!(
            (q[0] - 0.3).abs() < 1e-12 && (q[1] - 0.7).abs() < 1e-12 && (q[2] - 0.21).abs() < 1e-12
        );
    }
    #[test]
    #[cfg(feature = "codec")]
    fn polynomial_and_rational_formulas_match_independent_equations() {
        let polynomial = curve(
            [-2., 3.],
            &expressions(json!([
                ["t"],
                ["t", "t", "*", 2, "+"],
                ["t", 3, "*", "neg"]
            ])),
        )
        .unwrap();
        let circle = curve(
            [0., 1.],
            &expressions(json!([
                [1, "t", "t", "*", "-", 1, "t", "t", "*", "+", "/"],
                [2, "t", "*", 1, "t", "t", "*", "+", "/"],
                [0]
            ])),
        )
        .unwrap();
        for i in 0..=100 {
            let u = i as f64 / 100.;
            let t = -2. + 5. * u;
            let p = polynomial.evaluate(u).unwrap().point;
            assert!(
                (p[0] - t).abs() < 1e-12
                    && (p[1] - t * t - 2.).abs() < 1e-12
                    && (p[2] + 3. * t).abs() < 1e-12
            );
            let p = circle.evaluate(u).unwrap().point;
            assert!((p[0] - (1. - u * u) / (1. + u * u)).abs() < 1e-12);
            assert!((p[1] - 2. * u / (1. + u * u)).abs() < 1e-12);
            assert!((p[0] * p[0] + p[1] * p[1] - 1.).abs() < 1e-12);
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_formula_dispatch_retains_the_same_control_definition() {
        let expressions = expressions(json!([["t"], ["t", "t", "*"], [0]]));
        let direct = curve([-1., 2.], &expressions).unwrap();
        let value = crate::dispatch(
            json!({"op":"curve_formula","domain":[-1,2],"expressions":expressions}),
        )
        .unwrap();
        let decoded: Curve = value_codec::from_value(value).unwrap();
        assert_eq!(
            value_codec::to_value(direct).unwrap(),
            value_codec::to_value(decoded).unwrap()
        );
    }
    #[test]
    #[cfg(feature = "codec")]
    fn two_variable_formulas_retain_mixed_terms_and_independent_denominators() {
        let graph = surface(
            [-2., 3., -1., 4.],
            &expressions(json!([["u"], ["v"], ["u", "v", "*", "u", "u", "*", "+"]])),
        )
        .unwrap();
        let rational = surface(
            [0., 2., 0., 3.],
            &expressions(json!([
                ["u", 1, "u", "+", "/"],
                ["v", 1, "v", "+", "/"],
                ["u", "v", "*", 1, "u", "v", "*", "+", "/"]
            ])),
        )
        .unwrap();
        for i in 0..=20 {
            for j in 0..=20 {
                let x = i as f64 / 20.;
                let y = j as f64 / 20.;
                let u = -2. + 5. * x;
                let v = -1. + 5. * y;
                let p = graph.evaluate(x, y).unwrap().point;
                assert!(
                    (p[0] - u).abs() < 1e-12
                        && (p[1] - v).abs() < 1e-12
                        && (p[2] - u * v - u * u).abs() < 1e-12
                );
                let u = 2. * x;
                let v = 3. * y;
                let p = rational.evaluate(x, y).unwrap().point;
                assert!(
                    (p[0] - u / (1. + u)).abs() < 1e-12
                        && (p[1] - v / (1. + v)).abs() < 1e-12
                        && (p[2] - u * v / (1. + u * v)).abs() < 1e-12
                );
            }
        }
        assert!(
            surface(
                [-1., 1., 0., 1.],
                &expressions(json!([[1, "u", "/"], ["v"], [0]]))
            )
            .is_err()
        );
        assert!(surface([0., 1., 0., 1.], &expressions(json!([["t"], ["v"], [0]]))).is_err());
        assert!(curve([0., 1.], &expressions(json!([["u"], [0], [0]]))).is_err());
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_surface_formula_dispatch_keeps_the_control_net() {
        let expressions = expressions(json!([["u"], ["v"], ["u", "v", "*"]]));
        let direct = surface([-2., 3., 0., 4.], &expressions).unwrap();
        let value = crate::dispatch(
            json!({"op":"surface_formula","domain":[-2,3,0,4],"expressions":expressions}),
        )
        .unwrap();
        assert_eq!(value_codec::to_value(direct).unwrap(), value);
    }
    #[test]
    #[cfg(feature = "codec")]
    fn rejects_invalid_stack_degree_overflow_and_singular_intervals() {
        for bad in [
            json!([]),
            json!(["+"]),
            json!([1, 2]),
            json!(["sin"]),
            json!([1, 0, "/"]),
            json!([{}]),
        ] {
            let decoded = value_codec::from_value::<Vec<Vec<Token>>>(json!([bad, [0], [0]]));
            assert!(decoded.is_err() || curve([0., 1.], &decoded.unwrap()).is_err());
        }
        assert!(curve([-1., 1.], &expressions(json!([[1, "t", "/"], [0], [0]]))).is_err());
        let mut degree = vec![Token::T];
        for _ in 0..12 {
            degree.extend([Token::T, Token::Multiply]);
        }
        assert!(
            curve(
                [0., 1.],
                &[degree, vec![Token::Constant(0.)], vec![Token::Constant(0.)]]
            )
            .is_err()
        );
        assert!(
            curve(
                [0., 1.],
                &[
                    vec![Token::Constant(0.); 65],
                    vec![Token::Constant(0.)],
                    vec![Token::Constant(0.)]
                ]
            )
            .is_err()
        );
        assert!(
            curve(
                [0., 1.],
                &expressions(json!([[1e300, 1e300, "*"], [0], [0]]))
            )
            .is_err()
        );
        assert!(curve([0., 1.], &expressions(json!([[0], [0]]))).is_err());
    }
}
