//! Closed-region offset as source union/difference with a centered stroke band.
//! Curves and round joins are flattened with the requested chord tolerance.
//! Source winding is normalized under its explicit fill rule before offsetting.
use crate::path::{BezierPath, FLATTEN_TOLERANCE};
use crate::rings::{self, Rings};
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
    /// Winding rule of the source, before constructing the offset region.
    pub fill_rule: FillRule,
    pub tolerance: f64,
    /// Minimum full-circle sampling for round joins; tolerance can require more.
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
    check(
        rings.len() <= MAX_CONTOURS,
        "Offset contour budget exceeded",
    )?;
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

    check(
        opts.tolerance > 0. && opts.tolerance.is_finite(),
        "Invalid offset tolerance",
    )?;
    check(
        opts.segments > 0 && opts.segments <= 4096,
        "Invalid offset arc segments",
    )?;
    let source = rings::normalize(rings, opts.fill_rule)?;
    check(!source.is_empty(), "Offset source normalized empty")?;

    let half = opts.distance.abs();
    let mut bands: Rings = Vec::new();
    for ring in &source {
        let path = BezierPath::from_polyline(ring, true)?;
        let stroke = crate::stroke::StrokeOptions {
            width: 2. * half,
            join: opts.join,
            miter_limit: opts.miter_limit,
            ..Default::default()
        };
        let arc_tolerance = half * (1. - (std::f64::consts::PI / opts.segments as f64).cos());
        let tolerance = opts.tolerance.min(arc_tolerance.max(f64::EPSILON * half));
        for contour in crate::stroke::outline_stroke_tol(&path, &stroke, tolerance)? {
            bands.push(contour.to_ring(tolerance)?);
        }
    }
    check(!bands.is_empty(), "Offset stroke band empty")?;
    check(
        bands.len() <= MAX_CONTOURS,
        "Offset band contour budget exceeded",
    )?;

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
    use crate::rings::area;

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
    #[test]
    fn offset_honors_fill_rule_and_reversed_sources() {
        let outer = BezierPath::from_rect([0., 0.], [10., 10.]).unwrap();
        let inner = BezierPath::from_rect([2., 2.], [4., 4.]).unwrap();
        let rings = vec![outer.to_ring(0.01).unwrap(), inner.to_ring(0.01).unwrap()];
        let options = OffsetOptions {
            distance: 0.25,
            ..Default::default()
        };
        let nz = offset_closed_rings(&rings, &options).unwrap();
        let eo = offset_closed_rings(
            &rings,
            &OffsetOptions {
                fill_rule: FillRule::EvenOdd,
                ..options
            },
        )
        .unwrap();
        assert!(rings::inside([3., 3.], &nz));
        assert!(!rings::inside([3., 3.], &eo));
        let reversed = rings
            .iter()
            .map(|r| r.iter().rev().copied().collect())
            .collect();
        let rev = offset_closed_rings(&reversed, &options).unwrap();
        assert!(
            (nz.iter().map(|r| area(r)).sum::<f64>() - rev.iter().map(|r| area(r)).sum::<f64>())
                .abs()
                < 1e-8
        );
    }

    #[test]
    fn offset_miter_limit_changes_acute_corner() {
        let triangle = vec![vec![[0., 0.], [10., 0.], [0., 1.]]];
        let options = OffsetOptions {
            distance: 1.,
            miter_limit: 2.,
            ..Default::default()
        };
        let small = offset_closed_rings(&triangle, &options).unwrap();
        let large = offset_closed_rings(
            &triangle,
            &OffsetOptions {
                miter_limit: 100.,
                ..options
            },
        )
        .unwrap();
        assert!(
            large.iter().map(|r| area(r)).sum::<f64>()
                > small.iter().map(|r| area(r)).sum::<f64>() + 10.
        );
    }

    #[test]
    fn offset_rejects_invalid_tolerance_and_arc_budget() {
        let ring = vec![vec![[0., 0.], [2., 0.], [2., 2.], [0., 2.]]];
        for tolerance in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(
                offset_closed_rings(
                    &ring,
                    &OffsetOptions {
                        tolerance,
                        ..Default::default()
                    }
                )
                .is_err()
            );
        }
        assert!(
            offset_closed_rings(
                &ring,
                &OffsetOptions {
                    segments: usize::MAX,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
