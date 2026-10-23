//! Parallel plane sections with original-surface, continuous error bounds.
//! Admits complete regular graphs in either natural parameter direction.
//! Other topology, tangencies and resource exhaustion remain explicitly unresolved.
use crate::{
    Error, Result, check,
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
    surface::Surface,
    surface_distance::rectangle_bounds,
    surface_measure::jets,
};
use std::collections::BTreeMap;
type Domain = [[f64; 2]; 2];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Empty,
    Approximate,
    Unresolved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Complete,
    ChartNotProven,
    CompleteGraphNotProven,
    WorkLimit,
    PrecisionLimit,
}
#[derive(Clone, Debug)]
pub struct Section {
    pub level: f64,
    pub status: Status,
    pub stop: Stop,
    pub curves: Vec<Curve>,
    pub error_upper: Option<f64>,
    /// Complete original natural domain remains an obligation on any refusal.
    pub unresolved_domain: Option<Domain>,
    /// The solved parameter axis; the other axis parameterizes the graph.
    pub solved_axis: Option<usize>,
    pub work: usize,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub sections: Vec<Section>,
    pub work: usize,
    pub preparation_work: usize,
    pub complete: bool,
}
struct Budget {
    used: usize,
    max: usize,
}
impl Budget {
    fn charge(&mut self) -> Result<()> {
        if self.used == self.max {
            return Err(Error::new(
                "NURBS_SECTION_WORK_LIMIT",
                "Parallel section work exhausted",
            ));
        }
        self.used += 1;
        Ok(())
    }
}
fn domain(s: &Surface) -> Domain {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn bounds(s: &Surface, d: Domain, budget: &mut Budget) -> Result<Vec<I>> {
    budget.charge()?;
    rectangle_bounds(s, d)?
        .iter()
        .map(|x| I::new(x[0], x[1]))
        .collect()
}
fn dot(n: [f64; 3], p: &[I]) -> Result<I> {
    let mut value = I::point(0.);
    for k in 0..3 {
        value = value.add(I::point(n[k]).mul(p[k])?)?;
    }
    Ok(value)
}
fn negative(x: I) -> Result<I> {
    I::new(-x.hi, -x.lo)
}
fn c0(degree: usize, knots: &[f64], count: usize) -> bool {
    let (a, b) = (knots[degree], knots[count]);
    let mut i = 0;
    while i < knots.len() {
        let mut j = i + 1;
        while j < knots.len() && knots[j] == knots[i] {
            j += 1;
        }
        if knots[i] > a && knots[i] < b && j - i > degree {
            return false;
        }
        i = j;
    }
    true
}
#[derive(Clone, Copy)]
struct Graph {
    axis: usize,
    sign: f64,
    derivative: I,
}
fn charts(s: &Surface, n: [f64; 3], budget: &mut Budget) -> Result<Vec<Graph>> {
    if !c0(s.degree_u, &s.knots_u, s.control_points.len())
        || !c0(s.degree_v, &s.knots_v, s.control_points[0].len())
    {
        return Ok(vec![]);
    }
    let mut derivative = [None::<I>; 2];
    for u in s.degree_u..s.control_points.len() {
        if s.knots_u[u] == s.knots_u[u + 1] {
            continue;
        }
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_v[v] == s.knots_v[v + 1] {
                continue;
            }
            let d = [
                [s.knots_u[u], s.knots_u[u + 1]],
                [s.knots_v[v], s.knots_v[v + 1]],
            ];
            budget.charge()?;
            let j = jets::calculate(s, [u, v], d, None)?;
            let a = j[1][0];
            let b = j[0][1];
            let mut regular = false;
            for k in 0..3 {
                let x = (k + 1) % 3;
                let y = (k + 2) % 3;
                let cross = a[x].mul(b[y])?.sub(a[y].mul(b[x])?)?;
                regular |= cross.lo > 0. || cross.hi < 0.;
            }
            if !regular {
                return Ok(vec![]);
            }
            for axis in 0..2 {
                let tangent = if axis == 0 { a } else { b };
                let width = I::point(d[axis][1]).sub(I::point(d[axis][0]))?;
                let next = dot(n, &tangent)?.div(width)?;
                derivative[axis] = Some(derivative[axis].map_or(next, |old| I {
                    lo: old.lo.min(next.lo),
                    hi: old.hi.max(next.hi),
                }));
            }
        }
    }
    let mut out = vec![];
    for axis in [1, 0] {
        if let Some(d) = derivative[axis] {
            if d.lo > 0. {
                out.push(Graph {
                    axis,
                    sign: 1.,
                    derivative: d,
                });
            } else if d.hi < 0. {
                out.push(Graph {
                    axis,
                    sign: -1.,
                    derivative: negative(d)?,
                });
            }
        }
    }
    Ok(out)
}
fn residual(
    s: &Surface,
    d: Domain,
    n: [f64; 3],
    level: f64,
    g: Graph,
    b: &mut Budget,
) -> Result<I> {
    let f = dot(n, &bounds(s, d, b)?)?.sub(I::point(level))?;
    if g.sign > 0. { Ok(f) } else { negative(f) }
}
fn root_band(
    s: &Surface,
    mut d: Domain,
    n: [f64; 3],
    level: f64,
    g: Graph,
    iterations: usize,
    b: &mut Budget,
) -> Result<Domain> {
    let mut root = I::new(d[g.axis][0], d[g.axis][1])?;
    for _ in 0..iterations {
        let mid = root.lo * 0.5 + root.hi * 0.5;
        if mid < root.lo || mid > root.hi {
            break;
        }
        let mut query = d;
        query[g.axis] = [mid, mid];
        let f = residual(s, query, n, level, g, b)?;
        let next = I::point(mid)
            .sub(f.div(g.derivative)?)?
            .intersect(root.lo, root.hi)?;
        if next == root {
            break;
        }
        root = next;
    }
    d[g.axis] = [root.lo, root.hi];
    Ok(d)
}
fn unresolved(level: f64, d: Domain, stop: Stop) -> Section {
    Section {
        level,
        status: Status::Unresolved,
        stop,
        curves: vec![],
        error_upper: None,
        unresolved_domain: Some(d),
        solved_axis: None,
        work: 0,
    }
}
fn run(
    s: &Surface,
    n: [f64; 3],
    level: f64,
    tolerance: f64,
    graphs: &[Graph],
    b: &mut Budget,
) -> Result<Section> {
    let whole = domain(s);
    let mut selected = None;
    for &g in graphs {
        let mut low = whole;
        low[g.axis] = [whole[g.axis][0]; 2];
        let mut high = whole;
        high[g.axis] = [whole[g.axis][1]; 2];
        // Strict signs prove one interior root for every free parameter.
        if residual(s, low, n, level, g, b)?.hi < 0. && residual(s, high, n, level, g, b)?.lo > 0. {
            selected = Some(g);
            break;
        }
    }
    let Some(g) = selected else {
        return Ok(unresolved(
            level,
            whole,
            if graphs.is_empty() {
                Stop::ChartNotProven
            } else {
                Stop::CompleteGraphNotProven
            },
        ));
    };
    let free = 1 - g.axis;
    let mut pending = vec![whole[free]];
    let mut accepted = vec![];
    let mut endpoints = BTreeMap::<u64, [f64; 3]>::new();
    while let Some(t) = pending.pop() {
        let mut cell = whole;
        cell[free] = t;
        let band = root_band(s, cell, n, level, g, 8, b)?;
        let target = bounds(s, band, b)?;
        let mut points = [[0.; 3]; 2];
        for k in 0..2 {
            if let Some(p) = endpoints.get(&t[k].to_bits()) {
                points[k] = *p;
                continue;
            }
            let mut d = whole;
            d[free] = [t[k]; 2];
            let band = root_band(s, d, n, level, g, 24, b)?;
            let p = bounds(s, band, b)?;
            points[k] = std::array::from_fn(|j| p[j].lo * 0.5 + p[j].hi * 0.5);
            endpoints.insert(t[k].to_bits(), points[k]);
        }
        let chord: Vec<_> = (0..3)
            .map(|k| {
                I::new(
                    points[0][k].min(points[1][k]),
                    points[0][k].max(points[1][k]),
                )
            })
            .collect::<Result<_>>()?;
        let error = box_distance(&target, &chord)?.1;
        if error <= tolerance {
            accepted.push((t, points, error));
            continue;
        }
        let mid = t[0] * 0.5 + t[1] * 0.5;
        if !(t[0] < mid && mid < t[1]) {
            return Ok(unresolved(level, whole, Stop::PrecisionLimit));
        }
        pending.push([mid, t[1]]);
        pending.push([t[0], mid]);
    }
    accepted.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]));
    let mut curves = vec![];
    for chunk in accepted.chunks(255) {
        let mut points = vec![chunk[0].1[0].to_vec()];
        points.extend(chunk.iter().map(|c| c.1[1].to_vec()));
        let mut knots = vec![chunk[0].0[0]; 2];
        knots.extend(chunk.iter().take(chunk.len() - 1).map(|c| c.0[1]));
        knots.extend([chunk.last().unwrap().0[1]; 2]);
        let curve = Curve {
            degree: 1,
            weights: vec![1.; points.len()],
            control_points: points,
            knots,
            periodic: false,
        };
        curve.validate()?;
        curves.push(curve);
    }
    Ok(Section {
        level,
        status: Status::Approximate,
        stop: Stop::Complete,
        curves,
        error_upper: Some(accepted.iter().map(|c| c.2).fold(0., f64::max)),
        unresolved_domain: None,
        solved_axis: Some(g.axis),
        work: 0,
    })
}
/// Section the complete natural chart by n dot S = level for every supplied
/// level. Normal is not normalized: levels use the same equation coefficients.
/// One shared budget counts original spatial enclosure and jet queries. Empty
/// is proved by an outward whole-image enclosure; Approximate requires a
/// complete regular, strictly monotone graph and continuous original-to-stored
/// curve bounds. Every other case retains the full original domain obligation.
pub fn intersect(
    s: &Surface,
    normal: [f64; 3],
    levels: &[f64],
    tolerance: f64,
    max_work: usize,
) -> Result<Report> {
    s.validate()?;
    check(
        normal.iter().all(|x| x.is_finite() && x.abs() <= 1e6)
            && normal.iter().any(|x| *x != 0.)
            && (1..=1024).contains(&levels.len())
            && levels.iter().all(|x| x.is_finite() && x.abs() <= 1e12)
            && tolerance.is_finite()
            && tolerance > 0.
            && (1..=1000000).contains(&max_work)
            && s.control_points
                .iter()
                .flatten()
                .all(|p| p.len() == 3 && p.iter().all(|x| x.abs() <= 1e6)),
        "Sections need bounded 3D controls, nonzero bounded normal, 1..1024 finite levels, positive tolerance and 1..1000000 work",
    )?;
    let mut budget = Budget {
        used: 0,
        max: max_work,
    };
    let whole = domain(s);
    let image = dot(normal, &bounds(s, whole, &mut budget)?)?;
    let mut graphs = None;
    let mut preparation_work = 1;
    let mut sections = vec![];
    for &level in levels {
        let f = image.sub(I::point(level))?;
        if f.lo > 0. || f.hi < 0. {
            sections.push(Section {
                level,
                status: Status::Empty,
                stop: Stop::Complete,
                curves: vec![],
                error_upper: None,
                unresolved_domain: None,
                solved_axis: None,
                work: 0,
            });
            continue;
        }
        if graphs.is_none() {
            let before = budget.used;
            match charts(s, normal, &mut budget) {
                Ok(g) => graphs = Some(g),
                Err(e) if e.code == "NURBS_SECTION_WORK_LIMIT" => graphs = Some(vec![]),
                Err(e) => return Err(e),
            }
            preparation_work += budget.used - before;
        }
        let before = budget.used;
        let mut section = if budget.used == budget.max {
            unresolved(level, whole, Stop::WorkLimit)
        } else {
            match run(
                s,
                normal,
                level,
                tolerance,
                graphs.as_ref().unwrap(),
                &mut budget,
            ) {
                Ok(r) => r,
                Err(e) if e.code == "NURBS_SECTION_WORK_LIMIT" => {
                    unresolved(level, whole, Stop::WorkLimit)
                }
                Err(e) => return Err(e),
            }
        };
        section.work = budget.used - before;
        sections.push(section);
    }
    let complete = sections.iter().all(|s| s.status != Status::Unresolved);
    Ok(Report {
        sections,
        work: budget.used,
        preparation_work,
        complete,
    })
}

/// Internal completeness receipt for one original regular parameter graph.
/// The endpoint coordinates below are numerical representatives of certified
/// root bands, not exact intersection geometry or trim ownership evidence.
pub(crate) struct GraphCoverage {
    pub endpoints: [[f64; 2]; 2],
}
pub(crate) fn prove_graph_coverage(
    s: &Surface, normal: [f64; 3], level: f64, max_work: usize,
) -> Result<Option<GraphCoverage>> {
    let mut budget = Budget { used: 0, max: max_work };
    let whole = domain(s);
    for g in charts(s, normal, &mut budget)? {
        let mut low = whole;
        low[g.axis] = [whole[g.axis][0]; 2];
        let mut high = whole;
        high[g.axis] = [whole[g.axis][1]; 2];
        if residual(s, low, normal, level, g, &mut budget)?.hi >= 0.
            || residual(s, high, normal, level, g, &mut budget)?.lo <= 0. {
            continue;
        }
        let mut endpoints = [[0.; 2]; 2];
        for i in 0..2 {
            let mut query = whole;
            query[1-g.axis] = [whole[1-g.axis][i]; 2];
            let band = root_band(s, query, normal, level, g, 16, &mut budget)?;
            endpoints[i] = std::array::from_fn(|k| band[k][0] + (band[k][1]-band[k][0])*0.5);
        }
        return Ok(Some(GraphCoverage { endpoints }));
    }
    Ok(None)
}
