//! Winding transfer at an anchor avoided by the entire UV homotopy.
use super::{
    IsotopyLimits, IsotopyReport, PeriodicWindingReport, Report, inspect_periodic_winding,
    verify_isotopy,
};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};
fn directional_hulls(source: &Curve, candidate: &super::Piece, domain: [f64; 2]) -> Result<Vec<I>> {
    let projected = |curve: &Curve, translation: [f64; 2]| -> Result<Vec<I>> {
        let span = (curve.degree..curve.control_points.len())
            .find(|&k| {
                curve.knots[k] < curve.knots[k + 1]
                    && curve.knots[k] <= domain[0]
                    && domain[1] <= curve.knots[k + 1]
            })
            .ok_or_else(|| crate::input("Anchor projection lacks its original knot cell"))?;
        let net =
            crate::curve_distance::restricted_controls(curve, span, I::new(domain[0], domain[1])?)?;
        super::isotopy::ISOTOPY_DIRECTIONS
            .iter()
            .map(|normal| {
                let origin = &curve.control_points[span - curve.degree];
                let offset = (0..2).try_fold(I::point(0.), |v, a| {
                    v.add(
                        I::point(origin[a])
                            .add(I::point(translation[a]))?
                            .mul(I::point(normal[a]))?,
                    )
                })?;
                let mut lo = f64::INFINITY;
                let mut hi = f64::NEG_INFINITY;
                for pole in &net {
                    let value = pole[0]
                        .mul(I::point(normal[0]))?
                        .add(pole[1].mul(I::point(normal[1]))?)?
                        .div(pole[2])?
                        .add(offset)?;
                    lo = lo.min(value.lo);
                    hi = hi.max(value.hi);
                }
                I::new(lo, hi)
            })
            .collect()
    };
    let a = projected(source, [0.; 2])?;
    let b = projected(&candidate.curve, candidate.translation)?;
    a.iter()
        .zip(b)
        .map(|(a, b)| I::new(a.lo.min(b.lo), a.hi.max(b.hi)))
        .collect()
}
#[derive(Clone, Copy, Debug)]
pub struct AnchorLimits {
    pub isotopy: IsotopyLimits,
    pub tolerance_uv: f64,
    pub max_images: usize,
    pub max_cells: usize,
    /// Shared cell/image enclosure comparisons, including inconclusive ones,
    /// and additional original-control enclosure constructions.
    pub max_checks: usize,
}
#[derive(Clone, Debug)]
pub struct AnchorTransfer {
    pub isotopy: IsotopyReport,
    /// Winding of both source and lifted candidate at this periodic anchor.
    /// Only published after the full homotopy avoids every anchor image.
    pub winding: Option<i32>,
    pub source: Option<PeriodicWindingReport>,
    pub checks: usize,
    pub reason: Option<&'static str>,
}
/// Certify the integer used to calibrate a tolerance-qualified chart assembly.
/// Recomputes isotopy from the actual curves; no caller-authored certificate
/// is trusted. A box containing an anchor image is inconclusive, not proof
/// of an intersection. No boundary snapping or sample-only acceptance.
pub fn verify_anchor_winding(
    sources: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: AnchorLimits,
) -> Result<AnchorTransfer> {
    check(
        anchor.iter().all(|x| x.is_finite())
            && limits.tolerance_uv.is_finite()
            && limits.tolerance_uv > 0.
            && (1..=4096).contains(&limits.max_images)
            && (1..=100000).contains(&limits.max_cells)
            && (1..=100000).contains(&limits.max_checks),
        "Invalid anchor transfer inputs or budgets",
    )?;
    let isotopy = verify_isotopy(sources, partitions, chart, limits.isotopy)?;
    let mut out = AnchorTransfer {
        isotopy,
        winding: None,
        source: None,
        checks: 0,
        reason: None,
    };
    if !out.isotopy.periodic_isotopy_preserved {
        out.reason = Some("periodic-isotopy-unproven");
        return Ok(out);
    }
    let periods = [
        I::point(chart[0][1]).sub(I::point(chart[0][0]))?,
        I::point(chart[1][1]).sub(I::point(chart[1][0]))?,
    ];
    if periods.iter().any(|p| p.lo <= 0.) {
        out.reason = Some("period-unproven");
        return Ok(out);
    }
    for cell in &out.isotopy.cells {
        let mut projected = None;
        let mut ranges = [[0i32; 2]; 2];
        for a in 0..2 {
            let q = I::new(cell.hull[a][0], cell.hull[a][1])?
                .sub(I::point(anchor[a]))?
                .div(periods[a])?;
            let lo = q.lo.floor() - 1.;
            let hi = q.hi.ceil() + 1.;
            if lo < i32::MIN as f64 || hi > i32::MAX as f64 {
                out.reason = Some("anchor-index-range");
                return Ok(out);
            }
            ranges[a] = [lo as i32, hi as i32];
        }
        for u in i64::from(ranges[0][0])..=i64::from(ranges[0][1]) {
            for v in i64::from(ranges[1][0])..=i64::from(ranges[1][1]) {
                if out.checks == limits.max_checks {
                    out.reason = Some("anchor-check-budget");
                    return Ok(out);
                }
                out.checks += 1;
                let mut separated = false;
                for (a, k) in [u, v].into_iter().enumerate() {
                    let q = I::point(anchor[a]).add(periods[a].mul(I::point(k as f64))?)?;
                    separated |= q.hi < cell.hull[a][0] || cell.hull[a][1] < q.lo;
                }
                if !separated {
                    if projected.is_none() {
                        if limits.max_checks - out.checks < 2 {
                            out.reason = Some("anchor-check-budget");
                            return Ok(out);
                        }
                        out.checks += 2;
                        projected = Some(directional_hulls(
                            &sources[cell.loop_index],
                            &partitions[cell.loop_index].pieces[cell.piece_index],
                            cell.domain,
                        )?);
                    }
                    let image = [
                        I::point(anchor[0]).add(periods[0].mul(I::point(u as f64))?)?,
                        I::point(anchor[1]).add(periods[1].mul(I::point(v as f64))?)?,
                    ];
                    for (normal, hull) in super::isotopy::ISOTOPY_DIRECTIONS
                        .iter()
                        .zip(projected.as_ref().unwrap())
                    {
                        if out.checks == limits.max_checks {
                            out.reason = Some("anchor-check-budget");
                            return Ok(out);
                        }
                        out.checks += 1;
                        let q = image[0]
                            .mul(I::point(normal[0]))?
                            .add(image[1].mul(I::point(normal[1]))?)?;
                        if q.hi < hull.lo || hull.hi < q.lo {
                            separated = true;
                            break;
                        }
                    }
                    if !separated {
                        out.reason = Some("anchor-homotopy-clearance-unproven");
                        return Ok(out);
                    }
                }
            }
        }
    }
    let loops: Vec<_> = sources.iter().map(|c| vec![c.clone()]).collect();
    let source = inspect_periodic_winding(
        &loops,
        chart,
        anchor,
        limits.tolerance_uv,
        limits.max_images,
        limits.max_cells,
    )?;
    out.winding = source.winding;
    if out.winding.is_none() {
        out.reason = Some("source-anchor-winding-unproven");
    }
    out.source = Some(source);
    Ok(out)
}
