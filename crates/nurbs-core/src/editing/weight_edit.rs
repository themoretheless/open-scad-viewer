//! Immutable local rational-weight edits, including periodic storage aliases.
use crate::{check, curve::Curve, surface::Surface, Result};
use std::collections::BTreeMap;

/// Set selected weights without changing controls, knots or degree.
/// Periodic duplicated controls share one logical weight. Duplicate logical
/// edits are rejected, even if they specify the same value.
pub fn curve(source: &Curve, edits: &[(usize, f64)]) -> Result<Curve> {
    source.validate()?;
    check(
        !edits.is_empty() && edits.len() <= 256,
        "Curve weight edit requires 1..256 entries",
    )?;
    let n = source.weights.len();
    let period = if source.periodic {
        n - source.degree
    } else {
        n
    };
    let mut values = BTreeMap::new();
    for &(index, value) in edits {
        check(index < n, "Curve weight index out of range")?;
        check(
            values.insert(index % period, value).is_none(),
            "Duplicate logical curve weight edit",
        )?;
    }
    let mut result = source.clone();
    for (i, weight) in result.weights.iter_mut().enumerate() {
        if let Some(value) = values.get(&(i % period)) {
            *weight = *value;
        }
    }
    result.validate()?;
    Ok(result)
}

/// Set selected tensor-product weights. Periodic U/V aliases, including
/// corner copies, are updated together. The whole net is validated atomically.
pub fn surface(source: &Surface, edits: &[(usize, usize, f64)]) -> Result<Surface> {
    source.validate()?;
    check(
        !edits.is_empty() && edits.len() <= 1024,
        "Surface weight edit requires 1..1024 entries",
    )?;
    let nu = source.weights.len();
    let nv = source.weights[0].len();
    let pu = if source.periodic_u {
        nu - source.degree_u
    } else {
        nu
    };
    let pv = if source.periodic_v {
        nv - source.degree_v
    } else {
        nv
    };
    let mut values = BTreeMap::new();
    for &(u, v, value) in edits {
        check(u < nu && v < nv, "Surface weight index out of range")?;
        check(
            values.insert((u % pu, v % pv), value).is_none(),
            "Duplicate logical surface weight edit",
        )?;
    }
    let mut result = source.clone();
    for (u, row) in result.weights.iter_mut().enumerate() {
        for (v, weight) in row.iter_mut().enumerate() {
            if let Some(value) = values.get(&(u % pu, v % pv)) {
                *weight = *value;
            }
        }
    }
    result.validate()?;
    Ok(result)
}
