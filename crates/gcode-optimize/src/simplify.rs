use gcode_core::PlannedPath;

use crate::travel::dist;
use crate::{Budget, Result};

pub fn simplify_path(path: &PlannedPath, tolerance: f64, budget: &mut Budget) -> Result<PlannedPath> {
    if tolerance == 0.0 || path.points.len() < 3 {
        return Ok(path.clone());
    }
    budget.add(path.points.len())?;
    let points = if path.closed {
        simplify_closed(&path.points, tolerance, budget)?
    } else {
        simplify_open(&path.points, tolerance, budget)?
    };
    Ok(PlannedPath {
        points,
        closed: path.closed,
    })
}

fn simplify_open(points: &[[f64; 2]], tolerance: f64, budget: &mut Budget) -> Result<Vec<[f64; 2]>> {
    douglas_peucker(points, tolerance, budget)
}

fn simplify_closed(
    points: &[[f64; 2]],
    tolerance: f64,
    budget: &mut Budget,
) -> Result<Vec<[f64; 2]>> {
    let mut ring = points.to_vec();
    ring.push(points[0]);
    let mut simplified = douglas_peucker(&ring, tolerance, budget)?;
    if simplified.len() >= 2 && simplified.first() == simplified.last() {
        simplified.pop();
    }
    if simplified.len() < 3 {
        Ok(points.to_vec())
    } else {
        Ok(simplified)
    }
}

fn douglas_peucker(points: &[[f64; 2]], tolerance: f64, budget: &mut Budget) -> Result<Vec<[f64; 2]>> {
    if points.len() <= 2 {
        return Ok(points.to_vec());
    }
    budget.add(points.len())?;
    let first = points[0];
    let last = points[points.len() - 1];
    let mut max_d = 0.0;
    let mut index = 0;
    for (i, point) in points.iter().enumerate().skip(1).take(points.len().saturating_sub(2)) {
        let d = perpendicular_distance(*point, first, last);
        if d > max_d {
            max_d = d;
            index = i;
        }
    }
    if max_d > tolerance && index > 0 {
        let left = douglas_peucker(&points[..=index], tolerance, budget)?;
        let right = douglas_peucker(&points[index..], tolerance, budget)?;
        let mut out = left;
        out.extend_from_slice(&right[1..]);
        Ok(out)
    } else {
        Ok(vec![first, last])
    }
}

fn perpendicular_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = dx.hypot(dy);
    if len == 0.0 {
        return dist(p, a);
    }
    ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / len
}
