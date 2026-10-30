//! lib algorithm: min_wall_mm.
use super::*;
pub fn min_wall_mm(section: &LayerSection) -> f64 {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut has_points = false;
    for ring in &section.contours {
        for point in ring {
            has_points = true;
            min_x = min_x.min(point[0]);
            max_x = max_x.max(point[0]);
            min_y = min_y.min(point[1]);
            max_y = max_y.max(point[1]);
        }
    }
    if !has_points {
        return 0.0;
    }
    let span = (max_y - min_y).max(max_x - min_x);
    if !span.is_finite() || span <= 0.0 {
        return 0.0;
    }
    let mut min_wall = f64::INFINITY;
    // Rotation-invariant normal ray probes from edge midpoints
    for (ring_idx, ring) in section.contours.iter().enumerate() {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len = dx.hypot(dy);
            if len <= 1e-9 {
                continue;
            }
            let mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
            for sign in [1.0, -1.0] {
                let dir = [-sign * dy / len, sign * dx / len];
                let probe = [
                    mid[0] + dir[0] * (span * 1e-5),
                    mid[1] + dir[1] * (span * 1e-5),
                ];
                if winding(probe, &section.contours) == 0 {
                    continue;
                }
                let mut best_t = f64::INFINITY;
                for (other_ring_idx, other_ring) in section.contours.iter().enumerate() {
                    for j in 0..other_ring.len() {
                        if ring_idx == other_ring_idx && i == j {
                            continue;
                        }
                        let c = other_ring[j];
                        let d = other_ring[(j + 1) % other_ring.len()];
                        let s = [d[0] - c[0], d[1] - c[1]];
                        let denom = cross2(dir, s);
                        if denom.abs() <= 1e-12 {
                            continue;
                        }
                        let qp = sub2(c, mid);
                        let t = cross2(qp, s) / denom;
                        let u = cross2(qp, dir) / denom;
                        if t > 1e-7 && (0.0..=1.0).contains(&u) && t < best_t {
                            let check_mid =
                                [mid[0] + dir[0] * (t * 0.5), mid[1] + dir[1] * (t * 0.5)];
                            if winding(check_mid, &section.contours) != 0 {
                                best_t = t;
                            }
                        }
                    }
                }
                if best_t.is_finite() {
                    min_wall = min_wall.min(best_t);
                }
            }
        }
    }
    if min_wall.is_finite() {
        return min_wall;
    }
    let step = (span / 32.0).max(span * 1e-4);
    let mut y = min_y + step * 0.5;
    let mut hits = Vec::new();
    while y <= max_y - step * 0.25 {
        hits.clear();
        for ring in &section.contours {
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                let (y0, y1) = (a[1], b[1]);
                if (y0 <= y && y < y1) || (y1 <= y && y < y0) {
                    let t = (y - y0) / (y1 - y0);
                    if t.is_finite() {
                        hits.push(a[0] + (b[0] - a[0]) * t);
                    }
                }
            }
        }
        hits.sort_by(|a, b| a.total_cmp(b));
        let mut pair = 0;
        while pair + 1 < hits.len() {
            let x0 = hits[pair];
            let x1 = hits[pair + 1];
            let mid = [(x0 + x1) * 0.5, y];
            if winding(mid, &section.contours) != 0 {
                min_wall = min_wall.min((x1 - x0).abs());
            }
            pair += 2;
        }
        y += step;
    }
    if min_wall.is_finite() { min_wall } else { 0.0 }
}
