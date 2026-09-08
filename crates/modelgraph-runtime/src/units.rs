//! Canonical dimensional arithmetic: millimeters and degrees.
use crate::{Error, Result};

pub type Dimension = [i8; 2];
pub const LENGTH: Dimension = [1, 0];
pub const ANGLE: Dimension = [0, 1];
pub const SCALAR: Dimension = [0, 0];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Numeric {
    pub value: f64,
    pub dimension: Dimension,
}
impl From<f64> for Numeric {
    fn from(value: f64) -> Self {
        Self {
            value,
            dimension: SCALAR,
        }
    }
}
pub fn dimension_label(dimension: Dimension) -> String {
    format!("length^{} angle^{}", dimension[0], dimension[1])
}
fn pack(value: f64, dimension: [f64; 2], path: &str) -> Result<Numeric> {
    if !value.is_finite() || value.abs() > 1_000_000.0 {
        return Err(Error::new(
            "invalid_number",
            path,
            "Expression must produce a finite number within +/-1000000 in canonical units.",
        ));
    }
    if dimension
        .iter()
        .any(|v| !v.is_finite() || v.fract() != 0.0 || v.abs() > 8.0)
    {
        return Err(Error::new(
            "dimension_limit",
            path,
            "Dimension exponents must be integers within +/-8.",
        ));
    }
    Ok(Numeric {
        value,
        dimension: [dimension[0] as i8, dimension[1] as i8],
    })
}
pub fn equal(a: Numeric, b: Numeric, path: &str) -> Result<()> {
    if a.dimension != b.dimension {
        return Err(Error::new(
            "unit_mismatch",
            path,
            format!(
                "Incompatible dimensions: {} and {}.",
                dimension_label(a.dimension),
                dimension_label(b.dimension)
            ),
        ));
    }
    Ok(())
}
pub fn scalar(value: Numeric, path: &str) -> Result<f64> {
    if value.dimension != SCALAR {
        return Err(Error::new(
            "unit_mismatch",
            path,
            "Expected a dimensionless number.",
        ));
    }
    Ok(value.value)
}
pub fn quantity(value: f64, unit: &str, path: &str) -> Result<Numeric> {
    let (scale, dimension) = match unit {
        "mm" => (1.0, LENGTH),
        "cm" => (10.0, LENGTH),
        "m" => (1000.0, LENGTH),
        "in" => (25.4, LENGTH),
        "deg" => (1.0, ANGLE),
        "rad" => (180.0 / std::f64::consts::PI, ANGLE),
        _ => return Err(Error::new("type_error", path, "Unknown unit.")),
    };
    pack(value * scale, dimension.map(f64::from), path)
}
pub fn binary(op: &str, left: Numeric, right: Numeric, path: &str) -> Result<Numeric> {
    let (a, b, ad, bd) = (left.value, right.value, left.dimension, right.dimension);
    match op {
        "multiply" => {
            return pack(
                a * b,
                [
                    f64::from(ad[0]) + f64::from(bd[0]),
                    f64::from(ad[1]) + f64::from(bd[1]),
                ],
                path,
            )
        }
        "divide" => {
            return pack(
                a / b,
                [
                    f64::from(ad[0]) - f64::from(bd[0]),
                    f64::from(ad[1]) - f64::from(bd[1]),
                ],
                path,
            )
        }
        "pow" => {
            scalar(right, path)?;
            return pack(
                a.powf(b),
                [f64::from(ad[0]) * b, f64::from(ad[1]) * b],
                path,
            );
        }
        "and" => {
            return Ok(
                ((scalar(left, path)? != 0.0 && scalar(right, path)? != 0.0) as u8 as f64).into(),
            )
        }
        "or" => {
            return Ok(
                ((scalar(left, path)? != 0.0 || scalar(right, path)? != 0.0) as u8 as f64).into(),
            )
        }
        _ => (),
    }
    equal(left, right, path)?;
    let value = match op {
        "add" => a + b,
        "subtract" => a - b,
        "mod" => a % b,
        // min/max must retain JavaScript's signed-zero ordering.
        "min" => {
            if a == 0.0 && b == 0.0 {
                if a.is_sign_negative() || b.is_sign_negative() {
                    -0.0
                } else {
                    0.0
                }
            } else {
                a.min(b)
            }
        }
        "max" => {
            if a == 0.0 && b == 0.0 {
                if a.is_sign_positive() || b.is_sign_positive() {
                    0.0
                } else {
                    -0.0
                }
            } else {
                a.max(b)
            }
        }
        "lt" => return Ok(((a < b) as u8 as f64).into()),
        "le" => return Ok(((a <= b) as u8 as f64).into()),
        "eq" => return Ok(((a == b) as u8 as f64).into()),
        _ => return Err(Error::new("type_error", path, "Unknown numeric operation.")),
    };
    pack(value, ad.map(f64::from), path)
}
pub fn unary(op: &str, value: Numeric, path: &str, strict: bool) -> Result<Numeric> {
    let a = value.value;
    let dimension = value.dimension;
    let (number, result_dimension) = match op {
        "not" => return Ok(((scalar(value, path)? == 0.0) as u8 as f64).into()),
        "negate" => (-a, dimension.map(f64::from)),
        "abs" => (a.abs(), dimension.map(f64::from)),
        "floor" => (a.floor(), dimension.map(f64::from)),
        "ceil" => (a.ceil(), dimension.map(f64::from)),
        "sqrt" => (
            a.sqrt(),
            [f64::from(dimension[0]) / 2.0, f64::from(dimension[1]) / 2.0],
        ),
        "sin" | "cos" => {
            if dimension != ANGLE && (strict || dimension != SCALAR) {
                return Err(Error::new(
                    "unit_mismatch",
                    path,
                    "Trigonometry expects an angle.",
                ));
            }
            let radians = a * std::f64::consts::PI / 180.0;
            (
                if op == "sin" {
                    radians.sin()
                } else {
                    radians.cos()
                },
                [0.0, 0.0],
            )
        }
        _ => return Err(Error::new("type_error", path, "Unknown numeric operation.")),
    };
    pack(number, result_dimension, path)
}
pub fn field(value: Numeric, expected: Dimension, path: &str, strict: bool) -> Result<f64> {
    if value.dimension == expected || (value.dimension == SCALAR && (!strict || value.value == 0.0))
    {
        return Ok(value.value);
    }
    Err(Error::new(
        "unit_mismatch",
        path,
        format!(
            "Expected {}, received {}.",
            dimension_label(expected),
            dimension_label(value.dimension)
        ),
    ))
}
pub fn check_type(value: Numeric, kind: &str, path: &str) -> Result<Numeric> {
    let expected = match kind {
        "length" => LENGTH,
        "angle" => ANGLE,
        _ => SCALAR,
    };
    if value.dimension != expected {
        return Err(Error::new(
            "type_error",
            path,
            format!("Expected {kind}; incompatible units"),
        ));
    }
    let n = value.value;
    if kind == "int" && (n.fract() != 0.0 || !(-2147483648.0..=2147483647.0).contains(&n)) {
        return Err(Error::new(
            "type_error",
            path,
            "Expected int (signed 32-bit integer)",
        ));
    }
    if kind == "f32" {
        let rounded = n as f32;
        if !rounded.is_finite() {
            return Err(Error::new("type_error", path, "Value is outside f32 range"));
        }
        return Ok((rounded as f64).into());
    }
    Ok(value)
}
