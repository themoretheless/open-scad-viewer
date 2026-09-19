use gcode_core::PlannedPath;

use crate::travel::{dist, path_end, path_start};
use crate::{Budget, MAX_COMB_WAYPOINTS, Result};

pub struct CombOutcome {
    pub paths: Vec<PlannedPath>,
    pub waypoints: usize,
    pub uncombed: usize,
}

pub fn comb_layer(
    paths: &[PlannedPath],
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
    let mut out = Vec::new();
    let mut waypoints = 0usize;
    let mut uncombed = 0usize;
    out.push(paths[0].clone());
    for window in paths.windows(2) {
        let from = path_end(&window[0]);
        let to = path_start(&window[1]);
        if let (Some(a), Some(b)) = (from, to) {
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
        out.push(window[1].clone());
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
    if dist(a, b) <= 1e-9 {
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
    let (fwd, back) = ring_arcs(ring, i, j);
    let chosen = if path_length(&fwd) <= path_length(&back) {
        fwd
    } else {
        back
    };
    if chosen.len() > MAX_COMB_WAYPOINTS {
        return Ok(CombTravel::Uncombed);
    }
    let mut points = Vec::new();
    for &p in &chosen {
        if dist(p, a) > 1e-9 && dist(p, b) > 1e-9 {
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
        .min_by(|(_, a), (_, b)| dist(**a, p).total_cmp(&dist(**b, p)))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn ring_arcs(ring: &[[f64; 2]], i: usize, j: usize) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let n = ring.len();
    let mut fwd = Vec::new();
    let mut k = i;
    loop {
        fwd.push(ring[k]);
        if k == j {
            break;
        }
        k = (k + 1) % n;
    }
    let mut back = Vec::new();
    k = i;
    loop {
        back.push(ring[k]);
        if k == j {
            break;
        }
        k = (k + n - 1) % n;
    }
    (fwd, back)
}

fn path_length(points: &[[f64; 2]]) -> f64 {
    points.windows(2).map(|p| dist(p[0], p[1])).sum()
}

fn chord_hits_ring(a: [f64; 2], b: [f64; 2], ring: &[[f64; 2]]) -> bool {
    if ring.len() < 2 {
        return false;
    }
    for i in 0..ring.len() {
        let c = ring[i];
        let d = ring[(i + 1) % ring.len()];
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
