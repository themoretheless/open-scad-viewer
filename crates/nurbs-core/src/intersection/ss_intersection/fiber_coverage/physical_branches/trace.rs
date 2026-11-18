//! Qualified saved world/UV geometry for one embedded original open branch.
//! Every arc junction owns a fresh exact root selector shared by both charts.
//! Canonical endpoint substitution is followed by a fresh curve qualification;
//! independent traces are never joined by endpoint proximity.
use super::*;
use crate::{curve::Curve, distance_bounds::Interval as I};

mod replay;

#[derive(Clone, Copy, Debug)]
pub struct PhysicalFiberTraceLimits {
    pub tolerance_world: f64,
    pub tolerance_uv: f64,
    pub max_cells: usize,
    pub max_evaluations: usize,
    pub max_segments: usize,
}
impl Default for PhysicalFiberTraceLimits {
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
pub struct PhysicalFiberTraceArc {
    chart: usize,
    fiber: QualifiedFiber,
}
impl PhysicalFiberTraceArc {
    pub fn chart(&self) -> usize {
        self.chart
    }
    pub fn fiber(&self) -> &QualifiedFiber {
        &self.fiber
    }
}

#[derive(Clone, Debug)]
pub struct PhysicalFiberTracePort {
    selector: QualifiedFiber,
    bounds: Box4,
    parameters: [f64; 4],
    world: [f64; 3],
}
impl PhysicalFiberTracePort {
    pub fn selector(&self) -> &QualifiedFiber {
        &self.selector
    }
    pub fn parameter_bounds(&self) -> Box4 {
        self.bounds
    }
}

#[derive(Clone, Debug)]
pub struct PhysicalFiberTraceSegment {
    arc: usize,
    proof: QualifiedFiberWorldTrace,
}
impl PhysicalFiberTraceSegment {
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
struct Tubes {
    fibers: [QualifiedFiber; 2],
    world: f64,
    uv: [f64; 2],
}

#[derive(Clone, Debug)]
pub struct QualifiedPhysicalFiberTrace {
    owner: QualifiedPhysicalFiberBranches,
    branch: usize,
    arcs: Vec<PhysicalFiberTraceArc>,
    ports: Vec<PhysicalFiberTracePort>,
    segments: Vec<PhysicalFiberTraceSegment>,
    tubes: Vec<Tubes>,
    limits: PhysicalFiberTraceLimits,
    cells: usize,
    evaluations: usize,
    world_error: f64,
    uv_errors: [f64; 2],
}
impl QualifiedPhysicalFiberTrace {
    pub fn owner(&self) -> &QualifiedPhysicalFiberBranches {
        &self.owner
    }
    pub fn branch(&self) -> usize {
        self.branch
    }
    pub fn arcs(&self) -> &[PhysicalFiberTraceArc] {
        &self.arcs
    }
    pub fn ports(&self) -> &[PhysicalFiberTracePort] {
        &self.ports
    }
    pub fn segments(&self) -> &[PhysicalFiberTraceSegment] {
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
        self.reverify_with_budget(self.limits.max_cells).map(|_| ())
    }
    /// Freshly replay the saved proof and return its additional cell work.
    /// The allowance is independent of, and no larger than, the build limit.
    /// Owner replay remains governed by the original branch-construction limits.
    pub fn reverify_with_budget(&self, max_cells: usize) -> Result<usize> {
        replay::verify(self, true, max_cells)
    }
}

struct Work {
    used: usize,
    max: usize,
}
impl Work {
    fn remaining(&self) -> Result<usize> {
        if self.used >= self.max {
            Err(crate::resource("Physical saved trace work exhausted"))
        } else {
            Ok(self.max - self.used)
        }
    }
}

fn midpoint(d: [f64; 2]) -> f64 {
    (d[0] * 0.5 + d[1] * 0.5).max(d[0]).min(d[1])
}
fn same_source(a: &QualifiedFiber, b: &QualifiedFiber) -> bool {
    a.source().first() == b.source().first() && a.source().second() == b.source().second()
}
fn contained(a: Box4, b: Box4) -> bool {
    (0..4).all(|k| b[k][0] <= a[k][0] && a[k][1] <= b[k][1])
}
fn validate_limits(l: PhysicalFiberTraceLimits) -> Result<()> {
    check(
        l.tolerance_world.is_finite()
            && (1e-9..=1e-2).contains(&l.tolerance_world)
            && l.tolerance_uv.is_finite()
            && (1e-9..=1e-2).contains(&l.tolerance_uv)
            && (1..=1000000).contains(&l.max_cells)
            && (1..=1000000).contains(&l.max_evaluations)
            && (1..=256).contains(&l.max_segments),
        "Invalid physical saved trace limits",
    )
}
fn point_bounds(point: &QualifiedFiber) -> Result<Box4> {
    super::super::closed_branches::cycle::point_bounds(point)
}
fn world_bounds(s: &Surface, box4: Box4, w: &mut Work) -> Result<[[f64; 2]; 3]> {
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut cells = 0;
    for u in s.degree_u..s.control_points.len() {
        if s.knots_u[u] == s.knots_u[u + 1] {
            continue;
        }
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_v[v] == s.knots_v[v + 1] {
                continue;
            }
            let d = [
                [
                    box4[0][0].max(s.knots_u[u]),
                    box4[0][1].min(s.knots_u[u + 1]),
                ],
                [
                    box4[1][0].max(s.knots_v[v]),
                    box4[1][1].min(s.knots_v[v + 1]),
                ],
            ];
            if d.iter().any(|a| a[0] > a[1]) {
                continue;
            }
            w.remaining()?;
            w.used += 1;
            cells += 1;
            let b = crate::surface_distance::enclosure(s, [u, v], d)?;
            for k in 0..3 {
                bounds[k] = [bounds[k][0].min(b[k].lo), bounds[k][1].max(b[k].hi)];
            }
        }
    }
    check(
        cells > 0
            && bounds
                .iter()
                .all(|d| d.iter().all(|x| x.is_finite()) && d[0] <= d[1]),
        "No original physical world root enclosure",
    )?;
    Ok(bounds)
}
fn port(selector: QualifiedFiber, w: &mut Work) -> Result<PhysicalFiberTracePort> {
    let interval = selector.domain()[selector.free_axis().index()];
    let refined = super::super::closed_branches::band::refine_band(
        &selector,
        interval,
        w.remaining()?.min(100000),
    )?;
    w.used += refined.cells;
    let selector = refined.fiber;
    let bounds = point_bounds(&selector)?;
    let world = world_bounds(selector.source().first(), bounds, w)?.map(midpoint);
    Ok(PhysicalFiberTracePort {
        selector,
        parameters: bounds.map(midpoint),
        bounds,
        world,
    })
}
fn tube(
    arc: &PhysicalFiberTraceArc,
    port: &PhysicalFiberTracePort,
    w: &mut Work,
) -> Result<QualifiedFiber> {
    let f = &arc.fiber;
    let axis = f.free_axis().index();
    let bounds = port.bounds;
    let [a, b] = bounds[axis];
    let mid = port.parameters[axis];
    let padding = if a == b {
        0.
    } else {
        (4. * (b - a)).max(32. * (mid.next_up() - mid).abs())
    };
    let interval = [
        (a - padding).max(f.domain()[axis][0]),
        (b + padding).min(f.domain()[axis][1]),
    ];
    let refined =
        super::super::closed_branches::band::refine_band(f, interval, w.remaining()?.min(100000))?;
    w.used += refined.cells;
    let mut d = refined.fiber.domain();
    for k in 0..4 {
        if k != axis {
            d[k] = [d[k][0].min(bounds[k][0]), d[k][1].max(bounds[k][1])];
        }
    }
    let r = f
        .source()
        .certify_contracted(d, f.free_axis(), w.remaining()?.min(100000))?;
    w.used += r.rectangles();
    r.into_fiber()
        .ok_or_else(|| crate::input("Physical port root tube remains unproved"))
}
fn tube_errors(
    fibers: &[QualifiedFiber; 2],
    ports: &[&PhysicalFiberTracePort; 2],
    w: &mut Work,
) -> Result<(f64, [f64; 2])> {
    let mut world = 0_f64;
    let mut uv = [0_f64; 2];
    for (f, port) in fibers.iter().zip(ports) {
        let roots = point_bounds(f)?;
        let b = world_bounds(f.source().first(), roots, w)?;
        let mut width = I::point(0.);
        for k in 0..3 {
            width = width.add(
                I::point(b[k][1].max(port.world[k])).sub(I::point(b[k][0].min(port.world[k])))?,
            )?;
        }
        world = world.max(width.hi);
        for support in 0..2 {
            let mut width = I::point(0.);
            for k in 2 * support..2 * support + 2 {
                width = width.add(
                    I::point(roots[k][1].max(port.parameters[k]))
                        .sub(I::point(roots[k][0].min(port.parameters[k])))?,
                )?;
            }
            uv[support] = uv[support].max(width.hi);
        }
    }
    Ok((world, uv))
}
fn qualify(
    source: &QualifiedFiber,
    mut curves: [Curve; 3],
    start: Option<&PhysicalFiberTracePort>,
    end: Option<&PhysicalFiberTracePort>,
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
    check(remaining >= 2, "No physical saved-curve agreement work")?;
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
    r.into_trace()
        .ok_or_else(|| crate::input("Actual physical world/pcurve coefficients remain unproved"))
}

fn plan(owner: &QualifiedPhysicalFiberBranches, branch: usize) -> Result<Vec<(usize, [f64; 2])>> {
    let b = owner
        .branches()
        .get(branch)
        .ok_or_else(|| crate::input("Unknown physical trace branch"))?;
    let atlas = owner.owner().owner();
    let free = b.free_axis.index();
    let start = owner.vertices()[b.vertices[0]].chart();
    let finish = owner.vertices()[b.vertices[1]].chart();
    let mut current = start;
    let mut low = b.domain[0];
    let mut out = Vec::new();
    loop {
        let high = atlas.fibers()[current].domain()[free][1];
        if high == b.domain[1] {
            check(
                current == finish,
                "Physical trace final chart lost its boundary vertex",
            )?;
            out.push((current, [low, high]));
            return Ok(out);
        }
        let next = b
            .charts
            .iter()
            .copied()
            .filter(|&i| {
                let d = atlas.fibers()[i].domain()[free];
                d[0] <= high && d[1] > high
            })
            .max_by(|&a, &c| {
                let da = atlas.fibers()[a].domain()[free][1];
                let dc = atlas.fibers()[c].domain()[free][1];
                da.total_cmp(&dc)
                    .then_with(|| {
                        let a_finish = da == b.domain[1] && a == finish;
                        let c_finish = dc == b.domain[1] && c == finish;
                        a_finish.cmp(&c_finish)
                    })
                    .then(c.cmp(&a))
            })
            .ok_or_else(|| crate::input("Physical branch chart coverage has no forward arc"))?;
        out.push((current, [low, high]));
        low = high;
        current = next;
        check(
            out.len() <= b.charts.len(),
            "Physical trace chart walk did not advance",
        )?;
    }
}

/// Construct one complete open physical branch. Failure of any shared port,
/// tube, adaptive segment or fresh endpoint qualification publishes nothing.
pub fn construct_physical_fiber_trace(
    owner: &QualifiedPhysicalFiberBranches,
    branch: usize,
    limits: PhysicalFiberTraceLimits,
) -> Result<QualifiedPhysicalFiberTrace> {
    validate_limits(limits)?;
    owner.reverify()?;
    let b = owner
        .branches()
        .get(branch)
        .ok_or_else(|| crate::input("Unknown physical trace branch"))?;
    let atlas = owner.owner().owner();
    let axis = b.free_axis.index();
    let plan = plan(owner, branch)?;
    let mut w = Work {
        used: 0,
        max: limits.max_cells,
    };
    let mut ports = Vec::with_capacity(plan.len() + 1);
    ports.push(port(
        owner.vertices()[b.vertices[0]].point().clone(),
        &mut w,
    )?);
    for i in 1..plan.len() {
        let cut = plan[i - 1].1[1];
        let previous = &atlas.fibers()[plan[i - 1].0];
        let next = &atlas.fibers()[plan[i].0];
        let previous_domain = previous.domain();
        let next_domain = next.domain();
        let mut d = previous_domain;
        for k in 0..4 {
            d[k] = [
                previous_domain[k][0].max(next_domain[k][0]),
                previous_domain[k][1].min(next_domain[k][1]),
            ];
            check(
                d[k][0] <= d[k][1],
                "Physical junction charts have no common root domain",
            )?;
        }
        d[axis] = [cut; 2];
        let r = previous.source().certify(
            d,
            b.free_axis,
            FiberLimits {
                max_rectangles: w.remaining()?.min(100000),
                preconditioner: Some(
                    previous
                        .report()
                        .operator
                        .as_ref()
                        .ok_or_else(|| crate::input("Missing physical chart root operator"))?
                        .preconditioner(),
                ),
            },
        )?;
        w.used += r.report().rectangles;
        let selector = r
            .into_fiber()
            .ok_or_else(|| crate::input("Shared physical arc root remains unproved"))?;
        let bounds = point_bounds(&selector)?;
        check(
            contained(selector.domain(), previous.domain())
                && contained(selector.domain(), next.domain())
                && contained(bounds, previous.domain())
                && contained(bounds, next.domain()),
            "Physical junction root is not shared by both original charts",
        )?;
        ports.push(port(selector, &mut w)?);
    }
    ports.push(port(
        owner.vertices()[b.vertices[1]].point().clone(),
        &mut w,
    )?);

    let tolerance = (limits.tolerance_world.min(limits.tolerance_uv) / 2.).max(1e-12);
    let mut arcs = Vec::with_capacity(plan.len());
    let mut segments = Vec::new();
    let mut tubes = Vec::with_capacity(plan.len());
    let mut evaluations = 0;
    for (i, (chart, interval)) in plan.into_iter().enumerate() {
        let original = &atlas.fibers()[chart];
        check(
            ports[i].parameters[axis] == interval[0]
                && ports[i + 1].parameters[axis] == interval[1]
                && interval[0] < interval[1],
            "Physical trace port clocks collapsed",
        )?;
        let mut d = original.domain();
        d[axis] = interval;
        let r = original.source().certify(
            d,
            original.free_axis(),
            FiberLimits {
                max_rectangles: w.remaining()?.min(100000),
                preconditioner: Some(
                    original
                        .report()
                        .operator
                        .as_ref()
                        .ok_or_else(|| crate::input("Missing physical arc root operator"))?
                        .preconditioner(),
                ),
            },
        )?;
        w.used += r.report().rectangles;
        let inner = r
            .into_fiber()
            .ok_or_else(|| crate::input("Original physical arc clipping remains unproved"))?;
        let arc = PhysicalFiberTraceArc {
            chart,
            fiber: inner,
        };
        let ta = tube(&arc, &ports[i], &mut w)?;
        let tb = tube(&arc, &ports[i + 1], &mut w)?;
        let (world, uv) = tube_errors(
            &[ta.clone(), tb.clone()],
            &[&ports[i], &ports[i + 1]],
            &mut w,
        )?;
        if world > limits.tolerance_world / 4. || uv.iter().any(|v| *v > limits.tolerance_uv / 4.) {
            return Err(crate::input(format!(
                "Physical junction tube exceeds tolerance: world {world}, uv {uv:?}"
            )));
        }
        check(
            evaluations < limits.max_evaluations && segments.len() < limits.max_segments,
            "Physical saved trace allowance exhausted",
        )?;
        let build = super::super::construct_trace::construct_local_fiber_trace(
            &arc.fiber,
            FiberTraceBuildLimits {
                tolerance_world: tolerance,
                max_cells: w.remaining()?.min(100000),
                max_evaluations: (limits.max_evaluations - evaluations).min(16384),
                max_segments: limits.max_segments - segments.len(),
                ..Default::default()
            },
        )?;
        w.used += build.cells;
        evaluations += build.evaluations;
        let reason = format!(
            "Whole physical arc {i} trace remains unproved: {} (cells {}, evaluations {})",
            build.reason, build.cells, build.evaluations
        );
        let pieces = build.trace.ok_or_else(|| crate::input(reason))?;
        check(
            !pieces.is_empty() && segments.len() + pieces.len() <= limits.max_segments,
            "Physical trace segment budget exhausted",
        )?;
        let count = pieces.len();
        for (j, p) in pieces.iter().enumerate() {
            let curves = [
                p.curve().clone(),
                p.pcurves()[0].clone(),
                p.pcurves()[1].clone(),
            ];
            let proof = qualify(
                p.source(),
                curves,
                if j == 0 { Some(&ports[i]) } else { None },
                if j + 1 == count {
                    Some(&ports[i + 1])
                } else {
                    None
                },
                tolerance,
                &mut w,
            )?;
            segments.push(PhysicalFiberTraceSegment { arc: i, proof });
        }
        arcs.push(arc);
        tubes.push(Tubes {
            fibers: [ta, tb],
            world,
            uv,
        });
    }
    let mut result = QualifiedPhysicalFiberTrace {
        owner: owner.clone(),
        branch,
        arcs,
        ports,
        segments,
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
    replay::verify(&result, false, limits.max_cells)?;
    Ok(result)
}

#[cfg(test)]
mod tests;
