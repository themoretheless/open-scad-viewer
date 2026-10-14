//! Bounded endpoint ranges without cumulative floating-point drift.
use crate::units::{self, Numeric, SCALAR};
use crate::{Error, Result};

pub fn resolve_interval(
    start: Numeric,
    end: Numeric,
    inclusive: bool,
    count: Option<Numeric>,
    step: Option<Numeric>,
    path: &str,
) -> Result<Vec<Numeric>> {
    units::equal(start, end, path)?;
    if count.is_some() && step.is_some() {
        return Err(Error::new(
            "invalid_range",
            path,
            "Use either by or count, never both.",
        ));
    }
    let (a, b) = (start.value, end.value);
    let (length, step) = if let Some(count) = count {
        let count = units::scalar(count, path)?;
        if count.fract() != 0.0 || !(0.0..=256.0).contains(&count) {
            return Err(Error::new(
                "invalid_count",
                path,
                "Range count must be an integer from 0 to 256.",
            ));
        }
        if !inclusive && a == b && count > 0.0 {
            return Err(Error::new(
                "invalid_range",
                path,
                "An empty exclusive interval cannot contain samples.",
            ));
        }
        let step = if count <= 1.0 {
            units::binary("subtract", start, start, path)?
        } else {
            units::binary(
                "divide",
                units::binary("subtract", end, start, path)?,
                (if inclusive { count - 1.0 } else { count }).into(),
                path,
            )?
        };
        (count as usize, step)
    } else {
        if step.is_none()
            && (start.dimension != SCALAR
                || end.dimension != SCALAR
                || a.fract() != 0.0
                || b.fract() != 0.0)
        {
            return Err(Error::new(
                "invalid_range",
                path,
                "Only integer dimensionless ranges have an implicit step; specify by or count.",
            ));
        }
        let step = step.unwrap_or_else(|| 1.0.into());
        units::equal(start, step, path)?;
        let s = step.value;
        if s == 0.0 {
            return Err(Error::new(
                "invalid_range",
                path,
                "Range step cannot be zero.",
            ));
        }
        let distance = (b - a) / s;
        // Match Math.round (ties toward positive infinity), including negative ranges.
        let rounded = (distance + 0.5).floor();
        let q = if (distance - rounded).abs() <= 8.0 * f64::EPSILON * distance.abs().max(1.0) {
            rounded
        } else {
            distance
        };
        let count = if q < 0.0 {
            0.0
        } else if inclusive {
            q.floor() + 1.0
        } else {
            q.ceil()
        };
        if !count.is_finite() || count > 256.0 {
            return Err(Error::new(
                "invalid_count",
                path,
                format!(
                    "Range creates {} values; maximum 256. Increase by or reduce count.",
                    if count.is_infinite() {
                        "Infinity".to_owned()
                    } else {
                        count.to_string()
                    }
                ),
            ));
        }
        (count as usize, step)
    };
    let mut output = Vec::with_capacity(length);
    for i in 0..length {
        output.push(
            if count.is_some() && inclusive && length > 1 && i == length - 1 {
                end
            } else {
                units::binary(
                    "add",
                    start,
                    units::binary("multiply", (i as f64).into(), step, path)?,
                    path,
                )?
            },
        );
    }
    Ok(output)
}
