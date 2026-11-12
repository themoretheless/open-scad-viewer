//! Original-domain correspondence; no rounded trim/extraction is substituted.
use super::{Status, curve_bounds, mapped};
use super::CurveOnSurface;
use crate::{
    Result, check,
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
    surface::Surface,
};

#[derive(Clone, Debug)]
pub struct CorrespondenceReport {
    pub status: Status,
    pub cells: usize,
    /// Present only after every segment of the supplied relation is proved.
    pub distance_upper: Option<f64>,
    /// Full requested relation is retained when proof is incomplete. A mismatch
    /// rejects this relation, not every possible geometric correspondence.
    pub unresolved_relation: Vec<[f64; 2]>,
}
fn interpolate(a: f64, b: f64, t: f64) -> Result<I> {
    if t == 0. {
        return Ok(I::point(a));
    }
    if t == 1. {
        return Ok(I::point(b));
    }
    if a == b {
        return Ok(I::point(a));
    }
    I::point(a)
        .add(I::point(b).sub(I::point(a))?.mul(I::point(t))?)?
        .intersect(a.min(b), a.max(b))
}
fn ends(
    c: &Curve,
    a: f64,
    b: f64,
    cell: [f64; 2],
    reversed: bool,
    physical: bool,
) -> Result<[I; 2]> {
    let domain = c.domain();
    let endpoint = |t| -> Result<I> {
        let value = interpolate(a, b, t)?;
        if !physical {
            return mapped(c, value, reversed);
        }
        let value = if reversed {
            I::point(domain[0]).add(I::point(domain[1]).sub(value)?)?
        } else {
            value
        };
        value.intersect(domain[0], domain[1])
    };
    Ok([endpoint(cell[0])?, endpoint(cell[1])?])
}
fn distance(c: &Curve, p: &Curve, s: &Surface, e: [I; 2], q: [I; 2]) -> Result<Option<(f64, f64)>> {
    let edge = curve_bounds(c, I::new(e[0].lo.min(e[1].lo), e[0].hi.max(e[1].hi))?)?;
    let uv = curve_bounds(p, I::new(q[0].lo.min(q[1].lo), q[0].hi.max(q[1].hi))?)?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let Some(us) = crate::periodic_chart::charts(uv[0], domain[0], s.periodic_u)? else {
        return Ok(None);
    };
    let Some(vs) = crate::periodic_chart::charts(uv[1], domain[1], s.periodic_v)? else {
        return Ok(None);
    };
    let mut box_ = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for u in &us {
        for v in &vs {
            let bounds = crate::surface_distance::rectangle_bounds(
                s,
                [[u.range.lo, u.range.hi], [v.range.lo, v.range.hi]],
            )?;
            for k in 0..3 {
                box_[k][0] = box_[k][0].min(bounds[k][0]);
                box_[k][1] = box_[k][1].max(bounds[k][1]);
            }
        }
    }
    let surface = box_
        .into_iter()
        .map(|x| I::new(x[0], x[1]))
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(box_distance(&edge, &surface)?))
}
/// Verify a continuous monotone piecewise affine map between normalized edge
/// and pcurve parameters. Both coordinates may increase or decrease, but their
/// senses must remain fixed. Partial ranges are allowed and explicitly retained
/// in `nodes`; this function does not claim whole-boundary coverage for them.
/// Binary64 nodes define exact nominal fractions. Outward affine mapping and
/// original homogeneous blossoms include arithmetic uncertainty; no rounded
/// trimmed curve is accepted as evidence for the original boundary.
/// Success bounds every point of this relation, including periodic chart lifts.
/// No inverse/existence search, uniqueness, sewing or solid proof is implied.
pub fn verify_correspondence(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    nodes: &[[f64; 2]],
    tolerance: f64,
    max_cells: usize,
) -> Result<CorrespondenceReport> {
    verify_oriented_correspondence(c, p, s, nodes, false, tolerance, max_cells)
}
/// As `verify_correspondence`, with an outward-mapped reversal of the original
/// edge domain. Nodes refer to its oriented traversal; no reverse-knot proxy is
/// constructed, and `1-t` rounding is enclosed rather than silently stored.
pub fn verify_oriented_correspondence(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    nodes: &[[f64; 2]],
    edge_reversed: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<CorrespondenceReport> {
    let curve = CurveOnSurface {
        curve: c,
        pcurve: p,
        surface: s,
    };
    verify_impl(curve, nodes, edge_reversed, tolerance, max_cells, false)
}
/// Verify an authored piecewise-affine relation in the ORIGINAL physical
/// parameter domains. No rounded normalization to unit fractions is substituted.
/// The 3D edge's oriented clock reflects about its original domain when reversed;
/// pcurve parameters remain independently authored. Partial ranges stay partial.
pub fn verify_parameter_correspondence(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    parameters: &[[f64; 2]],
    edge_reversed: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<CorrespondenceReport> {
    verify_impl(
        CurveOnSurface {
            curve: c,
            pcurve: p,
            surface: s,
        },
        parameters,
        edge_reversed,
        tolerance,
        max_cells,
        true,
    )
}
fn verify_impl(
    curve: CurveOnSurface<'_>,
    nodes: &[[f64; 2]],
    edge_reversed: bool,
    tolerance: f64,
    max_cells: usize,
    physical: bool,
) -> Result<CorrespondenceReport> {
    let CurveOnSurface {
        curve: c,
        pcurve: p,
        surface: s,
    } = curve;
    c.validate()?;
    p.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3 && p.control_points[0].len() == 2,
        "Correspondence needs 3D edge and 2D pcurve",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0. && (1..=100000).contains(&max_cells),
        "Correspondence needs positive tolerance and bounded work",
    )?;
    let domains = if physical {
        [c.domain(), p.domain()]
    } else {
        [[0., 1.]; 2]
    };
    check(
        (2..=1025).contains(&nodes.len())
            && nodes.iter().all(|n| {
                (0..2).all(|k| n[k].is_finite() && n[k] >= domains[k][0] && n[k] <= domains[k][1])
            }),
        "Correspondence needs 2..1025 finite nodes inside its declared clock domains",
    )?;
    let senses = [nodes[1][0] > nodes[0][0], nodes[1][1] > nodes[0][1]];
    check(
        nodes
            .windows(2)
            .all(|w| (0..2).all(|k| w[0][k] != w[1][k] && (w[1][k] > w[0][k]) == senses[k])),
        "Correspondence must be strictly monotone in both coordinates",
    )?;
    let mut report = CorrespondenceReport {
        status: Status::Unresolved,
        cells: 0,
        distance_upper: None,
        unresolved_relation: nodes.to_vec(),
    };
    let mut maximum = 0_f64;
    for pair in nodes.windows(2) {
        let mut stack = vec![[0., 1.]];
        while let Some(cell) = stack.pop() {
            if report.cells == max_cells {
                return Ok(report);
            }
            report.cells += 1;
            let e = ends(c, pair[0][0], pair[1][0], cell, edge_reversed, physical)?;
            let q = ends(p, pair[0][1], pair[1][1], cell, false, physical)?;
            if let Ok(Some(upper)) =
                crate::curve_surface_composition::upper_mapped_cell(c, p, s, e, q)
            {
                if upper <= tolerance {
                    maximum = maximum.max(upper);
                    continue;
                }
            }
            if let Some((_, upper)) = distance(c, p, s, e, q)? {
                if upper <= tolerance {
                    maximum = maximum.max(upper);
                    continue;
                }
            }
            let mid = cell[0] + (cell[1] - cell[0]) * 0.5;
            let witness = distance(
                c,
                p,
                s,
                ends(
                    c,
                    pair[0][0],
                    pair[1][0],
                    [mid, mid],
                    edge_reversed,
                    physical,
                )?,
                ends(p, pair[0][1], pair[1][1], [mid, mid], false, physical)?,
            )?;
            if witness.is_some_and(|(lower, _)| lower > tolerance) {
                report.status = Status::Mismatch;
                return Ok(report);
            }
            if mid == cell[0] || mid == cell[1] {
                return Ok(report);
            }
            stack.push([mid, cell[1]]);
            stack.push([cell[0], mid]);
        }
    }
    report.status = Status::WithinTolerance;
    report.distance_upper = Some(maximum);
    report.unresolved_relation.clear();
    Ok(report)
}

/// Bound transport between two authored oriented clocks of the SAME immutable
/// curve. Original-span rational derivative bounds and outward clock differences
/// include reflection/rounding drift; this is not a rounded reverse-curve proof.
pub fn verify_parameter_clocks(
    c: &Curve,
    nodes: &[[f64; 2]],
    reversed: [bool; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<CorrespondenceReport> {
    c.validate()?;
    let domain = c.domain();
    check(
        (2..=1025).contains(&nodes.len())
            && nodes
                .iter()
                .flatten()
                .all(|v| v.is_finite() && *v >= domain[0] && *v <= domain[1])
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_cells),
        "Curve clock transport needs original-domain nodes and bounded work",
    )?;
    let mut report = CorrespondenceReport {
        status: Status::Unresolved,
        cells: 1,
        distance_upper: None,
        unresolved_relation: nodes.to_vec(),
    };
    let mut delta = 0f64;
    for n in nodes {
        if reversed[0] == reversed[1] && n[0] == n[1] {
            continue;
        }
        let actual = |v: f64, r: bool| -> Result<I> {
            if r {
                I::point(domain[0])
                    .add(I::point(domain[1]).sub(I::point(v))?)?
                    .intersect(domain[0], domain[1])
            } else {
                Ok(I::point(v))
            }
        };
        let d = actual(n[0], reversed[0])?.sub(actual(n[1], reversed[1])?)?;
        delta = delta.max(d.lo.abs().max(d.hi.abs()));
    }
    let (upper, cells) = parameter_clock_error_bound(c, delta, max_cells)?;
    report.cells = cells;
    report.distance_upper = upper;
    if upper.is_some_and(|v| v <= tolerance) {
        report.status = Status::WithinTolerance;
        report.unresolved_relation.clear();
    }
    Ok(report)
}
/// Conditional world-distance bound for two parameters on this ORIGINAL curve
/// whose physical scalar distance is at most `parameter_error_upper`. This does
/// not establish that a caller's clocks satisfy that premise. Both parameters
/// must lie in the original domain. None means exhausted work, never zero error.
pub fn parameter_clock_error_bound(
    c: &Curve,
    parameter_error_upper: f64,
    max_cells: usize,
) -> Result<(Option<f64>, usize)> {
    c.validate()?;
    check(
        parameter_error_upper.is_finite()
            && parameter_error_upper >= 0.
            && (1..=100000).contains(&max_cells),
        "Clock error bound needs finite nonnegative scalar error and bounded work",
    )?;
    let mut cells = 1;
    if parameter_error_upper == 0. {
        return Ok((Some(0.), cells));
    }
    // Original C0 positive rational spans give a global Lipschitz bound.
    let mut speed = 0f64;
    for span in c.degree..c.control_points.len() {
        if c.knots[span] < c.knots[span + 1] {
            if cells == max_cells {
                return Ok((None, cells));
            }
            cells += 1;
            let range = [c.knots[span], c.knots[span + 1]];
            let jets = crate::curve_jets::enclose(c, span, range)?;
            let width = I::point(range[1]).sub(I::point(range[0]))?;
            let mut bound = I::point(0.);
            for v in &jets[1] {
                let d = v.div(width)?;
                bound = bound.add(I::point(d.lo.abs().max(d.hi.abs())))?;
            }
            speed = speed.max(bound.hi);
        }
    }
    Ok((
        Some(I::point(speed).mul(I::point(parameter_error_upper))?.hi),
        cells,
    ))
}
