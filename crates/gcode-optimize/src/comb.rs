use gcode_core::PlannedPath;

use crate::travel::{dist, dist_sq, path_end, path_start};
use crate::{Budget, MAX_COMB_WAYPOINTS, Result};

pub struct CombOutcome {
    pub paths: Vec<PlannedPath>,
    pub waypoints: usize,
    pub uncombed: usize,
}

pub fn comb_layer(
    paths: Vec<PlannedPath>,
    contours: &[Vec<[f64; 2]>],
    budget: &mut Budget,
) -> Result<CombOutcome> {
    if paths.is_empty() {
        return Ok(CombOutcome {
            paths: Vec::new(),
            waypoints: 0,
            uncombed: 0,
        });
    }
    let mut out = Vec::with_capacity(paths.len());
    let mut waypoints = 0usize;
    let mut uncombed = 0usize;
    let mut iter = paths.into_iter();
    let first = iter.next().unwrap();
    let mut prev_end = path_end(&first);
    out.push(first);
    for path in iter {
        let to = path_start(&path);
        let next_end = path_end(&path);
        if let (Some(a), Some(b)) = (prev_end, to) {
            budget.add(contours.len().saturating_mul(8))?;
            match comb_travel(a, b, contours, budget)? {
                CombTravel::Direct => {}
                CombTravel::Waypoints(points) => {
                    waypoints += points.len();
                    for point in points {
                        out.push(PlannedPath {
                            points: vec![point],
                            closed: false,
                        });
                    }
                }
                CombTravel::Uncombed => uncombed += 1,
            }
        }
        prev_end = next_end;
        out.push(path);
    }
    Ok(CombOutcome {
        paths: out,
        waypoints,
        uncombed,
    })
}

enum CombTravel {
    Direct,
    Waypoints(Vec<[f64; 2]>),
    Uncombed,
}

fn comb_travel(
    a: [f64; 2],
    b: [f64; 2],
    contours: &[Vec<[f64; 2]>],
    budget: &mut Budget,
) -> Result<CombTravel> {
    if dist_sq(a, b) <= 1e-18 {
        return Ok(CombTravel::Direct);
    }
    let mut hit: Option<usize> = None;
    for (index, ring) in contours.iter().enumerate() {
        budget.add(ring.len())?;
        if chord_hits_ring(a, b, ring) {
            hit = Some(index);
            break;
        }
    }
    let Some(index) = hit else {
        return Ok(CombTravel::Direct);
    };
    let ring = &contours[index];
    if ring.len() < 3 {
        return Ok(CombTravel::Uncombed);
    }
    let i = nearest_vertex(ring, a);
    let j = nearest_vertex(ring, b);
    if i == j {
        return Ok(CombTravel::Uncombed);
    }
    let chosen = shortest_ring_arc(ring, i, j);
    if chosen.len() > MAX_COMB_WAYPOINTS {
        return Ok(CombTravel::Uncombed);
    }
    let mut points = Vec::with_capacity(chosen.len());
    for &p in &chosen {
        if dist_sq(p, a) > 1e-18 && dist_sq(p, b) > 1e-18 {
            points.push(p);
        }
    }
    if points.is_empty() {
        Ok(CombTravel::Uncombed)
    } else {
        Ok(CombTravel::Waypoints(points))
    }
}

fn nearest_vertex(ring: &[[f64; 2]], p: [f64; 2]) -> usize {
    ring.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| dist_sq(**a, p).total_cmp(&dist_sq(**b, p)))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn shortest_ring_arc(ring: &[[f64; 2]], i: usize, j: usize) -> Vec<[f64; 2]> {
    let n = ring.len();
    let mut fwd_len = 0.0;
    let mut fwd_count = 1usize;
    let mut k = i;
    while k != j {
        let next = (k + 1) % n;
        fwd_len += dist(ring[k], ring[next]);
        fwd_count += 1;
        k = next;
    }
    let mut back_len = 0.0;
    let mut back_count = 1usize;
    k = i;
    while k != j {
        let next = (k + n - 1) % n;
        back_len += dist(ring[k], ring[next]);
        back_count += 1;
        k = next;
    }
    if fwd_len <= back_len {
        let mut fwd = Vec::with_capacity(fwd_count);
        k = i;
        loop {
            fwd.push(ring[k]);
            if k == j {
                break;
            }
            k = (k + 1) % n;
        }
        fwd
    } else {
        let mut back = Vec::with_capacity(back_count);
        k = i;
        loop {
            back.push(ring[k]);
            if k == j {
                break;
            }
            k = (k + n - 1) % n;
        }
        back
    }
}

fn chord_hits_ring(a: [f64; 2], b: [f64; 2], ring: &[[f64; 2]]) -> bool {
    if ring.len() < 2 {
        return false;
    }
    let ab_min_x = a[0].min(b[0]);
    let ab_max_x = a[0].max(b[0]);
    let ab_min_y = a[1].min(b[1]);
    let ab_max_y = a[1].max(b[1]);
    let n = ring.len();
    for i in 0..n {
        let c = ring[i];
        let d = ring[if i + 1 < n { i + 1 } else { 0 }];
        if c[0].max(d[0]) < ab_min_x
            || c[0].min(d[0]) > ab_max_x
            || c[1].max(d[1]) < ab_min_y
            || c[1].min(d[1]) > ab_max_y
        {
            continue;
        }
        if segments_intersect(a, b, c, d) {
            return true;
        }
    }
    false
}

fn segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    if o1 == 0.0 || o2 == 0.0 || o3 == 0.0 || o4 == 0.0 {
        return false;
    }
    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}

fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
