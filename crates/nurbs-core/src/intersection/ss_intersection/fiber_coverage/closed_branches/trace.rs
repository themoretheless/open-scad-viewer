//! Saved paired world/UV curves for a complete original first-return cycle.
//! Ports are exact root selectors, while saved coefficients are tolerance
//! geometry. Root bands at every port certify a monotone endpoint rephasing:
//! outside the two disjoint bands the clock is unchanged; inside each band
//! both clocks stay in the same proved root tube. The tube diameter plus the
//! actual saved-curve root error bounds the complete ORIGINAL closed branch.
use super::*;
use crate::{curve::Curve, distance_bounds::Interval as I};
mod geometry;
mod replay;
use geometry::*;
#[derive(Clone, Copy, Debug)]
pub struct ClosedFiberTraceLimits {
    pub tolerance_world: f64,
    pub tolerance_uv: f64,
    pub max_cells: usize,
    pub max_evaluations: usize,
    pub max_segments: usize,
}
impl Default for ClosedFiberTraceLimits {
    fn default() -> Self {
        Self {
            tolerance_world: 1e-6,
            tolerance_uv: 1e-6,
            max_cells: 1000000,
            max_evaluations: 16384,
            max_segments: 256,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ClosedFiberTraceSegment {
    arc: usize,
    proof: QualifiedFiberWorldTrace,
}
impl ClosedFiberTraceSegment {
    pub fn arc(&self) -> usize {
        self.arc
    }
    pub fn proof(&self) -> &QualifiedFiberWorldTrace {
        &self.proof
    }
    pub fn curve(&self) -> &Curve {
        self.proof.curve()
    }
    pub fn pcurves(&self) -> &[Curve; 2] {
        self.proof.pcurves()
    }
}
#[derive(Clone, Debug)]
struct Port {
    parameters: [f64; 4],
    world: [f64; 3],
}
#[derive(Clone, Debug)]
struct Tubes {
    fibers: [QualifiedFiber; 2],
    world: f64,
    uv: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct QualifiedClosedFiberTrace {
    cycle: OrderedClosedFiberCycle,
    segments: Vec<ClosedFiberTraceSegment>,
    ports: Vec<Port>,
    tubes: Vec<Tubes>,
    limits: ClosedFiberTraceLimits,
    cells: usize,
    evaluations: usize,
    world_error: f64,
    uv_errors: [f64; 2],
}
impl QualifiedClosedFiberTrace {
    pub fn cycle(&self) -> &OrderedClosedFiberCycle {
        &self.cycle
    }
    pub fn segments(&self) -> &[ClosedFiberTraceSegment] {
        &self.segments
    }
    pub fn world_error_upper(&self) -> f64 {
        self.world_error
    }
    pub fn uv_error_upper(&self) -> [f64; 2] {
        self.uv_errors
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn evaluations(&self) -> usize {
        self.evaluations
    }
    pub fn reverify(&self) -> Result<()> {
        replay::verify(self, true)
    }
}
struct Work {
    used: usize,
    max: usize,
}
impl Work {
    fn remaining(&self) -> Result<usize> {
        if self.used >= self.max {
            Err(crate::resource("Closed saved trace work exhausted"))
        } else {
            Ok(self.max - self.used)
        }
    }
}
fn reverse(c: Curve) -> Result<Curve> {
    c.reverse()
}
fn same_source(a: &QualifiedFiber, b: &QualifiedFiber) -> bool {
    a.source().first() == b.source().first() && a.source().second() == b.source().second()
}
fn contained(a: Box4, b: Box4) -> bool {
    (0..4).all(|k| b[k][0] <= a[k][0] && a[k][1] <= b[k][1])
}
fn validate_limits(l: ClosedFiberTraceLimits) -> Result<()> {
    check(
        l.tolerance_world.is_finite()
            && l.tolerance_world >= 1e-9
            && l.tolerance_world <= 1e-2
            && l.tolerance_uv.is_finite()
            && l.tolerance_uv >= 1e-9
            && l.tolerance_uv <= 1e-2
            && (1..=1000000).contains(&l.max_cells)
            && (1..=1000000).contains(&l.max_evaluations)
            && (2..=256).contains(&l.max_segments),
        "Invalid closed saved trace limits",
    )
}
fn tube(
    arc: &OrderedClosedFiberArc,
    port: &ClosedFiberRootPort,
    canonical: &Port,
    w: &mut Work,
) -> Result<QualifiedFiber> {
    let f = arc.fiber();
    let axis = f.free_axis().index();
    let bounds = port.parameter_bounds();
    let [a, b] = bounds[axis];
    let mid = canonical.parameters[axis];
    let padding = if a == b {
        0.
    } else {
        (4. * (b - a)).max(32. * (mid.next_up() - mid).abs())
    };
    let interval = [
        (a - padding).max(f.domain()[axis][0]),
        (b + padding).min(f.domain()[axis][1]),
    ];
    let refined = band::refine_band(f, interval, w.remaining()?.min(100000))?;
    w.used += refined.cells;
    let mut d = refined.fiber.domain();
    for k in 0..4 {
        if k != axis {
            d[k] = [d[k][0].min(bounds[k][0]), d[k][1].max(bounds[k][1])];
        }
    }
    let r = f.source().certify(
        d,
        f.free_axis(),
        FiberLimits {
            max_rectangles: w.remaining()?.min(100000),
            preconditioner: None,
        },
    )?;
    w.used += r.report().rectangles;
    r.into_fiber()
        .ok_or_else(|| crate::input("Shared original port root tube remains unproved"))
}
fn qualify(
    source: &QualifiedFiber,
    mut curves: [Curve; 3],
    start: Option<&Port>,
    end: Option<&Port>,
    tolerance: f64,
    w: &mut Work,
) -> Result<QualifiedFiberWorldTrace> {
    for (at, port) in [(0, start), (1, end)] {
        if let Some(port) = port {
            let index = if at == 0 {
                0
            } else {
                curves[0].control_points.len() - 1
            };
            curves[0].control_points[index] = port.world.to_vec();
            for support in 0..2 {
                let index = if at == 0 {
                    0
                } else {
                    curves[support + 1].control_points.len() - 1
                };
                curves[support + 1].control_points[index] =
                    port.parameters[2 * support..2 * support + 2].to_vec();
            }
        }
    }
    let remaining = w.remaining()?;
    check(remaining >= 2, "No closed saved-curve agreement work")?;
    let r = qualify_fiber_world_trace(
        source,
        &[curves[1].clone(), curves[2].clone()],
        &curves[0],
        FiberWorldTraceLimits {
            tolerance_world: tolerance,
            max_rectangles: (remaining / 2).min(100000),
            max_agreement_cells: (remaining - remaining / 2).min(100000),
        },
    )?;
    w.used += r.rectangles() + r.agreement_cells();
    r.into_trace().ok_or_else(|| {
        crate::input("Actual shared closed world/pcurve coefficients remain unproved")
    })
}
/// Construct the whole ordered component, with literal paired endpoint closure.
/// No partial path is published on exhaustion or a failed endpoint/tube proof.
pub fn construct_closed_fiber_trace(
    cycle: &OrderedClosedFiberCycle,
    limits: ClosedFiberTraceLimits,
) -> Result<QualifiedClosedFiberTrace> {
    validate_limits(limits)?;
    let mut w = Work {
        used: 0,
        max: limits.max_cells,
    };
    let n = cycle.arcs().len();
    let mut ports = Vec::new();
    for p in cycle.ports() {
        let box3 = world_bounds(p.selector().source().first(), p.parameter_bounds(), &mut w)?;
        ports.push(Port {
            parameters: p.parameter_bounds().map(midpoint),
            world: box3.map(midpoint),
        });
    }
    let tolerance = (limits.tolerance_world.min(limits.tolerance_uv) / 2.).max(1e-12);
    let mut segments = Vec::new();
    let mut tubes = Vec::new();
    let mut evaluations = 0;
    for i in 0..n {
        let arc = &cycle.arcs()[i];
        let f = arc.fiber();
        let axis = f.free_axis().index();
        let a = &ports[i];
        let b = &ports[(i + 1) % n];
        let mut interval = [a.parameters[axis], b.parameters[axis]];
        interval.sort_by(f64::total_cmp);
        check(
            interval[0] < interval[1],
            "Closed trace port clocks collapsed",
        )?;
        let mut d = f.domain();
        d[axis] = interval;
        let r = f.source().certify(
            d,
            f.free_axis(),
            FiberLimits {
                max_rectangles: w.remaining()?.min(100000),
                preconditioner: Some(f.report().operator.as_ref().unwrap().preconditioner()),
            },
        )?;
        w.used += r.report().rectangles;
        let inner = r
            .into_fiber()
            .ok_or_else(|| crate::input("Original closed arc clipping remains unproved"))?;
        let ta = tube(arc, &cycle.ports()[i], a, &mut w)?;
        let tb = tube(arc, &cycle.ports()[(i + 1) % n], b, &mut w)?;
        let (world, uv) = tube_errors(&[ta.clone(), tb.clone()], &[a, b], &mut w)?;
        check(
            world <= limits.tolerance_world / 4.
                && uv.iter().all(|v| *v <= limits.tolerance_uv / 4.),
            "Closed junction tube exceeds tolerance",
        )?;
        let build = super::super::construct_trace::construct_local_fiber_trace(
            &inner,
            FiberTraceBuildLimits {
                tolerance_world: tolerance,
                max_cells: w.remaining()?.min(100000),
                max_evaluations: limits.max_evaluations - evaluations,
                max_segments: limits.max_segments - segments.len(),
                ..Default::default()
            },
        )?;
        w.used += build.cells;
        evaluations += build.evaluations;
        let reason = format!(
            "Whole closed arc {i} trace remains unproved: {} (cells {}, evaluations {})",
            build.reason, build.cells, build.evaluations
        );
        let mut pieces = build.trace.ok_or_else(|| crate::input(reason))?;
        check(
            !pieces.is_empty() && segments.len() + pieces.len() <= limits.max_segments,
            "Closed trace segment budget exhausted",
        )?;
        if !arc.forward() {
            pieces.reverse();
        }
        let count = pieces.len();
        for (j, p) in pieces.iter().enumerate() {
            let mut c = [
                p.curve().clone(),
                p.pcurves()[0].clone(),
                p.pcurves()[1].clone(),
            ];
            if !arc.forward() {
                c = [
                    reverse(c[0].clone())?,
                    reverse(c[1].clone())?,
                    reverse(c[2].clone())?,
                ];
            }
            let proof = qualify(
                p.source(),
                c,
                if j == 0 { Some(a) } else { None },
                if j + 1 == count { Some(b) } else { None },
                tolerance,
                &mut w,
            )?;
            segments.push(ClosedFiberTraceSegment { arc: i, proof });
        }
        tubes.push(Tubes {
            fibers: [ta, tb],
            world,
            uv,
        });
    }
    let mut result = QualifiedClosedFiberTrace {
        cycle: cycle.clone(),
        segments,
        ports,
        tubes,
        limits,
        cells: w.used,
        evaluations,
        world_error: 0.,
        uv_errors: [0.; 2],
    };
    let (world, uv) = replay::errors(&result)?;
    result.world_error = world;
    result.uv_errors = uv;
    replay::verify(&result, false)?;
    Ok(result)
}
#[cfg(test)]
mod tests;
