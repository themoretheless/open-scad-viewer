//! Decode host expressions into native formula instructions.
use super::{Token, parse};
use crate::{Result, check};
use value_codec::{Deserialize, Serialize, Value};

impl Serialize for Token {
    fn to_value(&self) -> Value {
        match self {
            Self::Constant(value) => value.to_value(),
            Self::T => "t".to_value(),
            Self::U => "u".to_value(),
            Self::V => "v".to_value(),
            Self::Negate => "neg".to_value(),
            Self::Add => "+".to_value(),
            Self::Subtract => "-".to_value(),
            Self::Multiply => "*".to_value(),
            Self::Divide => "/".to_value(),
        }
    }
}

impl<'de> Deserialize<'de> for Token {
    fn from_value(value: Value) -> value_codec::Result<Self> {
        if let Some(number) = value.as_f64() {
            return Ok(Self::Constant(number));
        }
        Ok(match value.as_str() {
            Some("t") => Self::T,
            Some("u") => Self::U,
            Some("v") => Self::V,
            Some("neg") => Self::Negate,
            Some("+") => Self::Add,
            Some("-") => Self::Subtract,
            Some("*") => Self::Multiply,
            Some("/") => Self::Divide,
            Some(_) => return Err(value_codec::error("Unknown formula operator or variable")),
            None => {
                return Err(value_codec::error(
                    "Formula tokens must be numbers or operators",
                ));
            }
        })
    }
}

/// Host coordinates may contain infix strings or reverse Polish token lists.
pub fn tokens(values: &[Value], surface: bool) -> Result<Vec<Vec<Token>>> {
    check(
        values.len() == 3,
        "Spatial formula requires three coordinate expressions",
    )?;
    values
        .iter()
        .map(|value| {
            if let Some(source) = value.as_str() {
                parse(source, surface)
            } else {
                let list = value
                    .as_array()
                    .ok_or_else(|| crate::input("Choose a formula string or token list"))?;
                check(
                    !list.is_empty() && list.len() <= 64,
                    "Each formula requires 1..64 tokens",
                )?;
                list.iter()
                    .map(|token| {
                        Token::from_value(token.clone())
                            .map_err(|error| crate::input(error.to_string()))
                    })
                    .collect()
            }
        })
        .collect()
}
