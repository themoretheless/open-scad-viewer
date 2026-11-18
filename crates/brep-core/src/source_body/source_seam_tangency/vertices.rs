//! Regular material vertex graph stars from complete original-source evidence.
//! Face jets, material-side exact seams and angular degree retain ONE source body.
mod corner;
mod fan;
pub mod material_boundary;
mod material;
mod poles;
use super::{Carrier, Class, material_orientation::OrientedCollection};
use crate::source_volume::regularity::stratified::PoleEvidence;
use crate::{
    source_vertex_links::{Node, VertexLink},
    source_volume::{Body, regularity::smooth::SmoothFaces},
};
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;

trait FaceEvidence<E: Carrier> {
    fn body(&self) -> &Body<E>;
    fn order(&self) -> usize;
    fn c1_poles(&self) -> &[super::C1PoleNeighborhood] {
        &[]
    }
    fn poles(&self) -> Option<&[super::C2PoleNeighborhood]> {
        None
    }
}
impl<E: Carrier> FaceEvidence<E> for SmoothFaces<'_, '_, E> {
    fn body(&self) -> &Body<E> {
        self.regular().body()
    }
    fn order(&self) -> usize {
        self.order()
    }
}
impl<'b, 's, E: Carrier, P: PoleEvidence<'b, 's, E>> FaceEvidence<E>
    for crate::source_volume::regularity::smooth::StratifiedSmoothFaces<'_, '_, 'b, 's, E, P>
{
    fn body(&self) -> &Body<E> {
        self.body()
    }
    fn order(&self) -> usize {
        self.order()
    }
    fn poles(&self) -> Option<&[super::C2PoleNeighborhood]> {
        Some(self.regular().poles().c2_neighborhoods())
    }
    fn c1_poles(&self) -> &[super::C1PoleNeighborhood] {
        self.regular().poles().c1_neighborhoods()
    }
}
trait EdgeEvidence<E: Carrier> {
    fn classes(&self) -> &[Class];
}
impl<E: Carrier> EdgeEvidence<E> for OrientedCollection<'_, '_, '_, E> {
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
    C0 {
        vertex: usize,
    },
    Smooth(Neighborhood),
    /// Physical pole disk, retained by this atlas through the same source body.
    Pole {
        vertex: usize,
        order: usize,
    },
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
    pub fn pole_neighborhoods(&self) -> &[super::C2PoleNeighborhood] {
        self.faces.poles().unwrap_or(&[])
    }
    pub fn c1_pole_neighborhoods(&self) -> &[super::C1PoleNeighborhood] {
        self.faces.c1_poles()
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
    qualify_impl(faces, oriented, axes, tolerance_uv, limits)
}
/// Complete ordinary AND pole coverage of the same original material body.
/// Pole disks retain their native C1/C2 authority; all ordinary stars are recomputed.
/// Stronger pole evidence never upgrades current declared classes. No partial
/// cover or foreign-source proof is accepted. Explicit C0 remains explicit C0.
pub fn qualify_stratified<'a, 'b, 's, E: Carrier + 'a, P: PoleEvidence<'b, 's, E>>(
    faces: &'a crate::source_volume::regularity::smooth::StratifiedSmoothFaces<
        '_,
        '_,
        'b,
        's,
        E,
        P,
    >,
    oriented: &'a OrientedCollection<'_, '_, '_, E>,
    axes: &[usize],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a, E>> {
    qualify_impl(faces, oriented, axes, tolerance_uv, limits)
}
const MAX_STEPS: usize = 1_000_000;
const MAX_NORMAL_SPANS: usize = 100_000;
const MAX_DOMAIN_CELLS: usize = 100_000;
const MAX_EXACT_WORK: u64 = 100_000_000;

fn qualify_impl<'a, E: Carrier + 'a>(
    faces: &'a dyn FaceEvidence<E>,
    oriented: &'a OrientedCollection<'_, '_, '_, E>,
    axes: &[usize],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a, E>> {
    let body = faces.body();
    let original = oriented.exact().original();
    let links = &body.geometry().topology().links;
    if !std::ptr::eq(body, original.body())
        || axes.len() != links.len()
        || axes.iter().any(|&a| a > 2)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || limits.steps > MAX_STEPS
        || limits.normal_spans > MAX_NORMAL_SPANS
        || limits.domain_cells > MAX_DOMAIN_CELLS
        || limits.exact_work > MAX_EXACT_WORK
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
    if (!shell.poles().is_empty() && faces.poles().is_none()) || links.is_empty() {
        return Ok(out);
    }
    let Some(polar) = poles::cover(
        body,
        faces.poles().unwrap_or(&[]),
        faces.c1_poles(),
        &mut out,
        limits.steps,
    ) else {
        return Ok(out);
    };
    let mut stars = Stars {
        body,
        axes,
        tolerance_uv,
        limits,
        out: &mut out,
    };
    let Some(vertices) = stars.classify(faces.order(), original.classes(), links, &polar) else {
        return Ok(out);
    };
    out.certificate = Some(Atlas {
        faces,
        edges: oriented,
        vertices,
    });
    out.uncertain_vertex = None;
    out.reason = "source-natural-vertex-graph-stars-qualified";
    Ok(out)
}

/// Edge use `(face, wire, edge)` to `(shell edge, slot)`.
type EdgeUses = BTreeMap<(usize, usize, usize), (usize, usize)>;

/// One vertex-star qualification run; every stage returns `None` when the
/// report already holds the refusal.
struct Stars<'r, 'a, 'b, E: Carrier> {
    body: &'b Body<E>,
    axes: &'b [usize],
    tolerance_uv: f64,
    limits: Limits,
    out: &'r mut Report<'a, E>,
}
impl<'b, E: Carrier> Stars<'_, '_, 'b, E> {
    fn step(&mut self) -> Option<()> {
        step(self.out, self.limits.steps).then_some(())
    }
    fn classify(
        &mut self,
        face_order: usize,
        classes: &[Class],
        links: &[VertexLink],
        polar: &BTreeMap<usize, poles::Disk<'_>>,
    ) -> Option<Vec<Vertex>> {
        let orders = self.orders(face_order, classes, links)?;
        let uses = self.uses()?;
        let corners = self.corners(&orders, polar, &uses)?;
        let mut vertices = Vec::new();
        for (vi, link) in links.iter().enumerate() {
            self.out.uncertain_vertex = Some(link.vertex);
            self.step()?;
            let order = *orders.get(&link.vertex)?;
            vertices.push(self.vertex(vi, link, order, polar, &corners)?);
        }
        Some(vertices)
    }
    fn orders(
        &mut self,
        face_order: usize,
        classes: &[Class],
        links: &[VertexLink],
    ) -> Option<BTreeMap<usize, usize>> {
        let mut orders = BTreeMap::new();
        for link in links {
            self.out.uncertain_vertex = Some(link.vertex);
            self.step()?;
            let mut order = face_order;
            for n in &link.cycle {
                self.step()?;
                order = order.min(match classes[n.edge] {
                    Class::C0 => 0,
                    Class::TangentPlanes => 1,
                    Class::Curvature { .. } => 2,
                });
            }
            if orders.insert(link.vertex, order).is_some() {
                return None;
            }
        }
        Some(orders)
    }
    fn uses(&mut self) -> Option<EdgeUses> {
        let mut uses = BTreeMap::new();
        for (edge, pair) in self.body.geometry().shell().uses().iter().enumerate() {
            self.step()?;
            for (slot, a) in pair.iter().enumerate() {
                uses.insert((a.face, a.wire, a.edge), (edge, slot));
            }
        }
        Some(uses)
    }
    fn corners(
        &mut self,
        orders: &BTreeMap<usize, usize>,
        polar: &BTreeMap<usize, poles::Disk<'_>>,
        uses: &EdgeUses,
    ) -> Option<BTreeMap<usize, Vec<corner::Corner<'b>>>> {
        let shell = self.body.geometry().shell();
        let mut corners = BTreeMap::<usize, Vec<corner::Corner<'b>>>::new();
        for (fi, wires) in shell.faces().iter().enumerate() {
            let source = wires[0].edges()[0].surface();
            for (wi, wire) in wires.iter().enumerate() {
                for ei in 0..wire.edges().len() {
                    self.step()?;
                    let vertex = shell.vertices()[fi][wi][ei][0];
                    let &order = orders.get(&vertex)?;
                    if order == 0 || polar.contains_key(&vertex) {
                        continue;
                    }
                    self.out.uncertain_vertex = Some(vertex);
                    let prev = (ei + wire.edges().len() - 1) % wire.edges().len();
                    let &(next, ns) = uses.get(&(fi, wi, ei))?;
                    let &(prior, ps) = uses.get(&(fi, wi, prev))?;
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
                        self.out.reason = "source-natural-corner-unproven";
                        return None;
                    };
                    corners.entry(vertex).or_default().push(c);
                }
            }
        }
        Some(corners)
    }
    fn vertex(
        &mut self,
        vi: usize,
        link: &VertexLink,
        order: usize,
        polar: &BTreeMap<usize, poles::Disk<'_>>,
        corners: &BTreeMap<usize, Vec<corner::Corner<'b>>>,
    ) -> Option<Vertex> {
        if order == 0 {
            return Some(Vertex::C0 {
                vertex: link.vertex,
            });
        }
        if let Some(n) = polar.get(&link.vertex) {
            if self.axes[vi] != n.dropped_axis() {
                self.out.reason = "source-pole-projected-axis-unproven";
                return None;
            }
            if order > n.order() {
                self.out.reason = "source-pole-requested-order-unproven";
                return None;
            }
            return Some(Vertex::Pole {
                vertex: link.vertex,
                order,
            });
        }
        let cs = corners.get(&link.vertex)?;
        let axis = self.axes[vi];
        let mut collars = Vec::new();
        let mut normal_sign = None;
        for c in cs {
            self.step()?;
            collars.push((c.face, self.collar(c, axis, &mut normal_sign)?));
        }
        let f = fan::certify(
            cs,
            &link.cycle,
            axis,
            self.limits.exact_work - self.out.exact_work,
        );
        self.out.exact_work += f.exact_work;
        if !f.proven {
            self.out.reason = "source-vertex-single-original-fan-unproven";
            return None;
        }
        Some(Vertex::Smooth(Neighborhood {
            vertex: link.vertex,
            point: cs[0].point,
            axis,
            order,
            collars,
        }))
    }
    /// Material collar of one corner, shrunk toward the corner until the
    /// projected normal has one sign shared by the whole star.
    fn collar(
        &mut self,
        c: &corner::Corner<'b>,
        axis: usize,
        normal_sign: &mut Option<bool>,
    ) -> Option<[[f64; 2]; 2]> {
        let shell = self.body.geometry().shell();
        let region = shell.regions()?.get(c.face)?;
        let m = material::certify(
            c,
            &shell.faces()[c.face],
            region,
            self.tolerance_uv,
            self.limits.steps - self.out.steps,
            self.limits.domain_cells - self.out.domain_cells,
        );
        self.out.steps += m.steps;
        self.out.domain_cells += m.domain_cells;
        let Some(mut rect) = m.rectangle else {
            self.out.reason = "source-vertex-material-collar-unproven";
            return None;
        };
        let mut direction = [0.; 3];
        direction[axis] = 1.;
        loop {
            let remaining = self.limits.normal_spans - self.out.normal_spans;
            if remaining == 0 {
                self.out.reason = "source-vertex-normal-work-limit";
                return None;
            }
            let Ok(r) = nurbs_core::surface_injectivity::normal_direction_bounds_detailed(
                c.source, rect, direction, remaining,
            ) else {
                self.out.normal_spans = self.limits.normal_spans;
                return None;
            };
            self.out.normal_spans += r.spans;
            let mut b = r.bounds?;
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
                    return None;
                }
                *normal_sign = Some(sign);
                return Some(rect);
            }
            if r.spans == 0 {
                return None;
            }
            shrink_toward_corner(&mut rect, c.uv)?;
        }
    }
}
/// Halves `rect` toward the corner parameter in both directions.
fn shrink_toward_corner(rect: &mut [[f64; 2]; 2], uv: [f64; 2]) -> Option<()> {
    for k in 0..2 {
        let mid = rect[k][0] * 0.5 + rect[k][1] * 0.5;
        if !(rect[k][0] < mid && mid < rect[k][1]) {
            return None;
        }
        if uv[k] == rect[k][0] {
            rect[k][1] = mid;
        } else if uv[k] == rect[k][1] {
            rect[k][0] = mid;
        } else {
            return None;
        }
    }
    Some(())
}
