//! C1 owned source-star disks without imposing a common quadratic pole jet.
use super::{Boundary, Carrier, Class, PoleStrata};
use nurbs_core::{Error, Result, factored_surface_normals::prepare_exact_meridian_tangency};
use std::collections::{BTreeMap, BTreeSet};
/// A local owned star, not every body portion above this projected disk.
pub struct C1PoleNeighborhood {
    vertex: usize,
    axis: usize,
    point: [f64; 3],
    radius: f64,
    faces: Vec<usize>,
    seams: Vec<usize>,
}
impl C1PoleNeighborhood {
    pub fn vertex(&self) -> usize {
        self.vertex
    }
    pub fn dropped_axis(&self) -> usize {
        self.axis
    }
    pub fn point(&self) -> [f64; 3] {
        self.point
    }
    pub fn projected_radius_lower(&self) -> f64 {
        self.radius
    }
    pub fn faces(&self) -> &[usize] {
        &self.faces
    }
    pub fn seams(&self) -> &[usize] {
        &self.seams
    }
}
/// Fresh original gradient identities and complete existing normal coverage
/// remain bound to this immutable boundary and its privately admitted body.
pub struct C1PoleAtlas<'b, 's, E: Carrier> {
    boundary: &'b Boundary<'s, E>,
    neighborhoods: Vec<C1PoleNeighborhood>,
}
impl<'b, 's, E: Carrier> C1PoleAtlas<'b, 's, E> {
    pub fn boundary(&self) -> &'b Boundary<'s, E> {
        self.boundary
    }
    pub fn neighborhoods(&self) -> &[C1PoleNeighborhood] {
        &self.neighborhoods
    }
}
pub struct C1PoleLimits {
    pub incidence_steps: usize,
    pub exact_work: u64,
}
pub struct C1PoleReport<'b, 's, E: Carrier> {
    pub certificate: Option<C1PoleAtlas<'b, 's, E>>,
    pub steps: usize,
    pub exact_work: u64,
    pub completed_neighborhoods: usize,
    pub uncertain_vertex: Option<usize>,
    pub uncertain_edge: Option<usize>,
    pub reason: &'static str,
}
fn step<E: Carrier>(r: &mut C1PoleReport<'_, '_, E>, max: usize) -> bool {
    if r.steps == max {
        r.reason = "pole-c1-incidence-work-limit";
        false
    } else {
        r.steps += 1;
        true
    }
}
/// The private graph atlas supplies a single graph disk, punctured chart
/// regularity and a uniform common limiting tangent plane at each pole.
/// Complete native link edges must all retain continuous normal coverage AND
/// fresh exact original graph-gradient identities. Those identities extend
/// across isolated projection zeros by continuity. C1 graphs then glue on
/// the punctured disk and their common uniform gradient limit removes the pole.
/// C0 declarations, tolerance-only agreement, unsupported maps and exhaustion
/// produce no partial atlas. No Hessian, ideal-law or whole-body C1 is inferred.
pub fn qualify_c1_pole_neighborhoods<'b, 's, E: Carrier>(
    boundary: &'b Boundary<'s, E>,
    limits: C1PoleLimits,
) -> Result<C1PoleReport<'b, 's, E>> {
    if !(1..=1000000).contains(&limits.incidence_steps)
        || !(1..=100000000).contains(&limits.exact_work)
    {
        return Err(Error::new(
            "BREP_SOURCE_POLE_C1",
            "Choose bounded pole incidence and exact work",
        ));
    }
    let mut out = C1PoleReport {
        certificate: None,
        steps: 0,
        exact_work: 0,
        completed_neighborhoods: 0,
        uncertain_vertex: None,
        uncertain_edge: None,
        reason: "pole-c1-owned-graph-atlas-required",
    };
    let graphs = match boundary.poles() {
        PoleStrata::TangentGraph(g) => g,
        PoleStrata::FacewiseCurvature(c) => c.graphs(),
        _ => return Ok(out),
    };
    let body = boundary.body();
    let ordinary = boundary.ordinary();
    if !std::ptr::eq(graphs.charts().transverse().atlas().body(), body)
        || !std::ptr::eq(ordinary.body(), body)
    {
        out.reason = "pole-c1-same-body-ownership-unproven";
        return Ok(out);
    }
    if graphs.stars().is_empty() {
        out.reason = "pole-c1-no-owned-stars";
        return Ok(out);
    }
    let mut links = BTreeMap::new();
    for link in &body.geometry().topology().links {
        if !step(&mut out, limits.incidence_steps) {
            return Ok(out);
        }
        if links.insert(link.vertex, link).is_some() {
            out.reason = "pole-c1-owned-link-unproven";
            return Ok(out);
        }
    }
    let mut covered = BTreeSet::new();
    for (index, _) in ordinary.polar_seams() {
        if !step(&mut out, limits.incidence_steps) {
            return Ok(out);
        }
        covered.insert(*index);
    }
    for (index, _) in ordinary.polar_curvature_seams() {
        if !step(&mut out, limits.incidence_steps) {
            return Ok(out);
        }
        covered.insert(*index);
    }
    let shell = body.geometry().shell();
    let mut vertices = BTreeSet::new();
    let mut exact = BTreeSet::new();
    let mut neighborhoods = Vec::new();
    for star in graphs.stars() {
        if !step(&mut out, limits.incidence_steps) {
            return Ok(out);
        }
        out.uncertain_vertex = Some(star.vertex());
        let Some(link) = links.get(&star.vertex()) else {
            out.reason = "pole-c1-owned-link-unproven";
            return Ok(out);
        };
        if !vertices.insert(star.vertex())
            || !star.projected_radius_lower().is_finite()
            || star.projected_radius_lower() <= 0.
        {
            out.reason = "pole-c1-original-graph-unproven";
            return Ok(out);
        }
        let mut faces = BTreeSet::new();
        for &face in star.faces() {
            if !step(&mut out, limits.incidence_steps) {
                return Ok(out);
            }
            faces.insert(face);
        }
        let mut seams = BTreeSet::new();
        for node in &link.cycle {
            if !step(&mut out, limits.incidence_steps) {
                return Ok(out);
            }
            seams.insert(node.edge);
        }
        if faces.is_empty() || seams.len() != faces.len() {
            out.reason = "pole-c1-complete-natural-fan-unproven";
            return Ok(out);
        }
        for &index in &seams {
            if !step(&mut out, limits.incidence_steps) {
                return Ok(out);
            }
            out.uncertain_edge = Some(index);
            let Some(uses) = shell.uses().get(index) else {
                out.reason = "pole-c1-meridian-ownership-unproven";
                return Ok(out);
            };
            if uses.iter().any(|a| {
                !faces.contains(&a.face)
                    || !shell.vertices()[a.face][a.wire][a.edge].contains(&star.vertex())
            }) {
                out.reason = "pole-c1-meridian-ownership-unproven";
                return Ok(out);
            }
            if matches!(ordinary.classes().get(index), None | Some(Class::C0)) {
                out.reason = "pole-c1-explicit-c0-meridian";
                return Ok(out);
            }
            if !covered.contains(&index) {
                out.reason = "pole-c1-complete-normal-coverage-unproven";
                return Ok(out);
            }
            let key = (index, star.dropped_axis());
            if exact.contains(&key) {
                continue;
            }
            if out.exact_work == limits.exact_work {
                out.reason = "pole-c1-exact-work-limit";
                return Ok(out);
            }
            let edge = &shell.edges()[index];
            let (Some(a), Some(b)) = (edge.exact_parameter_curve(0), edge.exact_parameter_curve(1))
            else {
                out.reason = "pole-c1-original-meridian-maps-unproven";
                return Ok(out);
            };
            let r = prepare_exact_meridian_tangency(
                [edge.fragment(0).surface(), edge.fragment(1).surface()],
                [a.0, b.0],
                [a.1, b.1],
                [a.2, b.2],
                star.dropped_axis(),
                limits.exact_work - out.exact_work,
            )?;
            out.exact_work += r.work;
            if r.certificate.is_none() {
                out.reason = r.reason;
                return Ok(out);
            }
            exact.insert(key);
        }
        neighborhoods.push(C1PoleNeighborhood {
            vertex: star.vertex(),
            axis: star.dropped_axis(),
            point: star.point(),
            radius: star.projected_radius_lower(),
            faces: faces.into_iter().collect(),
            seams: seams.into_iter().collect(),
        });
        out.completed_neighborhoods += 1;
        out.uncertain_edge = None;
    }
    out.certificate = Some(C1PoleAtlas {
        boundary,
        neighborhoods,
    });
    out.uncertain_vertex = None;
    out.reason = "original-owned-pole-c1-graph-neighborhoods-qualified";
    Ok(out)
}
