//! Named ModelGraph sketch adapter. Numeric solving belongs to sketch-core.
use crate::{Error, Result};
use sketch_core::{Constraint, Sketch};
use std::collections::{HashMap, HashSet};
use value_codec::{Value, json};

pub fn solve(points: &[Value], constraints: &[Value], path: &str) -> Result<Value> {
    let err = |m: String| Error::new("invalid_sketch", path, m);
    let tolerance = 1e-6;
    let mut index = HashMap::with_capacity(points.len());
    let mut positions = Vec::with_capacity(points.len());
    for (i, p) in points.iter().enumerate() {
        let id = p["id"]
            .as_str()
            .ok_or_else(|| err("Invalid sketch point ID.".into()))?;
        if index.insert(id, i).is_some() {
            return Err(err("Sketch point and constraint IDs must be unique.".into()));
        }
        let position = [p["position"][0].as_f64(), p["position"][1].as_f64()];
        let [Some(x), Some(y)] = position else {
            return Err(err("Invalid sketch coordinates.".into()));
        };
        if [x, y].iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
            return Err(err("Invalid sketch coordinates.".into()));
        }
        positions.push([x, y]);
    }
    let mut ids = HashSet::new();
    let mut native = Vec::with_capacity(constraints.len());
    for c in constraints {
        let id = c["id"]
            .as_str()
            .ok_or_else(|| err("Invalid sketch constraint ID.".into()))?;
        if !ids.insert(id) {
            return Err(err("Sketch point and constraint IDs must be unique.".into()));
        }
        let reference = |key: &str| -> Result<usize> {
            let name = c[key]
                .as_str()
                .ok_or_else(|| err("Invalid sketch point reference.".into()))?;
            index
                .get(name)
                .copied()
                .ok_or_else(|| err(format!("Unknown sketch point {name}.")))
        };
        let kind = c["kind"].as_str().unwrap_or("");
        native.push(match kind {
            "fix" => {
                let point = reference("point")?;
                let [Some(x), Some(y)] = [c["at"][0].as_f64(), c["at"][1].as_f64()] else {
                    return Err(err("Invalid fixed target.".into()));
                };
                if [x, y].iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
                    return Err(err("Invalid fixed target.".into()));
                }
                Constraint::Fix { point, at: [x, y] }
            }
            "horizontal" => Constraint::Horizontal {
                a: reference("a")?,
                b: reference("b")?,
            },
            "vertical" => Constraint::Vertical {
                a: reference("a")?,
                b: reference("b")?,
            },
            "coincident" => Constraint::Coincident {
                a: reference("a")?,
                b: reference("b")?,
            },
            "distance" => {
                let a = reference("a")?;
                let b = reference("b")?;
                let value = c["value"].as_f64().unwrap_or(f64::NAN);
                if !value.is_finite() || value <= 0. {
                    return Err(err(
                        "Distance must be positive; use coincident for zero distance.".into(),
                    ));
                }
                Constraint::Distance { a, b, value }
            }
            "parallel" => Constraint::Parallel {
                a: reference("a")?,
                b: reference("b")?,
                c: reference("c")?,
                d: reference("d")?,
            },
            "perpendicular" => Constraint::Perpendicular {
                a: reference("a")?,
                b: reference("b")?,
                c: reference("c")?,
                d: reference("d")?,
            },
            "equal_length" => Constraint::EqualLength {
                a: reference("a")?,
                b: reference("b")?,
                c: reference("c")?,
                d: reference("d")?,
            },
            _ => return Err(err("Invalid sketch constraint kind.".into())),
        });
    }
    let sketch = Sketch {
        points: positions,
        circles: vec![],
        constraints: native,
    };
    let (solution, diagnostics) = sketch_core::solve_with_diagnostics_options(
        &sketch,
        sketch_core::SolverOptions {
            tolerance,
            max_iterations: 64,
        },
    )
    .map_err(|e| err(e.message))?;
    let degenerate: HashSet<_> = diagnostics.degenerate_constraints.iter().copied().collect();
    let status = if diagnostics.inconsistent {
        "inconsistent"
    } else if solution.status == "degenerate" {
        "not_converged"
    } else {
        solution.status
    };
    let solved_points: Vec<_> = points
        .iter()
        .enumerate()
        .map(|(i, p)| json!({"id":p["id"], "position":solution.sketch.points[i]}))
        .collect();
    let reports: Vec<_> = constraints.iter().enumerate().map(|(i,c)| {
        let residual = diagnostics.constraint_residuals[i];
        json!({"id":c["id"], "residual_mm":residual, "satisfied":residual <= tolerance && !degenerate.contains(&i)})
    }).collect();
    let degenerate_ids: Vec<_> = diagnostics
        .degenerate_constraints
        .iter()
        .map(|&i| constraints[i]["id"].clone())
        .collect();
    Ok(
        json!({"status":status,"iterations":solution.iterations,"tolerance_mm":tolerance,"degrees_of_freedom":solution.degrees_of_freedom,"redundant_equations":diagnostics.redundant_equations,"maximum_residual_mm":solution.max_residual,"degenerate_constraints":degenerate_ids,"points":solved_points,"constraints":reports}),
    )
}
