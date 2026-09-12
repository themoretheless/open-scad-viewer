//! Closed-region offset via stroke-band + boolean (kurbo/curvex parity).
//!
//! Algorithm (same geometry as curvex `path_offset` / `kurbo::stroke` width `2·|d|`):
//! 1. Normalize source rings under nonzero union.
//! 2. For each contour, build the centered stroke band =
//!    `offset(+|d|) △ offset(-|d|)` (symmetric difference / outer−inner).
//! 3. Union all stroke bands.
//! 4. Outset = source ∪ band; inset = source − band.
//!
//! Nested holes automatically move opposite their parents. Open paths stay on
//! [`crate::path::BezierPath::offset`]'s parallel-polyline branch.
//!
//! Polyline offset joins approximate kurbo's stroke joins (Miter/Round/Bevel);
//! exact cubic stroke outlines remain on [`crate::stroke::outline_stroke`].
use crate::path::{BezierPath, FLATTEN_TOLERANCE};
use crate::rings::{self, Rings, area};
use crate::stroke::LineJoin;
use crate::tessellation::FillRule;
use crate::{Result, check};

const MAX_CONTOURS: usize = 4_096;
const MAX_VERTICES: usize = 100_000;

/// Options for closed-region offset (stroke-band pipeline).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetOptions {
    pub distance: f64,
    pub join: LineJoin,
    pub miter_limit: f64,
    /// Reserved for source normalization; currently rings are nonzero-unioned.
    pub fill_rule: FillRule,
    pub tolerance: f64,
    /// Arc segments for round joins (polyline offset).
    pub segments: usize,
}

impl Default for OffsetOptions {
    fn default() -> Self {
        Self {
            distance: 1.0,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            fill_rule: FillRule::NonZero,
            tolerance: FLATTEN_TOLERANCE,
            segments: 8,
        }
    }
}

impl OffsetOptions {
    pub fn with_distance(distance: f64) -> Self {
        Self {
            distance,
            ..Default::default()
        }
    }
}

/// Parse join name used by bridge / `BezierPath::offset` (`Miter`/`Round`/`Bevel`).
pub fn parse_join(name: &str) -> LineJoin {
    match name {
        "Round" | "round" => LineJoin::Round,
        "Bevel" | "bevel" | "Square" | "square" => LineJoin::Bevel,
        _ => LineJoin::Miter,
    }
}

fn join_name(join: LineJoin) -> &'static str {
    match join {
        LineJoin::Miter => "Miter",
        LineJoin::Round => "Round",
        LineJoin::Bevel => "Bevel",
    }
}

/// Offset a filled multi-contour region (outer + holes) by signed `opts.distance`.
///
/// Positive grows the filled area; negative shrinks it. Empty / collapsed
/// results fail closed.
pub fn offset_closed_rings(rings: &Rings, opts: &OffsetOptions) -> Result<Rings> {
    check(
        opts.distance.is_finite() && opts.distance != 0.0 && opts.distance.abs() <= 1e6,
        "Invalid offset distance",
    )?;
    check(
        opts.miter_limit >= 1.0 && opts.miter_limit.is_finite(),
        "Invalid miter limit",
    )?;
    check(!rings.is_empty(), "Empty offset source")?;
    check(rings.len() <= MAX_CONTOURS, "Offset contour budget exceeded")?;
    let vertex_count: usize = rings.iter().map(|r| r.len()).sum();
    check(
        vertex_count <= MAX_VERTICES,
        "Offset vertex budget exceeded",
    )?;
    for r in rings {
        check(r.len() >= 3, "Offset ring needs ≥3 vertices")?;
        check(
            r.iter().flatten().all(|v| v.is_finite()),
            "Non-finite offset geometry",
        )?;
    }

    let _ = (opts.fill_rule, opts.tolerance, opts.miter_limit);
    let source = rings::planar(rings, &Vec::new(), "union")?;
    check(!source.is_empty(), "Offset source normalized empty")?;

    let half = opts.distance.abs();
    let join = join_name(opts.join);
    let mut bands: Rings = Vec::new();
    for ring in &source {
        let band = stroke_band_for_ring(ring, half, join, opts.segments)?;
        bands.extend(band);
    }
    check(!bands.is_empty(), "Offset stroke band empty")?;
    check(bands.len() <= MAX_CONTOURS, "Offset band contour budget exceeded")?;

    let stroke_region = if bands.len() == 1 {
        bands
    } else {
        rings::planar(&bands, &Vec::new(), "union")?
    };
    check(!stroke_region.is_empty(), "Offset stroke region empty")?;

    let result = if opts.distance > 0.0 {
        rings::planar(&source, &stroke_region, "union")?
    } else {
        rings::planar(&source, &stroke_region, "difference")?
    };
    check(!result.is_empty(), "Offset collapsed the region")?;
    check(
        result.len() <= MAX_CONTOURS,
        "Offset result contour budget exceeded",
    )?;
    Ok(result)
}

/// Centered stroke band of width `2·half` around one closed polyline.
fn stroke_band_for_ring(
    ring: &[[f64; 2]],
    half: f64,
    join: &str,
    segments: usize,
) -> Result<Rings> {
    // `offset_join` keeps rings whose signed area matches the source. Normalize
    // to positive orientation so CW holes (compound) still get a stroke band.
    let mut oriented = ring.to_vec();
    if area(&oriented) < 0.0 {
        oriented.reverse();
    }
    let src_area = area(&oriented).abs();
    let src = vec![oriented];
    let plus = rings::offset_join(&src, half, join, segments)?;
    let mut minus = rings::offset_join(&src, -half, join, segments)?;
    // Large insets can yield a bogus non-shrinking parallel (same/larger area).
    // Treat that as collapsed so the band becomes the outer parallel only and
    // inset boolean can fail closed.
    let minus_area: f64 = minus.iter().map(|r| area(r).abs()).sum();
    if !minus.is_empty() && minus_area + 1e-9 >= src_area {
        minus.clear();
    }
    let plus_area: f64 = plus.iter().map(|r| area(r).abs()).sum();
    if plus.is_empty() && minus.is_empty() {
        return Err(crate::error("Offset parallels collapsed"));
    }
    if plus.is_empty() || minus.is_empty() {
        return Ok(if plus.is_empty() { minus } else { plus });
    }
    if plus_area + 1e-9 < src_area {
        return Err(crate::error("Offset outset parallel shrank"));
    }
    let (outer, inner) = if plus_area >= minus_area {
        (plus, minus)
    } else {
        (minus, plus)
    };
    let band = rings::planar(&outer, &inner, "difference")?;
    check(!band.is_empty(), "Stroke band collapsed")?;
    Ok(band)
}

/// Offset a closed Bézier outer (+ optional holes) → closed polyline paths.
pub fn offset_closed_path(
    outer: &BezierPath,
    holes: &[BezierPath],
    opts: &OffsetOptions,
) -> Result<Vec<BezierPath>> {
    check(outer.closed, "Offset path must be closed")?;
    let mut rings = vec![outer.to_ring(opts.tolerance)?];
    for h in holes {
        check(h.closed, "Offset hole must be closed")?;
        rings.push(h.to_ring(opts.tolerance)?);
    }
    let out = offset_closed_rings(&rings, opts)?;
    out.into_iter()
        .map(|r| BezierPath::from_polyline(&r, true))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::BezierPath;

    #[test]
    fn outset_square_grows() {
        let path = BezierPath::from_rect([0., 0.], [4., 4.]).unwrap();
        let out = offset_closed_path(
            &path,
            &[],
            &OffsetOptions {
                distance: 1.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        let a = area(&out[0].to_ring(0.05).unwrap()).abs();
        assert!((a - 36.0).abs() < 2.0, "area={a}");
    }

    #[test]
    fn inset_square_shrinks() {
        let path = BezierPath::from_rect([0., 0.], [10., 10.]).unwrap();
        let out = offset_closed_path(
            &path,
            &[],
            &OffsetOptions {
                distance: -2.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        let a = area(&out[0].to_ring(0.05).unwrap()).abs();
        assert!((a - 36.0).abs() < 3.0, "area={a}");
    }

    #[test]
    fn inset_too_far_collapses() {
        let path = BezierPath::from_rect([0., 0.], [2., 2.]).unwrap();
        let err = offset_closed_path(
            &path,
            &[],
            &OffsetOptions {
                // Past the inradius (1.0): region must vanish.
                distance: -3.0,
                ..Default::default()
            },
        );
        assert!(err.is_err(), "expected collapse, got {err:?}");
    }

    #[test]
    fn donut_outset_shrinks_hole() {
        let outer = BezierPath::from_rect([0., 0.], [20., 20.]).unwrap();
        let hole = BezierPath::from_rect([6., 6.], [14., 14.])
            .unwrap()
            .reverse();
        let before_hole = area(&hole.to_ring(0.05).unwrap()).abs();
        let out = offset_closed_path(
            &outer,
            &[hole],
            &OffsetOptions {
                distance: 1.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(out.len() >= 2, "expected outer+hole, got {}", out.len());
        let mut areas: Vec<f64> = out
            .iter()
            .map(|p| area(&p.to_ring(0.05).unwrap()).abs())
            .collect();
        areas.sort_by(|a, b| a.total_cmp(b));
        let hole_after = areas[0];
        assert!(
            hole_after < before_hole - 5.0,
            "hole before={before_hole} after={hole_after}"
        );
    }

    #[test]
    fn round_join_outset_ok() {
        let path = BezierPath::from_rect([0., 0.], [5., 5.]).unwrap();
        let out = offset_closed_path(
            &path,
            &[],
            &OffsetOptions {
                distance: 1.0,
                join: LineJoin::Round,
                segments: 12,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        let a = area(&out[0].to_ring(0.05).unwrap()).abs();
        assert!(a > 25.0 + 10.0, "area={a}");
    }
}
