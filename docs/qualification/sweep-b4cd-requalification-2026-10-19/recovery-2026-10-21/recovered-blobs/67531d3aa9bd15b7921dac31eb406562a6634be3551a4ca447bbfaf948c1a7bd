use gcode_core::PlannedPath;

use crate::travel::{dist, dist_sq, path_end, path_start};
use crate::{Budget, Result};

pub fn order_layer(
    paths: &mut Vec<PlannedPath>,
    max_2opt_swaps: usize,
    budget: &mut Budget,
) -> Result<(usize, usize)> {
    if paths.len() < 2 {
        return Ok((0, 0));
    }
    let flips = nearest_neighbor(paths, budget)?;
    let swaps = two_opt(paths, max_2opt_swaps, budget)?;
    Ok((flips, swaps))
}

fn nearest_neighbor(paths: &mut Vec<PlannedPath>, budget: &mut Budget) -> Result<usize> {
    let mut remaining = std::mem::take(paths);
    let mut ordered = Vec::with_capacity(remaining.len());
    let mut flips = 0usize;
    let mut current = remaining
        .first()
        .and_then(|path| path.points.first().copied())
        .unwrap_or([0.0, 0.0]);
    while !remaining.is_empty() {
        budget.add(remaining.len())?;
        let mut best = 0usize;
        let mut best_flip = false;
        let mut best_d_sq = f64::INFINITY;
        for (i, path) in remaining.iter().enumerate() {
            let Some(start) = path_start(path) else {
                continue;
            };
            let Some(end) = path_end(path) else {
                continue;
            };
            let d_start_sq = dist_sq(current, start);
            if d_start_sq < best_d_sq {
                best_d_sq = d_start_sq;
                best = i;
                best_flip = false;
            }
            if !path.closed {
                let d_end_sq = dist_sq(current, end);
                if d_end_sq < best_d_sq {
                    best_d_sq = d_end_sq;
                    best = i;
                    best_flip = true;
                }
            }
        }
        let mut chosen = remaining.swap_remove(best);
        if best_flip {
            chosen.points.reverse();
            flips += 1;
        }
        current = path_end(&chosen).unwrap_or(current);
        ordered.push(chosen);
    }
    *paths = ordered;
    Ok(flips)
}

#[inline]
fn opt_dist(a: Option<[f64; 2]>, b: Option<[f64; 2]>) -> f64 {
    match (a, b) {
        (Some(p), Some(q)) => dist(p, q),
        _ => 0.0,
    }
}

fn two_opt(paths: &mut [PlannedPath], max_swaps: usize, budget: &mut Budget) -> Result<usize> {
    let n = paths.len();
    if n < 4 || max_swaps == 0 {
        return Ok(0);
    }
    let mut swaps = 0usize;
    let mut improved = true;
    while improved && swaps < max_swaps {
        improved = false;
        for i in 0..n.saturating_sub(1) {
            for k in i + 1..n {
                budget.add(1)?;
                let mut before = 0.0;
                let mut after = 0.0;
                if i > 0 {
                    let prev_end = path_end(&paths[i - 1]);
                    before += opt_dist(prev_end, path_start(&paths[i]));
                    after += opt_dist(prev_end, path_end(&paths[k]));
                }
                if k + 1 < n {
                    let next_start = path_start(&paths[k + 1]);
                    before += opt_dist(path_end(&paths[k]), next_start);
                    after += opt_dist(path_start(&paths[i]), next_start);
                }
                if after + 1e-12 < before {
                    reverse_segment(paths, i, k);
                    swaps += 1;
                    improved = true;
                    if swaps >= max_swaps {
                        return Ok(swaps);
                    }
                }
            }
        }
    }
    Ok(swaps)
}

fn reverse_segment(paths: &mut [PlannedPath], i: usize, k: usize) {
    paths[i..=k].reverse();
    for path in &mut paths[i..=k] {
        if !path.closed {
            path.points.reverse();
        }
    }
}
