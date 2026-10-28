use super::*;

/// Union edge strips and outside join sectors instead of stitching two offset
/// chains across their seams. This also handles concavity and collapsed inner loops.
pub(super) fn stroke_pieces(
    pts: &[[f64; 2]],
    closed: bool,
    opts: &StrokeOptions,
    tolerance: f64,
    pieces: &mut crate::rings::Rings,
) -> Result<()> {
    let n = pts.len();
    check(n >= 2, "Polyline too short")?;
    let half = opts.width * 0.5;
    let edge_count = if closed { n } else { n - 1 };
    for i in 0..edge_count {
        let mut a = pts[i];
        let mut b = pts[(i + 1) % n];
        let d = unit2(sub2(b, a));
        if dist(a, b) <= 1e-12 {
            continue;
        }
        if !closed && opts.cap == LineCap::Square {
            if i == 0 {
                a = sub2(a, scale2(d, half));
            }
            if i == edge_count - 1 {
                b = add2(b, scale2(d, half));
            }
        }
        let normal = scale2(perp(d), half);
        push_piece(
            pieces,
            vec![
                add2(a, normal),
                sub2(a, normal),
                sub2(b, normal),
                add2(b, normal),
            ],
        );
    }
    let joints = if closed { 0..n } else { 1..n - 1 };
    for i in joints {
        let p = pts[i];
        let d0 = unit2(sub2(p, pts[(i + n - 1) % n]));
        let d1 = unit2(sub2(pts[(i + 1) % n], p));
        let cross = crate::rings::cross2(d0, d1);
        let dot = d0[0] * d1[0] + d0[1] * d1[1];
        if cross.abs() < 1e-12 {
            if dot < 0. && opts.join == LineJoin::Round {
                push_piece(
                    pieces,
                    arc_sector(p, half, 0., std::f64::consts::TAU, tolerance)?,
                );
            }
            continue;
        }
        let side = if cross > 0. { -half } else { half };
        let q0 = add2(p, scale2(perp(d0), side));
        let q1 = add2(p, scale2(perp(d1), side));
        let mut wedge = vec![p, q0];
        match opts.join {
            LineJoin::Round => {
                let angle = (q0[1] - p[1]).atan2(q0[0] - p[0]);
                wedge = arc_sector(p, half, angle, cross.atan2(dot), tolerance)?;
            }
            LineJoin::Miter => {
                if let Some(m) = line_intersect(q0, add2(q0, d0), q1, add2(q1, d1))
                    && dist(p, m) <= half * opts.miter_limit
                {
                    wedge.push(m);
                }
                wedge.push(q1);
            }
            LineJoin::Bevel => wedge.push(q1),
        }
        push_piece(pieces, wedge);
    }
    if !closed && opts.cap == LineCap::Round {
        let d0 = sub2(pts[1], pts[0]);
        let d1 = sub2(pts[n - 1], pts[n - 2]);
        // Outside semicircles; their diameter is covered by the edge strip.
        for (p, angle) in [
            (pts[0], d0[1].atan2(d0[0]) + std::f64::consts::FRAC_PI_2),
            (pts[n - 1], d1[1].atan2(d1[0]) - std::f64::consts::FRAC_PI_2),
        ] {
            push_piece(
                pieces,
                arc_sector(p, half, angle, std::f64::consts::PI, tolerance)?,
            );
        }
    }
    check(
        pieces.iter().map(Vec::len).sum::<usize>() <= 100_000,
        "Stroke vertex budget exceeded",
    )?;
    Ok(())
}

fn push_piece(pieces: &mut crate::rings::Rings, mut ring: Vec<[f64; 2]>) {
    let area = crate::rings::area(&ring);
    if area.abs() <= 1e-24 {
        return;
    }
    if area < 0. {
        ring.reverse();
    }
    pieces.push(ring);
}

pub(super) fn arc_sector(
    center: [f64; 2],
    radius: f64,
    start: f64,
    sweep: f64,
    tolerance: f64,
) -> Result<Vec<[f64; 2]>> {
    let step = (2. * (1. - (tolerance / radius).min(1.)).acos()).min(std::f64::consts::FRAC_PI_4);
    check(
        step > 0.,
        "Stroke tolerance below floating point resolution",
    )?;
    let count = (sweep.abs() / step).ceil().max(1.) as usize;
    check(count <= 4096, "Round stroke vertex budget exceeded")?;
    let mut ring = Vec::with_capacity(count + 2);
    ring.push(center);
    for i in 0..=count {
        let angle = start + sweep * i as f64 / count as f64;
        ring.push([
            center[0] + radius * angle.cos(),
            center[1] + radius * angle.sin(),
        ]);
    }
    Ok(ring)
}

pub(super) fn line_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<[f64; 2]> {
    let r = sub2(b, a);
    let s = sub2(d, c);
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-15 {
        return None;
    }
    let qp = sub2(c, a);
    let t = (qp[0] * s[1] - qp[1] * s[0]) / den;
    Some([a[0] + t * r[0], a[1] + t * r[1]])
}
