//! Named RushGraph sketch adapter. Numeric solving belongs to sketch-core.
use crate::{Error, Result};
use sketch_core::{Constraint, Sketch};
use std::collections::{HashMap, HashSet};
use value_codec::{Value, json};

pub fn solve(
    points: &[Value],
    circles: &[Value],
    constraints: &[Value],
    path: &str,
) -> Result<Value> {
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
    let mut circle_index = HashMap::with_capacity(circles.len());
    let mut native_circles = Vec::with_capacity(circles.len());
    for (i, c) in circles.iter().enumerate() {
        let id = c["id"]
            .as_str()
            .ok_or_else(|| err("Invalid sketch circle ID.".into()))?;
        if index.contains_key(id) || circle_index.insert(id, i).is_some() {
            return Err(err("Sketch point and constraint IDs must be unique.".into()));
        }
        let center = c["center"]
            .as_str()
            .and_then(|name| index.get(name).copied())
            .ok_or_else(|| err("Sketch circle center must name a sketch point.".into()))?;
        let radius = c["radius"].as_f64().unwrap_or(f64::NAN);
        if !radius.is_finite() || radius <= 0. || radius > 1e6 {
            return Err(err("Sketch circle radius must be positive.".into()));
        }
        native_circles.push(sketch_core::Circle { center, radius });
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
        let circle = |key: &str| -> Result<usize> {
            let name = c[key]
                .as_str()
                .ok_or_else(|| err("Invalid sketch circle reference.".into()))?;
            circle_index
                .get(name)
                .copied()
                .ok_or_else(|| err(format!("Unknown sketch circle {name}.")))
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
            "radius" => {
                let circle = circle("circle")?;
                let value = c["value"].as_f64().unwrap_or(f64::NAN);
                if !value.is_finite() || value <= 0. {
                    return Err(err("Radius must be positive.".into()));
                }
                Constraint::Radius { circle, value }
            }
            "concentric" => Constraint::Concentric {
                a: circle("a")?,
                b: circle("b")?,
            },
            "equal_radius" => Constraint::EqualRadius {
                a: circle("a")?,
                b: circle("b")?,
            },
            "point_on_circle" => Constraint::PointOnCircle {
                point: reference("point")?,
                circle: circle("circle")?,
            },
            "tangent_line_circle" => Constraint::TangentLineCircle {
                a: reference("a")?,
                b: reference("b")?,
                circle: circle("circle")?,
            },
            "tangent_circles" => Constraint::TangentCircles {
                a: circle("a")?,
                b: circle("b")?,
                internal: c["internal"].as_bool().unwrap_or(false),
            },
            _ => return Err(err("Invalid sketch constraint kind.".into())),
        });
    }
    let sketch = Sketch {
        points: positions,
        circles: native_circles,
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
    let solved_circles: Vec<_> = circles
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let circle = &solution.sketch.circles[i];
            json!({"id":c["id"], "center":solution.sketch.points[circle.center], "radius":circle.radius})
        })
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
    let mut report = json!({"status":status,"iterations":solution.iterations,"tolerance_mm":tolerance,"degrees_of_freedom":solution.degrees_of_freedom,"redundant_equations":diagnostics.redundant_equations,"maximum_residual_mm":solution.max_residual,"degenerate_constraints":degenerate_ids,"points":solved_points,"constraints":reports});
    if !solved_circles.is_empty() {
        report["circles"] = json!(solved_circles);
    }
    Ok(report)
}

type Hole = ([f64; 2], f64);

/// Closed outline from a solved sketch: straight boundary edges, optional arcs
/// along solved circles, and fully interior circular holes.
pub fn profile(
    item: &Value,
    boundary: &[Value],
    report: &Value,
    path: &str,
) -> Result<(Vec<[f64; 2]>, Vec<Hole>)> {
    let fail = |m: &str| Error::new("invalid_boundary", path, m);
    let pt = |v: &Value| [v[0].as_f64().unwrap_or(0.), v[1].as_f64().unwrap_or(0.)];
    let point = |id: &Value| {
        report["points"]
            .as_array()
            .and_then(|ps| ps.iter().find(|p| p["id"] == *id))
            .map(|p| pt(&p["position"]))
    };
    let circle = |id: &Value| {
        report["circles"]
            .as_array()
            .and_then(|cs| cs.iter().find(|c| c["id"] == *id))
            .map(|c| (pt(&c["center"]), c["radius"].as_f64().unwrap_or(0.)))
    };
    let mut arcs = HashMap::new();
    for arc in item["arcs"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
        let Some(at) = boundary.iter().position(|id| *id == arc["from"]) else {
            return Err(fail("Arc start must be a boundary point."));
        };
        let Some(c) = circle(&arc["circle"]) else {
            return Err(fail("Arc circle must name a sketch circle."));
        };
        if arcs
            .insert(at, (c, arc["ccw"].as_bool().unwrap_or(true)))
            .is_some()
        {
            return Err(fail("Each boundary edge may carry at most one arc."));
        }
    }
    let mut polygon = Vec::new();
    for (i, id) in boundary.iter().enumerate() {
        let start = point(id).ok_or_else(|| fail("Unknown boundary point."))?;
        polygon.push(start);
        let Some(&((center, radius), ccw)) = arcs.get(&i) else {
            continue;
        };
        let end = point(&boundary[(i + 1) % boundary.len()])
            .ok_or_else(|| fail("Unknown boundary point."))?;
        let on = |p: [f64; 2]| {
            ((p[0] - center[0]).hypot(p[1] - center[1]) - radius).abs() <= 1e-5 * radius.max(1.)
        };
        if !on(start) || !on(end) {
            return Err(fail(
                "Arc endpoints must lie on the arc circle; add point_on_circle constraints.",
            ));
        }
        let a0 = (start[1] - center[1]).atan2(start[0] - center[0]);
        let a1 = (end[1] - center[1]).atan2(end[0] - center[0]);
        let tau = std::f64::consts::TAU;
        let mut sweep = if ccw { a1 - a0 } else { a0 - a1 }.rem_euclid(tau);
        if sweep <= 1e-12 {
            sweep = tau;
        }
        let segments = ((sweep / tau) * 96.).ceil().max(2.) as usize;
        let sign = if ccw { 1. } else { -1. };
        for k in 1..segments {
            let a = a0 + sign * sweep * k as f64 / segments as f64;
            polygon.push([center[0] + radius * a.cos(), center[1] + radius * a.sin()]);
        }
    }
    let mut holes: Vec<Hole> = Vec::new();
    for id in item["holes"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
        let (c, r) = circle(id).ok_or_else(|| fail("Hole must name a sketch circle."))?;
        let n = polygon.len();
        let mut inside = false;
        let mut clearance = f64::INFINITY;
        for i in 0..n {
            let (p, q) = (polygon[i], polygon[(i + 1) % n]);
            if (p[1] > c[1]) != (q[1] > c[1])
                && c[0] < p[0] + (c[1] - p[1]) * (q[0] - p[0]) / (q[1] - p[1])
            {
                inside = !inside;
            }
            let d = [q[0] - p[0], q[1] - p[1]];
            let t = (((c[0] - p[0]) * d[0] + (c[1] - p[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]))
                .clamp(0., 1.);
            clearance = clearance.min((p[0] + t * d[0] - c[0]).hypot(p[1] + t * d[1] - c[1]));
        }
        if !inside || clearance <= r {
            return Err(fail("Hole circles must lie strictly inside the boundary."));
        }
        if holes
            .iter()
            .any(|(o, s)| (o[0] - c[0]).hypot(o[1] - c[1]) <= r + s)
        {
            return Err(fail("Hole circles must not overlap."));
        }
        holes.push((c, r));
    }
    Ok((polygon, holes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d_shape(hole_radius: Option<f64>) -> Value {
        let mut points = vec![
            json!({"id":"a","position":[0.1,0]}),
            json!({"id":"b","position":[19,0.2]}),
            json!({"id":"c","position":[20,11]}),
            json!({"id":"d","position":[0,10]}),
            json!({"id":"m","position":[20.5,4]}),
        ];
        let mut circles = vec![json!({"id":"end","center":"m","radius":3})];
        if hole_radius.is_some() {
            points.push(json!({"id":"hc","position":[8,5]}));
        }
        if let Some(r) = hole_radius {
            circles.push(json!({"id":"hole","center":"hc","radius":r}));
        }
        let mut constraints = vec![
            json!({"id":"fa","kind":"fix","point":"a","at":[0,0]}),
            json!({"id":"ab","kind":"horizontal","a":"a","b":"b"}),
            json!({"id":"w","kind":"distance","a":"a","b":"b","value":20}),
            json!({"id":"bc","kind":"vertical","a":"b","b":"c"}),
            json!({"id":"h","kind":"distance","a":"b","b":"c","value":10}),
            json!({"id":"cd","kind":"horizontal","a":"c","b":"d"}),
            json!({"id":"da","kind":"vertical","a":"d","b":"a"}),
            json!({"id":"bm","kind":"vertical","a":"b","b":"m"}),
            json!({"id":"ob","kind":"point_on_circle","point":"b","circle":"end"}),
            json!({"id":"oc","kind":"point_on_circle","point":"c","circle":"end"}),
        ];
        if let Some(r) = hole_radius {
            constraints.push(json!({"id":"fh","kind":"fix","point":"hc","at":[8,5]}));
            constraints.push(json!({"id":"rh","kind":"radius","circle":"hole","value":r}));
        }
        let mut sketch = json!({"id":"outline","op":"sketch","points":points,"circles":circles,
            "constraints":constraints,"boundary":["a","b","c","d"],
            "arcs":[{"from":"b","circle":"end"}],"allow_underconstrained":true});
        if hole_radius.is_some() {
            sketch["holes"] = json!(["hole"]);
        }
        json!({"language":"rush/ir-1","units":"mm","parameters":[],"nodes":[sketch,
            {"id":"part","op":"extrude","input":"outline","height":2}],"root":"part"})
    }

    #[test]
    fn solves_circle_radius_and_tangency() {
        let points = [
            json!({"id":"o","position":[0.3,0.2]}),
            json!({"id":"p","position":[-10,6]}),
            json!({"id":"q","position":[10,5]}),
        ];
        let circles = [json!({"id":"c","center":"o","radius":2})];
        let constraints = [
            json!({"id":"fo","kind":"fix","point":"o","at":[0,0]}),
            json!({"id":"fp","kind":"fix","point":"p","at":[-10,7]}),
            json!({"id":"h","kind":"horizontal","a":"p","b":"q"}),
            json!({"id":"d","kind":"distance","a":"p","b":"q","value":20}),
            json!({"id":"t","kind":"tangent_line_circle","a":"p","b":"q","circle":"c"}),
        ];
        let report = solve(&points, &circles, &constraints, "/nodes/0").unwrap();
        assert_eq!(report["status"], "solved", "{report:?}");
        let radius = report["circles"][0]["radius"].as_f64().unwrap();
        assert!((radius - 7.).abs() < 1e-6, "{radius}");
        let t = report["constraints"][4]["residual_mm"].as_f64().unwrap();
        assert!(t.abs() < 1e-6);
    }

    #[test]
    fn refuses_overconstrained_radius_and_unknown_circles() {
        let points = [json!({"id":"o","position":[0,0]})];
        let circles = [json!({"id":"c","center":"o","radius":2})];
        let constraints = [
            json!({"id":"fo","kind":"fix","point":"o","at":[0,0]}),
            json!({"id":"r1","kind":"radius","circle":"c","value":3}),
            json!({"id":"r2","kind":"radius","circle":"c","value":4}),
        ];
        let report = solve(&points, &circles, &constraints, "/nodes/0").unwrap();
        assert_ne!(report["status"], "solved");
        let bad = [json!({"id":"x","kind":"radius","circle":"o","value":1})];
        assert!(solve(&points, &circles, &bad, "").is_err());
    }

    #[test]
    fn emits_arcs_and_subtracts_holes() {
        let compiled = crate::compile(d_shape(None)).unwrap();
        let report = &compiled["sketch_solutions"][0];
        assert!((report["circles"][0]["radius"].as_f64().unwrap() - 5.).abs() < 1e-6);
        let source = compiled["source"].as_str().unwrap();
        assert!(source.contains("polygon(points="), "{source}");
        assert!(source.contains("25"), "arc bulge reaches x=25: {source}");
        let compiled = crate::compile(d_shape(Some(2.))).unwrap();
        let source = compiled["source"].as_str().unwrap();
        assert!(
            source.contains("difference()") && source.contains("circle(r=2"),
            "{source}"
        );
        let error = crate::compile(d_shape(Some(6.))).unwrap_err();
        assert_eq!(error.code, "invalid_boundary");
    }
}
