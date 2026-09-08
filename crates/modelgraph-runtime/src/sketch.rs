//! Bounded damped least-squares solver. Constraint references are resolved once,
//! and residual buffers are reused while computing numerical Jacobians.
use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
fn maximum(v: &[f64]) -> f64 {
    v.iter().map(|v| v.abs()).fold(0., f64::max)
}
fn rank(mut rows: Vec<Vec<f64>>) -> usize {
    if rows.is_empty() {
        return 0;
    }
    let mut pivot = 0;
    for column in 0..rows[0].len() {
        if pivot == rows.len() {
            break;
        }
        let mut best = pivot;
        for i in pivot + 1..rows.len() {
            if rows[i][column].abs() > rows[best][column].abs() {
                best = i;
            }
        }
        if rows[best][column].abs() < 1e-7 {
            continue;
        }
        rows.swap(pivot, best);
        let scale = rows[pivot][column];
        for j in column..rows[pivot].len() {
            rows[pivot][j] /= scale;
        }
        for i in pivot + 1..rows.len() {
            let factor = rows[i][column];
            for j in column..rows[i].len() {
                rows[i][j] -= factor * rows[pivot][j];
            }
        }
        pivot += 1;
    }
    pivot
}
fn linear_solve(mut a: Vec<Vec<f64>>, rhs: Vec<f64>) -> Option<Vec<f64>> {
    let n = rhs.len();
    for (i, r) in rhs.into_iter().enumerate() {
        a[i].push(r);
    }
    for c in 0..n {
        let mut best = c;
        for i in c + 1..n {
            if a[i][c].abs() > a[best][c].abs() {
                best = i;
            }
        }
        if a[best][c].abs() < 1e-16 {
            return None;
        }
        a.swap(c, best);
        let (leading, trailing) = a.split_at_mut(c + 1);
        let pivot = &leading[c];
        for row in trailing {
            let factor = row[c] / pivot[c];
            for (cell, pivot_cell) in row[c..=n].iter_mut().zip(&pivot[c..=n]) {
                *cell -= factor * pivot_cell;
            }
        }
    }
    let mut result = vec![0.; n];
    for i in (0..n).rev() {
        let mut sum = a[i][n];
        for j in i + 1..n {
            sum -= a[i][j] * result[j];
        }
        result[i] = sum / a[i][i];
    }
    Some(result)
}
#[derive(Clone, Copy)]
enum Kind {
    Fix,
    Horizontal,
    Vertical,
    Coincident,
    Distance,
    Parallel,
    Perpendicular,
    EqualLength,
}
struct Constraint<'a> {
    id: &'a str,
    kind: Kind,
    refs: [usize; 4],
    target: [f64; 2],
    offset: usize,
    count: usize,
}
fn residual(x: &[f64], cs: &[Constraint<'_>], out: &mut [f64]) {
    for c in cs {
        let [a, b, d, e] = c.refs;
        let i = c.offset;
        if matches!(c.kind, Kind::Fix) {
            out[i] = x[a] - c.target[0];
            out[i + 1] = x[a + 1] - c.target[1];
            continue;
        }
        let u = [x[b] - x[a], x[b + 1] - x[a + 1]];
        let lu = u[0].hypot(u[1]);
        match c.kind {
            Kind::Horizontal => out[i] = u[1],
            Kind::Vertical => out[i] = u[0],
            Kind::Coincident => {
                out[i] = u[0];
                out[i + 1] = u[1];
            }
            Kind::Distance => out[i] = lu - c.target[0],
            _ => {
                let v = [x[e] - x[d], x[e + 1] - x[d + 1]];
                let lv = v[0].hypot(v[1]);
                out[i] = match c.kind {
                    Kind::EqualLength => lu - lv,
                    Kind::Parallel => (u[0] * v[1] - u[1] * v[0]) / lu.max(lv).max(1e-12),
                    _ => (u[0] * v[0] + u[1] * v[1]) / lu.max(lv).max(1e-12),
                };
            }
        }
    }
}
fn jacobian(x: &[f64], cs: &[Constraint<'_>], count: usize) -> Vec<Vec<f64>> {
    let mut rows = vec![vec![0.; x.len()]; count];
    let mut plus = x.to_vec();
    let mut minus = x.to_vec();
    let mut hi = vec![0.; count];
    let mut lo = vec![0.; count];
    for j in 0..x.len() {
        let h = 1e-6f64.max(x[j].abs() * 1e-8);
        plus[j] += h;
        minus[j] -= h;
        residual(&plus, cs, &mut hi);
        residual(&minus, cs, &mut lo);
        for i in 0..count {
            rows[i][j] = (hi[i] - lo[i]) / (2. * h);
        }
        plus[j] = x[j];
        minus[j] = x[j];
    }
    rows
}
pub fn solve(points: &[Value], constraints: &[Value], path: &str) -> Result<Value> {
    let err = |m: String| Error::new("invalid_sketch", path, m);
    let tolerance = 1e-6;
    let mut index = HashMap::with_capacity(points.len());
    let mut x = Vec::with_capacity(points.len() * 2);
    for (i, p) in points.iter().enumerate() {
        if index.insert(p["id"].as_str().unwrap(), i * 2).is_some() {
            return Err(err("Sketch point and constraint IDs must be unique.".into()));
        }
        x.push(p["position"][0].as_f64().unwrap());
        x.push(p["position"][1].as_f64().unwrap());
    }
    if x.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
        return Err(err("Invalid sketch coordinates.".into()));
    }
    let mut ids = HashSet::new();
    let mut count = 0;
    let mut cs = Vec::with_capacity(constraints.len());
    for c in constraints {
        let id = c["id"].as_str().unwrap();
        if !ids.insert(id) {
            return Err(err("Sketch point and constraint IDs must be unique.".into()));
        }
        let kind = match c["kind"].as_str().unwrap() {
            "fix" => Kind::Fix,
            "horizontal" => Kind::Horizontal,
            "vertical" => Kind::Vertical,
            "coincident" => Kind::Coincident,
            "distance" => Kind::Distance,
            "parallel" => Kind::Parallel,
            "perpendicular" => Kind::Perpendicular,
            _ => Kind::EqualLength,
        };
        let keys: &[&str] = if matches!(kind, Kind::Fix) {
            &["point"]
        } else if c.get("c").is_some() {
            &["a", "b", "c", "d"]
        } else {
            &["a", "b"]
        };
        let mut refs = [0; 4];
        for (i, k) in keys.iter().enumerate() {
            let name = c[k].as_str().unwrap();
            refs[i] = *index
                .get(name)
                .ok_or_else(|| err(format!("Unknown sketch point {name}.")))?;
        }
        let target = if matches!(kind, Kind::Fix) {
            let a = [c["at"][0].as_f64().unwrap(), c["at"][1].as_f64().unwrap()];
            if a.iter().any(|v| !v.is_finite() || v.abs() > 1e6) {
                return Err(err("Invalid fixed target.".into()));
            }
            a
        } else if matches!(kind, Kind::Distance) {
            let v = c["value"].as_f64().unwrap();
            if v <= 0. || !v.is_finite() {
                return Err(err(
                    "Distance must be positive; use coincident for zero distance.".into(),
                ));
            }
            [v, 0.]
        } else {
            [0., 0.]
        };
        let size = if matches!(kind, Kind::Fix | Kind::Coincident) {
            2
        } else {
            1
        };
        cs.push(Constraint {
            id,
            kind,
            refs,
            target,
            offset: count,
            count: size,
        });
        count += size;
    }
    let mut damping = 1e-3;
    let mut iterations = 0;
    let mut r = vec![0.; count];
    let mut candidate_residual = vec![0.; count];
    while iterations < 64 {
        residual(&x, &cs, &mut r);
        if maximum(&r) <= tolerance {
            break;
        }
        let j = jacobian(&x, &cs, count);
        let n = x.len();
        let mut lhs = vec![vec![0.; n]; n];
        let mut rhs = vec![0.; n];
        for a in 0..n {
            for k in 0..j.len() {
                rhs[a] -= j[k][a] * r[k];
                for b in 0..n {
                    lhs[a][b] += j[k][a] * j[k][b];
                }
            }
            lhs[a][a] += damping;
        }
        if let Some(step) = linear_solve(lhs, rhs) {
            let candidate: Vec<f64> = x.iter().zip(step.iter()).map(|(v, s)| v + s).collect();
            residual(&candidate, &cs, &mut candidate_residual);
            let error = |v: &[f64]| v.iter().map(|r| r * r).sum::<f64>();
            if candidate.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
                && error(&candidate_residual) < error(&r)
            {
                x = candidate;
                damping = (damping / 3.).max(1e-12);
            } else {
                damping = (damping * 10.).min(1e12);
            }
        } else {
            damping *= 10.;
        }
        iterations += 1;
    }
    residual(&x, &cs, &mut r);
    let j = jacobian(&x, &cs, count);
    let jacobian_rank = rank(j.clone());
    let degenerate: Vec<&str> = cs
        .iter()
        .filter(|c| matches!(c.kind, Kind::Parallel | Kind::Perpendicular))
        .filter(|c| {
            let [a, b, d, e] = c.refs;
            (x[a] - x[b]).hypot(x[a + 1] - x[b + 1]) <= tolerance
                || (x[d] - x[e]).hypot(x[d + 1] - x[e + 1]) <= tolerance
        })
        .map(|c| c.id)
        .collect();
    let converged = maximum(&r) <= tolerance && degenerate.is_empty();
    let linear = cs.iter().all(|c| {
        matches!(
            c.kind,
            Kind::Fix | Kind::Horizontal | Kind::Vertical | Kind::Coincident
        )
    });
    let status = if converged {
        if x.len() > jacobian_rank {
            "underconstrained"
        } else {
            "solved"
        }
    } else if linear
        && rank(
            j.into_iter()
                .zip(r.iter())
                .map(|(mut row, r)| {
                    row.push(*r);
                    row
                })
                .collect(),
        ) > jacobian_rank
    {
        "inconsistent"
    } else {
        "not_converged"
    };
    let solved_points: Vec<Value> = points
        .iter()
        .enumerate()
        .map(|(i, p)| json!({"id":p["id"],"position":[x[i*2],x[i*2+1]]}))
        .collect();
    let reports:Vec<Value>=cs.iter().map(|c|{let value=maximum(&r[c.offset..c.offset+c.count]);json!({"id":c.id,"residual_mm":value,"satisfied":value<=tolerance&&!degenerate.contains(&c.id)})}).collect();
    Ok(
        json!({"status":status,"iterations":iterations,"tolerance_mm":tolerance,"degrees_of_freedom":x.len()-jacobian_rank,"redundant_equations":count.saturating_sub(jacobian_rank),"maximum_residual_mm":maximum(&r),"degenerate_constraints":degenerate,"points":solved_points,"constraints":reports}),
    )
}
