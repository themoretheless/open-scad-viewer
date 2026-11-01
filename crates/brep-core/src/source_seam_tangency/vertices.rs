//! Regular material vertex graph stars from complete original-source evidence.
//! Face jets, material-side exact seams and angular degree retain ONE source body.
mod corner;
mod fan;
mod material;
use super::{Carrier, Class, material_orientation::OrientedCollection};
use crate::{
    source_vertex_links::Node,
    source_volume::{Body, regularity::smooth::SmoothFaces},
};
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;

trait FaceEvidence<E: Carrier> {
    fn body(&self) -> &Body<E>;
    fn order(&self) -> usize;
}
impl<E: Carrier> FaceEvidence<E> for SmoothFaces<'_, '_, E> {
    fn body(&self) -> &Body<E> {
        self.regular().body()
    }
    fn order(&self) -> usize {
        self.order()
    }
}
trait EdgeEvidence<E: Carrier> {
    fn body(&self) -> &Body<E>;
    fn classes(&self) -> &[Class];
}
impl<E: Carrier> EdgeEvidence<E> for OrientedCollection<'_, '_, '_, E> {
    fn body(&self) -> &Body<E> {
        self.exact().original().body()
    }
    fn classes(&self) -> &[Class] {
        self.exact().original().classes()
    }
}
pub struct Neighborhood {
    vertex: usize,
    point: [f64; 3],
    axis: usize,
    order: usize,
    collars: Vec<(usize, [[f64; 2]; 2])>,
}
impl Neighborhood {
    pub fn vertex(&self) -> usize {
        self.vertex
    }
    pub fn point(&self) -> [f64; 3] {
        self.point
    }
    pub fn dropped_axis(&self) -> usize {
        self.axis
    }
    pub fn order(&self) -> usize {
        self.order
    }
    /// Original parameter collars. A regular physical graph neighborhood
    /// exists; no specific metric radius is asserted by this certificate.
    pub fn collars(&self) -> &[(usize, [[f64; 2]; 2])] {
        &self.collars
    }
}
pub enum Vertex {
    C0 { vertex: usize },
    Smooth(Neighborhood),
}
pub struct Atlas<'a, E: Carrier> {
    faces: &'a dyn FaceEvidence<E>,
    edges: &'a dyn EdgeEvidence<E>,
    vertices: Vec<Vertex>,
}
impl<E: Carrier> Atlas<'_, E> {
    pub fn body(&self) -> &Body<E> {
        self.faces.body()
    }
    pub fn face_order(&self) -> usize {
        self.faces.order()
    }
    pub fn edge_classes(&self) -> &[Class] {
        self.edges.classes()
    }
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub steps: usize,
    pub normal_spans: usize,
    pub domain_cells: usize,
    pub exact_work: u64,
}
pub struct Report<'a, E: Carrier> {
    pub certificate: Option<Atlas<'a, E>>,
    pub steps: usize,
    pub normal_spans: usize,
    pub domain_cells: usize,
    pub exact_work: u64,
    pub uncertain_vertex: Option<usize>,
    pub reason: &'static str,
}
fn step<E: Carrier>(out: &mut Report<'_, E>, max: usize) -> bool {
    if out.steps == max {
        out.reason = "source-vertex-incidence-work-limit";
        false
    } else {
        out.steps += 1;
        true
    }
}

/// Complete C0/C1/C2 classification at ordinary original vertices. Fresh
/// natural-corner ownership, material collars, common projected rank and exact
/// single winding prove a physical Ck graph neighborhood for every smooth
/// candidate. Explicit C0 stays C0; unproved or exhausted requests yield no
/// atlas. Non-natural corners and pole neighborhoods require separate proofs.
pub fn qualify<'a, E: Carrier + 'a>(
    faces: &'a SmoothFaces<'_, '_, E>,
    oriented: &'a OrientedCollection<'_, '_, '_, E>,
    axes: &[usize],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a, E>> {
    let body = faces.regular().body();
    let original = oriented.exact().original();
    let links = &body.geometry().topology().links;
    if !std::ptr::eq(body, original.body())
        || axes.len() != links.len()
        || axes.iter().any(|&a| a > 2)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || limits.steps > 1000000
        || limits.normal_spans > 100000
        || limits.domain_cells > 100000
        || limits.exact_work > 100000000
    {
        return Err(Error::new(
            "BREP_SOURCE_VERTEX_GRAPH",
            "Use the same original body and bounded complete vertex proposals",
        ));
    }
    let shell = body.geometry().shell();
    let mut out = Report {
        certificate: None,
        steps: 0,
        normal_spans: 0,
        domain_cells: 0,
        exact_work: 0,
        uncertain_vertex: None,
        reason: "source-vertex-graph-unproven",
    };
    if !shell.poles().is_empty() || links.is_empty() {
        return Ok(out);
    }
    let mut orders = BTreeMap::new();
    for link in links {
        out.uncertain_vertex = Some(link.vertex);
        if !step(&mut out, limits.steps) {
            return Ok(out);
        }
        let mut order = faces.order();
        for n in &link.cycle {
            if !step(&mut out, limits.steps) {
                return Ok(out);
            }
            order = order.min(match original.classes()[n.edge] {
                Class::C0 => 0,
                Class::TangentPlanes => 1,
                Class::Curvature { .. } => 2,
            });
        }
        if orders.insert(link.vertex, order).is_some() {
            return Ok(out);
        }
    }
    let mut uses = BTreeMap::new();
    for (edge, pair) in shell.uses().iter().enumerate() {
        if !step(&mut out, limits.steps) {
            return Ok(out);
        }
        for (slot, a) in pair.iter().enumerate() {
            uses.insert((a.face, a.wire, a.edge), (edge, slot));
        }
    }
    let mut corners = BTreeMap::<usize, Vec<corner::Corner<'_>>>::new();
    for (fi, wires) in shell.faces().iter().enumerate() {
        let source = wires[0].edges()[0].surface();
        for (wi, wire) in wires.iter().enumerate() {
            for ei in 0..wire.edges().len() {
                if !step(&mut out, limits.steps) {
                    return Ok(out);
                }
                let vertex = shell.vertices()[fi][wi][ei][0];
                let Some(&order) = orders.get(&vertex) else {
                    return Ok(out);
                };
                if order == 0 {
                    continue;
                }
                out.uncertain_vertex = Some(vertex);
                let prev = (ei + wire.edges().len() - 1) % wire.edges().len();
                let Some(&(next, ns)) = uses.get(&(fi, wi, ei)) else {
                    return Ok(out);
                };
                let Some(&(prior, ps)) = uses.get(&(fi, wi, prev)) else {
                    return Ok(out);
                };
                let nodes = [
                    Node {
                        edge: next,
                        end: ns,
                    },
                    Node {
                        edge: prior,
                        end: 1 - ps,
                    },
                ];
                let Some(c) = corner::prepare(
                    fi,
                    wi,
                    [ei, prev],
                    source,
                    &wire.edges()[ei],
                    &wire.edges()[prev],
                    nodes,
                ) else {
                    out.reason = "source-natural-corner-unproven";
                    return Ok(out);
                };
                corners.entry(vertex).or_default().push(c);
            }
        }
    }
    let mut vertices = Vec::new();
    for (vi, link) in links.iter().enumerate() {
        out.uncertain_vertex = Some(link.vertex);
        if !step(&mut out, limits.steps) {
            return Ok(out);
        }
        let order = orders[&link.vertex];
        if order == 0 {
            vertices.push(Vertex::C0 {
                vertex: link.vertex,
            });
            continue;
        }
        let Some(cs) = corners.get(&link.vertex) else {
            return Ok(out);
        };
        let mut collars = Vec::new();
        let mut normal_sign = None;
        for c in cs {
            if !step(&mut out, limits.steps) {
                return Ok(out);
            }
            let region = &shell.regions().unwrap()[c.face];
            let m = material::certify(
                c,
                &shell.faces()[c.face],
                region,
                tolerance_uv,
                limits.steps - out.steps,
                limits.domain_cells - out.domain_cells,
            );
            out.steps += m.steps;
            out.domain_cells += m.domain_cells;
            let Some(mut rect) = m.rectangle else {
                out.reason = "source-vertex-material-collar-unproven";
                return Ok(out);
            };
            loop {
                let remaining = limits.normal_spans - out.normal_spans;
                if remaining == 0 {
                    out.reason = "source-vertex-normal-work-limit";
                    return Ok(out);
                }
                let mut direction = [0.; 3];
                direction[axes[vi]] = 1.;
                let r = match nurbs_core::surface_injectivity::normal_direction_bounds_detailed(
                    c.source, rect, direction, remaining,
                ) {
                    Ok(r) => r,
                    Err(_) => {
                        out.normal_spans = limits.normal_spans;
                        return Ok(out);
                    }
                };
                out.normal_spans += r.spans;
                let Some(mut b) = r.bounds else {
                    return Ok(out);
                };
                if region.chart_winding() == -1 {
                    b = [-b[1], -b[0]];
                }
                let sign = if b[0] > 0. {
                    Some(true)
                } else if b[1] < 0. {
                    Some(false)
                } else {
                    None
                };
                if let Some(sign) = sign {
                    if normal_sign.is_some_and(|s| s != sign) {
                        return Ok(out);
                    }
                    normal_sign = Some(sign);
                    collars.push((c.face, rect));
                    break;
                }
                if r.spans == 0 {
                    return Ok(out);
                }
                for k in 0..2 {
                    let mid = rect[k][0] * 0.5 + rect[k][1] * 0.5;
                    if !(rect[k][0] < mid && mid < rect[k][1]) {
                        return Ok(out);
                    }
                    if c.uv[k] == rect[k][0] {
                        rect[k][1] = mid;
                    } else if c.uv[k] == rect[k][1] {
                        rect[k][0] = mid;
                    } else {
                        return Ok(out);
                    }
                }
            }
        }
        let f = fan::certify(
            cs,
            &link.cycle,
            axes[vi],
            limits.exact_work - out.exact_work,
        );
        out.exact_work += f.exact_work;
        if !f.proven {
            out.reason = "source-vertex-single-original-fan-unproven";
            return Ok(out);
        }
        vertices.push(Vertex::Smooth(Neighborhood {
            vertex: link.vertex,
            point: cs[0].point,
            axis: axes[vi],
            order,
            collars,
        }));
    }
    out.certificate = Some(Atlas {
        faces,
        edges: oriented,
        vertices,
    });
    out.uncertain_vertex = None;
    out.reason = "source-natural-vertex-graph-stars-qualified";
    Ok(out)
}
