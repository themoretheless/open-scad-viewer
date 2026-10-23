use gcode_core::PlannedPath;

use crate::travel::dist;
use crate::{Budget, Result};

pub fn simplify_path(
    path: &PlannedPath,
    tolerance: f64,
    budget: &mut Budget,
) -> Result<PlannedPath> {
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

fn simplify_open(
    points: &[[f64; 2]],
    tolerance: f64,
    budget: &mut Budget,
) -> Result<Vec<[f64; 2]>> {
    douglas_peucker(points, tolerance, budget)
}

fn simplify_closed(
    points: &[[f64; 2]],
    tolerance: f64,
    budget: &mut Budget,
) -> Result<Vec<[f64; 2]>> {
    let n = points.len();
    let ring_len = n + 1;
    let mut keep = vec![false; ring_len];
    keep[0] = true;
    keep[ring_len - 1] = true;
    let mut stack = Vec::with_capacity(32);
    stack.push((0usize, ring_len - 1));
    let at = |idx: usize| -> [f64; 2] {
        if idx < n { points[idx] } else { points[0] }
    };
    while let Some((start, end)) = stack.pop() {
        let seg_len = end - start + 1;
        if seg_len <= 2 {
            continue;
        }
        budget.add(seg_len)?;
        let first = at(start);
        let last = at(end);
        let dx = last[0] - first[0];
        let dy = last[1] - first[1];
        let len = (dx * dx + dy * dy).sqrt();
        let mut max_d = 0.0;
        let mut index = start;
        for i in start + 1..end {
            let point = at(i);
            let d = if len == 0.0 {
                dist(point, first)
            } else {
                ((point[0] - first[0]) * dy - (point[1] - first[1]) * dx).abs() / len
            };
            if d > max_d {
                max_d = d;
                index = i;
            }
        }
        if max_d > tolerance && index > start {
            keep[index] = true;
            stack.push((index, end));
            stack.push((start, index));
        }
    }
    let count = keep[..n].iter().filter(|&&k| k).count();
    if count < 3 {
        Ok(points.to_vec())
    } else {
        let mut out = Vec::with_capacity(count);
        for (p, &k) in points.iter().zip(&keep[..n]) {
            if k {
                out.push(*p);
            }
        }
        Ok(out)
    }
}

fn douglas_peucker(
    points: &[[f64; 2]],
    tolerance: f64,
    budget: &mut Budget,
) -> Result<Vec<[f64; 2]>> {
    let n = points.len();
    if n <= 2 {
        return Ok(points.to_vec());
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = Vec::with_capacity(32);
    stack.push((0usize, n - 1));
    while let Some((start, end)) = stack.pop() {
        let seg_len = end - start + 1;
        if seg_len <= 2 {
            continue;
        }
        budget.add(seg_len)?;
        let first = points[start];
        let last = points[end];
        let dx = last[0] - first[0];
        let dy = last[1] - first[1];
        let len = (dx * dx + dy * dy).sqrt();
        let mut max_d = 0.0;
        let mut index = start;
        for (offset, point) in points[start + 1..end].iter().enumerate() {
            let d = if len == 0.0 {
                dist(*point, first)
            } else {
                ((point[0] - first[0]) * dy - (point[1] - first[1]) * dx).abs() / len
            };
            if d > max_d {
                max_d = d;
                index = start + 1 + offset;
            }
        }
        if max_d > tolerance && index > start {
            keep[index] = true;
            stack.push((index, end));
            stack.push((start, index));
        }
    }
    let count = keep.iter().filter(|&&k| k).count();
    let mut out = Vec::with_capacity(count);
    for (p, &k) in points.iter().zip(&keep) {
        if k {
            out.push(*p);
        }
    }
    Ok(out)
}
