//! lib algorithm: arc_fit_polyline.
use super::*;
/// Welds chains of short `G1` polyline segments into smooth circular `G2` (CW) and `G3` (CCW)
/// arcs wherever consecutive vertices lie within `config.tolerance_mm` of a common circle.
pub fn arc_fit_polyline(points: &[[f64; 2]], closed: bool, config: &ArcWelderConfig) -> Vec<FittedSegment> {
    if points.len() < 2 {
        return Vec::new();
    }
    let mut seq = points.to_vec();
    if closed && seq.first() != seq.last() {
        seq.push(seq[0]);
    }
    let n = seq.len();
    let min_pts = config.min_arc_points.max(3);
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < n {
        if i + min_pts <= n {
            let mut best_end = None;
            let mut best_circle = None;
            for end in (i + min_pts - 1)..n {
                let mid = i + (end - i) / 2;
                let p_start = seq[i];
                let p_mid = seq[mid];
                let p_end = seq[end];
                if (p_end[0] - p_start[0]).hypot(p_end[1] - p_start[1]) < 1e-6 {
                    break;
                }
                let Some((center, radius, cw)) = circumcircle(p_start, p_mid, p_end) else {
                    break;
                };
                if radius > config.max_radius_mm {
                    break;
                }
                let mut valid = true;
                for w in i..=end {
                    let dist = (seq[w][0] - center[0]).hypot(seq[w][1] - center[1]);
                    if (dist - radius).abs() > config.tolerance_mm {
                        valid = false;
                        break;
                    }
                    if w > i && w < end {
                        let c_step = (seq[w][0] - seq[w - 1][0]) * (seq[w + 1][1] - seq[w][1])
                            - (seq[w][1] - seq[w - 1][1]) * (seq[w + 1][0] - seq[w][0]);
                        if (c_step < 0.0) != cw {
                            valid = false;
                            break;
                        }
                    }
                }
                if valid {
                    best_end = Some(end);
                    best_circle = Some((center, radius, cw));
                } else {
                    break;
                }
            }
            if let (Some(end), Some((center, radius, clockwise))) = (best_end, best_circle) {
                let start = seq[i];
                let end_pt = seq[end];
                out.push(FittedSegment::Arc {
                    start,
                    end: end_pt,
                    ij_offset: [center[0] - start[0], center[1] - start[1]],
                    radius,
                    clockwise,
                });
                i = end;
                continue;
            }
        }
        out.push(FittedSegment::Linear {
            start: seq[i],
            end: seq[i + 1],
        });
        i += 1;
    }
    out
}
