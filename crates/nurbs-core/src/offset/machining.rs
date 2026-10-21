//! Outside contour machining: offset paths plus original-boundary clearance.
//! These are planar geometric paths, not machine instructions or pocket clearing.
use crate::{
    Result, check,
    curve::Curve,
    curve_distance, curve_offset_join, curve_offset_wire,
    distance_bounds::Interval as I,
    trim_domain::{Location, TrimDomain},
    trim_region_audit,
};
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub tool_radius: f64,
    pub stock_allowance: f64,
    pub stepover: f64,
    pub passes: usize,
    pub tolerance: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub offset_cells: usize,
    pub clearance_cells: usize,
    pub clearance_pairs: usize,
    pub region_pairs: usize,
    pub region_cells: usize,
    pub domain_cells: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            offset_cells: 65536,
            clearance_cells: 1000000,
            clearance_pairs: 100000,
            region_pairs: 100000,
            region_cells: 100000,
            domain_cells: 1000000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Complete,
    InvalidContour,
    ContourUnresolved,
    OffsetNotQualified,
    OffsetWorkLimit,
    SideUnresolved,
    WrongSide,
    ClearanceWorkLimit,
    ClearanceUnresolved,
    UnsafeClearance,
}
#[derive(Debug)]
pub struct Pass {
    pub curves: Vec<Curve>,
    /// Positive outward source-normal offset magnitude, including proof margin.
    pub offset_distance: f64,
    /// Bound on source-normal offset portions. Corner connectors are separately
    /// designed represented miter/bevel paths and have their own clearance proof.
    pub source_offset_error_upper: f64,
    /// Includes the safety margin, stored offset-distance rounding and fit
    /// deviation relative to the requested nominal tool/stock/pass offset.
    pub nominal_offset_error_upper: f64,
    pub minimum_clearance_bounds: [f64; 2],
    pub outside_proven: bool,
}
pub struct Report {
    /// All-or-nothing plan. Every returned pass is continuously clearance-qualified.
    pub paths: Option<Vec<Pass>>,
    pub source_region: trim_region_audit::Report,
    pub stop: Stop,
    pub qualified_passes: usize,
    /// Exact accepted-fit node count; a failed opaque fit reserves its entire
    /// remaining allowance. Thus this is a conservative charge, not CPU time.
    pub offset_cells_charged: usize,
    pub clearance_cells_charged: usize,
    pub clearance_pairs: usize,
    pub domain_cells: usize,
}
fn chunks(points: &[[f64; 2]]) -> Result<Vec<Curve>> {
    let mut curves = vec![];
    let edges = points.len() - 1;
    for start in (0..edges).step_by(255) {
        let end = (start + 255).min(edges);
        let poles = points[start..=end]
            .iter()
            .map(|p| p.to_vec())
            .collect::<Vec<_>>();
        let mut knots = vec![start as f64; 2];
        knots.extend((start + 1..end).map(|i| i as f64));
        knots.extend([end as f64; 2]);
        let c = Curve {
            degree: 1,
            weights: vec![1.; poles.len()],
            control_points: poles,
            knots,
            periodic: false,
        };
        c.validate()?;
        curves.push(c);
    }
    Ok(curves)
}
fn append(points: &mut Vec<[f64; 2]>, p: [f64; 2]) {
    if points.last() != Some(&p) {
        points.push(p);
    }
}
/// Generate 1..64 nested outside contour passes, ordered from outermost inward.
/// The original contour must be a proved simple closed 2D outer loop with
/// 2..64 authored pieces. Either orientation is supported. A margin of three quarters of
/// the requested tolerance is added to every requested tool/stock offset.
/// Source-normal portions have their own continuous fit bounds. Represented
/// miter/bevel corner connectors are accepted only by the independent global
/// clearance proof. No rapid links, plunges, engagement or pocket coverage are
/// inferred. Source curves are immutable.
pub fn generate(contour: &[Curve], spec: Spec, limits: Limits) -> Result<Report> {
    check(
        (2..=64).contains(&contour.len())
            && (1..=64).contains(&spec.passes)
            && spec.tool_radius.is_finite()
            && spec.tool_radius > 0.
            && spec.tool_radius <= 1e6
            && spec.stock_allowance.is_finite()
            && (0. ..=1e6).contains(&spec.stock_allowance)
            && spec.stepover.is_finite()
            && spec.stepover > 0.
            && spec.stepover <= 1e6
            && spec.tolerance.is_finite()
            && spec.tolerance > 0.
            && spec.tolerance * 0.125 > 0.
            && (1..=65536).contains(&limits.offset_cells)
            && (1..=1000000).contains(&limits.clearance_cells)
            && (1..=100000).contains(&limits.clearance_pairs)
            && (1..=100000).contains(&limits.region_pairs)
            && (1..=100000).contains(&limits.region_cells)
            && (1..=1000000).contains(&limits.domain_cells),
        "Toolpath needs bounded positive tool/step/tolerance, 1..64 passes, 2..64 contour pieces and bounded work",
    )?;
    for c in contour {
        c.validate()?;
        check(
            c.control_points
                .iter()
                .all(|p| p.len() == 2 && p.iter().all(|x| x.abs() <= 1e6)),
            "Toolpath source must have bounded XY controls",
        )?;
    }
    let required = I::point(spec.tool_radius)
        .add(I::point(spec.stock_allowance))?
        .hi;
    let margin = I::point(spec.tolerance).mul(I::point(0.75))?;
    let largest = I::point(required)
        .add(margin)?
        .add(I::point(spec.stepover).mul(I::point((spec.passes - 1) as f64))?)?
        .hi;
    check(
        largest.is_finite() && largest <= 1e6,
        "Toolpath offset exceeds 1e6",
    )?;
    let loops = vec![contour.to_vec()];
    let source_region = trim_region_audit::inspect(
        &loops,
        spec.tolerance * 0.25,
        limits.region_pairs,
        limits.region_cells,
        limits.domain_cells,
    )?;
    let mut report = Report {
        paths: None,
        stop: Stop::ContourUnresolved,
        qualified_passes: 0,
        offset_cells_charged: 0,
        clearance_cells_charged: 0,
        clearance_pairs: 0,
        domain_cells: source_region.domain_cells,
        source_region,
    };
    if report.source_region.valid != Some(true) {
        if report.source_region.valid == Some(false) {
            report.stop = Stop::InvalidContour;
        }
        return Ok(report);
    }
    let winding = report.source_region.winding[0].unwrap_or(0);
    if winding != 1 && winding != -1 {
        return Ok(report);
    }
    let region = TrimDomain::new(&loops, spec.tolerance * 0.25)?;
    let mut paths = vec![];
    for pass in 0..spec.passes {
        let distance = I::point(required)
            .add(margin)?
            .add(I::point(spec.stepover).mul(I::point((spec.passes - 1 - pass) as f64))?)?
            .hi;
        let signed = -(winding as f64) * distance;
        let mut wires = vec![];
        let mut fit_error = 0_f64;
        for source in contour {
            let remaining = limits.offset_cells - report.offset_cells_charged;
            if remaining <= 1 {
                report.stop = Stop::OffsetWorkLimit;
                return Ok(report);
            }
            let wire = match curve_offset_wire::bevel_wire(
                source,
                signed,
                spec.tolerance * 0.125,
                remaining - 1,
            ) {
                Ok(w) => w,
                Err(_) => {
                    report.offset_cells_charged = limits.offset_cells;
                    report.stop = Stop::OffsetNotQualified;
                    return Ok(report);
                }
            };
            let leaves = wire
                .roles
                .iter()
                .filter(|r| matches!(r, curve_offset_wire::Role::Source(_)))
                .count();
            let roots = (source.degree..source.control_points.len())
                .filter(|&i| source.knots[i] < source.knots[i + 1])
                .count();
            report.offset_cells_charged += 2 * leaves - roots;
            fit_error = fit_error.max(wire.error_upper_mm);
            wires.push(wire);
        }
        let mut points = vec![wires[0].edges[0].points[0]];
        for i in 0..wires.len() {
            let wire = &wires[i];
            let next = &wires[(i + 1) % wires.len()];
            for edge in &wire.edges {
                check(
                    points.last() == Some(&edge.points[0]),
                    "Toolpath offset pieces lost stored endpoint connectivity",
                )?;
                append(&mut points, edge.points[1]);
            }
            let a = *points.last().unwrap();
            let b = next.edges[0].points[0];
            if a != b {
                let direction = |w: &curve_offset_wire::Wire, last: bool| -> Option<[f64; 2]> {
                    let indices: Vec<_> = if last {
                        (0..w.edges.len()).rev().collect()
                    } else {
                        (0..w.edges.len()).collect()
                    };
                    for j in indices {
                        let e = &w.edges[j];
                        let d = [
                            e.points[1][0] - e.points[0][0],
                            e.points[1][1] - e.points[0][1],
                        ];
                        if d.iter().any(|x| *x != 0.) {
                            return Some(d);
                        }
                    }
                    None
                };
                if let (Some(u), Some(v)) = (direction(wire, true), direction(next, false)) {
                    if let Ok(join) =
                        curve_offset_join::miter(a, u, b, v, 8. * distance, spec.tolerance * 0.125)
                    {
                        append(&mut points, join.points[1]);
                    }
                }
                append(&mut points, b);
            }
        }
        check(
            points.len() >= 3 && points.first() == points.last(),
            "Toolpath connector failed exact stored closure",
        )?;
        let curves = chunks(&points)?;
        if report.domain_cells == limits.domain_cells {
            report.stop = Stop::SideUnresolved;
            return Ok(report);
        }
        let classification = region.classify_point(
            points[0],
            (limits.domain_cells - report.domain_cells).min(100000),
        )?;
        report.domain_cells += classification.cells;
        if classification.location != Location::Outside {
            report.stop = if classification.location == Location::Inside {
                Stop::WrongSide
            } else {
                Stop::SideUnresolved
            };
            return Ok(report);
        }
        let mut clearance = [f64::INFINITY; 2];
        for c in &curves {
            for source in contour {
                let remaining = limits.clearance_cells - report.clearance_cells_charged;
                if remaining == 0 || report.clearance_pairs == limits.clearance_pairs {
                    report.stop = Stop::ClearanceWorkLimit;
                    return Ok(report);
                }
                report.clearance_pairs += 1;
                let checked = match curve_distance::prove_clearance(
                    c,
                    source,
                    required,
                    remaining.min(100000),
                ) {
                    Ok(r) => r,
                    Err(e) if e.code == "NURBS_RESOURCE_LIMIT" => {
                        report.clearance_cells_charged = limits.clearance_cells;
                        report.stop = Stop::ClearanceWorkLimit;
                        return Ok(report);
                    }
                    Err(e) => return Err(e),
                };
                report.clearance_cells_charged += checked.cells;
                for k in 0..2 {
                    clearance[k] = clearance[k].min(checked.distance_bounds[k]);
                }
                if checked.distance_bounds[0] < required {
                    report.stop = if checked.distance_bounds[1] < required {
                        Stop::UnsafeClearance
                    } else {
                        Stop::ClearanceUnresolved
                    };
                    return Ok(report);
                }
            }
        }
        let nominal = I::point(spec.tool_radius)
            .add(I::point(spec.stock_allowance))?
            .add(I::point(spec.stepover).mul(I::point((spec.passes - 1 - pass) as f64))?)?;
        let gap = I::point(distance).sub(nominal)?;
        let nominal_error = I::point(gap.lo.abs().max(gap.hi.abs()))
            .add(I::point(fit_error))?
            .hi;
        if nominal_error > spec.tolerance {
            report.stop = Stop::OffsetNotQualified;
            return Ok(report);
        }
        // Positive clearance over every connected curve keeps winding constant
        // throughout the closed path. One proved outside witness fixes its side.
        paths.push(Pass {
            curves,
            offset_distance: distance,
            source_offset_error_upper: fit_error,
            nominal_offset_error_upper: nominal_error,
            minimum_clearance_bounds: clearance,
            outside_proven: true,
        });
        report.qualified_passes += 1;
    }
    report.stop = Stop::Complete;
    report.paths = Some(paths);
    Ok(report)
}
