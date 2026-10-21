//! Original scalar root proofs and continuously checked overlap pcurves.
use super::ContactClass;
use crate::{
    Result, check,
    curve::Curve,
    curve_distance, curve_jets,
    curve_surface_agreement::{self, Status},
    distance_bounds::Interval as I,
    surface::Surface,
};
#[derive(Clone, Debug)]
pub struct CertifiedPlaneCrossing {
    pub curve_parameter: [f64; 2],
    pub uv_bounds: [[f64; 2]; 2],
    pub position_bounds: [[f64; 2]; 3],
    pub contact: ContactClass,
}
#[derive(Clone, Debug)]
pub struct CertifiedCurvePlaneIntersection {
    pub crossings: Vec<CertifiedPlaneCrossing>,
    /// Original curve and independently checked authored pcurve, covering the
    /// full natural source curve when it lies exactly in the containing plane.
    pub overlap: Option<(Curve, Curve)>,
    pub unresolved_curve_domains: Vec<[f64; 2]>,
    pub complete: bool,
    pub cells: usize,
}
fn bounds(c: &Curve, span: usize, d: [f64; 2]) -> Result<Vec<I>> {
    curve_distance::enclosure(c, span, I::new(d[0], d[1])?)
}
fn uv(
    p: &[I],
    s: &Surface,
    axes: [usize; 2],
    origin: [f64; 2],
    end: [f64; 2],
) -> Result<[[f64; 2]; 2]> {
    let d = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut out = [[0.; 2]; 2];
    for k in 0..2 {
        let t = p[axes[k]]
            .sub(I::point(origin[k]))?
            .div_signed(I::point(end[k]).sub(I::point(origin[k]))?)?
            .mul(I::point(d[k][1]).sub(I::point(d[k][0]))?)?
            .add(I::point(d[k][0]))?
            .intersect(d[k][0], d[k][1])?;
        out[k] = [t.lo, t.hi];
    }
    Ok(out)
}
/// Original positive-rational curve against an exactly rectangular axis-aligned
/// affine plane chart. Complete source projection containment is mandatory;
/// possible clipping or other surface parameterizations stay unqualified.
/// Outward interval Newton and original derivative bounds prove scalar root
/// existence/uniqueness; exact clamped source endpoints are handled separately.
/// Coincident source curves retain their complete original geometry and a
/// continuously verified rational UV trace. Nothing authorizes B-rep topology.
/// `tolerance` is spatial localization/overlap agreement, never root proximity.
/// Work is shared across all source spans and overlap verification, 1..100000.
pub fn intersect_curve_plane_certified_report(
    c: &Curve,
    s: &Surface,
    tolerance: f64,
    max_cells: usize,
) -> Result<CertifiedCurvePlaneIntersection> {
    c.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3 && !c.periodic,
        "Certified plane query needs a nonperiodic 3D source curve",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0. && (1..=100000).contains(&max_cells),
        "Certified plane query needs positive tolerance and bounded work",
    )?;
    let whole = c.domain();
    let mut out = CertifiedCurvePlaneIntersection {
        crossings: vec![],
        overlap: None,
        unresolved_curve_domains: vec![],
        complete: false,
        cells: 0,
    };
    let Some((axes, fixed, origin, end)) = crate::ss_intersection::rectangle_chart(s) else {
        out.unresolved_curve_domains.push(whole);
        return Ok(out);
    };
    let level = s.control_points[0][0][fixed];
    if c.control_points.iter().all(|p| p[fixed] < level)
        || c.control_points.iter().all(|p| p[fixed] > level)
    {
        out.complete = true;
        return Ok(out);
    }
    if c.control_points.iter().any(|p| {
        (0..2).any(|k| p[axes[k]] < origin[k].min(end[k]) || p[axes[k]] > origin[k].max(end[k]))
    }) {
        out.unresolved_curve_domains.push(whole);
        return Ok(out);
    }
    if c.control_points.iter().all(|p| p[fixed] == level) {
        let sd = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let mut pcurve = c.clone();
        pcurve.control_points = c
            .control_points
            .iter()
            .map(|p| {
                (0..2)
                    .map(|k| {
                        sd[k][0]
                            + (sd[k][1] - sd[k][0]) * (p[axes[k]] - origin[k])
                                / (end[k] - origin[k])
                    })
                    .collect()
            })
            .collect();
        let proof = curve_surface_agreement::verify(c, &pcurve, s, false, tolerance, max_cells)?;
        out.cells = proof.cells;
        if proof.status == Status::WithinTolerance {
            out.overlap = Some((c.clone(), pcurve));
            out.complete = true;
        } else {
            out.unresolved_curve_domains.push(whole);
        }
        return Ok(out);
    }
    let mut pending: Vec<_> = (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .map(|i| (i, [c.knots[i], c.knots[i + 1]], false, 0usize))
        .collect();
    check(
        pending.len() <= 4096,
        "Certified plane query source span limit exceeded",
    )?;
    while let Some((span, d, exists, depth)) = pending.pop() {
        if out.cells == max_cells {
            out.unresolved_curve_domains.push(d);
            out.unresolved_curve_domains
                .extend(pending.into_iter().map(|(_, d, _, _)| d));
            break;
        }
        out.cells += 1;
        let image = bounds(c, span, d)?;
        let f = image[fixed].sub(I::point(level))?;
        if f.lo > 0. || f.hi < 0. {
            continue;
        }
        let jet = curve_jets::enclose(c, span, d)?;
        let derivative = jet[1][fixed].div(I::point(d[1]).sub(I::point(d[0]))?)?;
        if derivative.lo > 0. || derivative.hi < 0. {
            let clamped_start = c.knots[..=c.degree].iter().all(|&k| k == whole[0]);
            let clamped_end = c.knots[c.control_points.len()..]
                .iter()
                .all(|&k| k == whole[1]);
            let endpoint =
                if d[0] == whole[0] && clamped_start && c.control_points[0][fixed] == level {
                    Some((whole[0], &c.control_points[0]))
                } else if d[1] == whole[1]
                    && clamped_end
                    && c.control_points.last().unwrap()[fixed] == level
                {
                    Some((whole[1], c.control_points.last().unwrap()))
                } else {
                    None
                };
            if let Some((t, p)) = endpoint {
                let point: Vec<_> = p.iter().map(|&x| I::point(x)).collect();
                out.crossings.push(CertifiedPlaneCrossing {
                    curve_parameter: [t, t],
                    uv_bounds: uv(&point, s, axes, origin, end)?,
                    position_bounds: std::array::from_fn(|k| [p[k], p[k]]),
                    contact: ContactClass::Boundary,
                });
                continue;
            }
            let mid = d[0] + (d[1] - d[0]) * 0.5;
            let fm = bounds(c, span, [mid; 2])?[fixed].sub(I::point(level))?;
            let n = I::point(mid).sub(fm.div_signed(derivative)?)?;
            if n.hi < d[0] || n.lo > d[1] {
                continue;
            }
            let root = [n.lo.max(d[0]), n.hi.min(d[1])];
            let proven = exists || (n.lo > d[0] && n.hi < d[1]);
            if proven {
                let p = bounds(c, span, root)?;
                let mut squared = I::point(0.);
                for k in 0..3 {
                    let w = I::point(p[k].hi).sub(I::point(p[k].lo))?;
                    squared = squared.add(w.mul(w)?)?;
                }
                if squared.hi.sqrt().next_up() <= tolerance {
                    out.crossings.push(CertifiedPlaneCrossing {
                        curve_parameter: root,
                        uv_bounds: uv(&p, s, axes, origin, end)?,
                        position_bounds: std::array::from_fn(|k| {
                            if k == fixed {
                                [level, level]
                            } else {
                                [p[k].lo, p[k].hi]
                            }
                        }),
                        contact: ContactClass::Transverse,
                    });
                    continue;
                }
                if depth < 64 && root[0] < root[1] {
                    pending.push((span, root, true, depth + 1));
                    continue;
                }
            }
        }
        let split = d[0] + (d[1] - d[0]) * 0.4375;
        if depth == 64 || split <= d[0] || split >= d[1] {
            out.unresolved_curve_domains.push(d);
            continue;
        }
        // If the parent already had a root, splitting does not assert which
        // child contains it. Each child must independently reestablish existence.
        pending.push((span, [split, d[1]], false, depth + 1));
        pending.push((span, [d[0], split], false, depth + 1));
    }
    out.complete = out.unresolved_curve_domains.is_empty();
    Ok(out)
}
