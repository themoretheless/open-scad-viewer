//! Original implicit sphere -> globally embedded, oriented and smooth body.
//! Exact membership is necessary but never substitutes for whole-boundary
//! embedding, complete pole quotients, vertex fans, nesting or orientation.
use crate::{Error, Model, Result};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy)]
pub struct Limits {
    pub volume: crate::volume_validity::Limits,
    pub linear_cells: usize,
    pub regularity_cells: usize,
    pub implicit_cells: usize,
    pub implicit_exact_work: u64,
}
pub struct Report {
    pub original_center: [f64; 3],
    pub original_radius: f64,
    pub certified: bool,
    pub source_boundary_exact: bool,
    pub boundary_error_upper: Option<f64>,
    pub source_topology_preserved: bool,
    pub source_g1: bool,
    pub source_g2: bool,
    pub implicit_exact_work: u64,
    pub implicit_cells: usize,
    pub supports: Vec<nurbs_core::implicit_quadric::Report>,
    pub regularity: Option<crate::boundary_embedding::regularity::Report>,
    pub volume: Option<crate::volume_validity::Report>,
    pub collapsed_edges: Vec<usize>,
    pub quotient_vertex_links_complete: bool,
    pub reason: Option<&'static str>,
}
pub struct SphereBody {
    model: Model,
    report: Report,
}
impl SphereBody {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub enum Attempt {
    Certified(Box<SphereBody>),
    Refused(Box<Report>),
}
fn refuse(mut r: Report, reason: &'static str) -> Attempt {
    r.reason = Some(reason);
    Attempt::Refused(Box::new(r))
}
// Contract only authored, whole, constant pole boundaries. Merely labelling an
// edge degenerate cannot delete it from a physical vertex link. Every use must
// independently own the same exact surface pole and point-valued world edge.
fn quotient_links(model: &Model) -> Option<Vec<usize>> {
    let mut collapsed = BTreeSet::new();
    let mut uses = vec![0; model.edges.len()];
    for (face, f) in model.faces.iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {
                uses[c.edge] += 1;
                if model.edges[c.edge].degenerate {
                    let (_, _, edge, _) =
                        crate::face_injectivity::declared_pole_chart(model, face)?;
                    if edge != c.edge {
                        return None;
                    }
                    collapsed.insert(edge);
                }
            }
        }
    }
    for (edge, e) in model.edges.iter().enumerate() {
        if e.degenerate {
            if !collapsed.contains(&edge) || uses[edge] == 0 {
                return None;
            }
        } else if uses[edge] != 2 {
            return None;
        }
    }
    type Node = (usize, usize);
    let mut links = vec![BTreeMap::<Node, Vec<Node>>::new(); model.vertices.len()];
    for f in &model.faces {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            let physical = model.loops[wire]
                .coedges
                .iter()
                .filter(|c| !collapsed.contains(&c.edge))
                .collect::<Vec<_>>();
            if physical.len() < 3 {
                return None;
            }
            for i in 0..physical.len() {
                let a = physical[i];
                let b = physical[(i + 1) % physical.len()];
                let ae = usize::from(!a.reversed);
                let bs = usize::from(b.reversed);
                let vertex = model.edges[a.edge].vertices[ae];
                if vertex != model.edges[b.edge].vertices[bs] {
                    return None;
                }
                let x = (a.edge, ae);
                let y = (b.edge, bs);
                links[vertex].entry(x).or_default().push(y);
                links[vertex].entry(y).or_default().push(x);
            }
        }
    }
    for graph in links {
        if graph.is_empty() || graph.values().any(|v| v.len() != 2) {
            return None;
        }
        let mut visited = BTreeSet::new();
        let mut pending = vec![*graph.first_key_value()?.0];
        while let Some(n) = pending.pop() {
            if visited.insert(n) {
                pending.extend(graph[&n].iter().copied());
            }
        }
        if visited.len() != graph.len() {
            return None;
        }
    }
    Some(collapsed.into_iter().collect())
}
fn validate_inputs(
    model: &Model,
    center: [f64; 3],
    radius: f64,
    tolerance_uv: f64,
    l: Limits,
) -> Result<()> {
    model.validate()?;
    if model.faces.is_empty()
        || model.faces.len() > 1024
        || l.implicit_exact_work > 16000000
        || l.implicit_cells > 100000
        || l.regularity_cells > 100000
        || l.linear_cells > 100000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(Error::new(
            "BREP_IMPLICIT_SOURCE_INVALID",
            "Invalid implicit source limits",
        ));
    }
    let v = l.volume;
    let b = v.boundary;
    let c = b.contacts;
    if !(1..=1000000).contains(&v.orientation_cells)
        || !(1..=8000000).contains(&v.orientation_domain_cells)
        || !(1..=100000).contains(&v.orientation_spans)
        || !(1..=100000).contains(&v.nesting_pairs)
        || !(2..=1000000).contains(&v.nesting_cells)
        || !(2..=8000000).contains(&v.nesting_domain_cells)
        || b.exact_work > 1000000
        || !(1..=100000).contains(&b.trim_pairs)
        || !(1..=100000).contains(&b.trim_cells)
        || !(1..=1000000).contains(&b.trim_domain_cells)
        || !(1..=100000).contains(&b.spans)
        || !(1..=100000).contains(&c.pairs)
        || !(1..=1000000).contains(&c.cells)
        || !(1..=8000000).contains(&c.domain_cells)
        || !(1..=100000).contains(&c.cells_per_pair)
        || !(1..=1000000).contains(&c.domain_cells_per_pair)
    {
        return Err(Error::new(
            "BREP_IMPLICIT_SOURCE_INVALID",
            "Invalid original sphere global proof limits",
        ));
    }
    // Validate ALL represented faces and raw equation inputs before any budget
    // refusal. These zero-work attempts provide no geometry/proof credit.
    for f in &model.faces {
        nurbs_core::implicit_quadric::inspect_sphere(&f.surface, center, radius, 0, 0)?;
    }
    Ok(())
}
fn empty_report(center: [f64; 3], radius: f64) -> Report {
    Report {
        original_center: center,
        original_radius: radius,
        certified: false,
        source_boundary_exact: false,
        boundary_error_upper: None,
        source_topology_preserved: false,
        source_g1: false,
        source_g2: false,
        implicit_exact_work: 0,
        implicit_cells: 0,
        supports: Vec::new(),
        regularity: None,
        volume: None,
        collapsed_edges: Vec::new(),
        quotient_vertex_links_complete: false,
        reason: None,
    }
}
/// Construct from literal source radius/center using the native exact charts.
/// Translation uses error-free sums; an unrepresentable exact placement refuses
/// rather than replacing the original sphere by rounded control points.
pub fn construct_sphere(
    center: [f64; 3],
    radius: f64,
    tolerance_uv: f64,
    l: Limits,
) -> Result<Attempt> {
    let mut model = crate::analytic::sphere_exact(radius)?;
    validate_inputs(&model, center, radius, tolerance_uv, l)?;
    let translate = |point: &mut [f64]| -> Option<()> {
        for k in 0..3 {
            if center[k] == 0. {
                continue;
            }
            let a = point[k];
            let b = center[k];
            let sum = a + b;
            let bv = sum - a;
            let av = sum - bv;
            let error = (a - av) + (b - bv);
            if !sum.is_finite() || error != 0. {
                return None;
            }
            point[k] = sum;
        }
        Some(())
    };
    let exact = (|| -> Option<()> {
        for v in &mut model.vertices {
            translate(&mut v.point)?;
        }
        for e in &mut model.edges {
            for p in &mut e.curve.control_points {
                translate(p)?;
            }
        }
        for f in &mut model.faces {
            for row in &mut f.surface.control_points {
                for p in row {
                    translate(p)?;
                }
            }
        }
        Some(())
    })()
    .is_some();
    if !exact {
        return Ok(refuse(
            empty_report(center, radius),
            "original-sphere-exact-translation-unproved",
        ));
    }
    verify_sphere(&model, center, radius, tolerance_uv, l)
}
/// Verify the actual model against the ORIGINAL exact sphere equation. A closed
/// embedded connected boundary contained in that regular connected level set is
/// the whole sphere (invariance of domain + compactness), not a selected subset.
/// Outward native volume ownership then determines its bounded ball. Every face,
/// including parameter poles, has the same analytic implicit tangent/curvature.
/// No identity report, allowed contacts, cap list or caller success flag enters.
/// The supplied model is never reconstructed, repaired or reoriented.
pub fn verify_sphere(
    model: &Model,
    center: [f64; 3],
    radius: f64,
    tolerance_uv: f64,
    l: Limits,
) -> Result<Attempt> {
    validate_inputs(model, center, radius, tolerance_uv, l)?;
    let mut r = empty_report(center, radius);
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.bodies[0].outer_shell != 0
        || !model.bodies[0].inner_shells.is_empty()
        || !model.shells[0].closed
    {
        return Ok(refuse(r, "original-sphere-single-shell-ownership-unproved"));
    }
    let mut owned = vec![false; model.faces.len()];
    for u in &model.shells[0].faces {
        if owned[u.face] {
            return Ok(refuse(r, "original-sphere-face-cover-unproved"));
        }
        owned[u.face] = true;
    }
    if owned.iter().any(|v| !*v) {
        return Ok(refuse(r, "original-sphere-face-cover-unproved"));
    }
    for f in &model.faces {
        let proof = nurbs_core::implicit_quadric::inspect_sphere(
            &f.surface,
            center,
            radius,
            l.implicit_exact_work - r.implicit_exact_work,
            l.implicit_cells - r.implicit_cells,
        )?;
        r.implicit_exact_work += proof.exact_work;
        r.implicit_cells += proof.cells;
        let proven = proof.regular_implicit_support;
        r.supports.push(proof);
        if !proven {
            return Ok(refuse(r, "original-sphere-implicit-support-unproved"));
        }
    }
    let Some(edges) = quotient_links(model) else {
        return Ok(refuse(r, "complete-pole-quotient-vertex-links-unproved"));
    };
    r.collapsed_edges = edges;
    r.quotient_vertex_links_complete = true;
    let regularity = crate::boundary_embedding::regularity::inspect(model, l.regularity_cells)?;
    let proven = regularity.all_faces_regular_modulo_poles;
    r.regularity = Some(regularity);
    if !proven {
        return Ok(refuse(
            r,
            "complete-boundary-pole-quotient-regularity-unproved",
        ));
    }
    // Smooth implicit support alone cannot rule out folded/overlapping patches.
    // The SAME call must prove the full actual surface complex is embedded,
    // closed, nested and outward, before any original boundary/Gk flag is set.
    let volume =
        crate::volume_validity::inspect_with_linear(model, tolerance_uv, l.volume, l.linear_cells)?;
    let proven = volume.proven;
    r.volume = Some(volume);
    if !proven {
        return Ok(refuse(r, "global-original-sphere-boundary-volume-unproved"));
    }
    r.certified = true;
    r.source_boundary_exact = true;
    r.boundary_error_upper = Some(0.);
    r.source_topology_preserved = true;
    r.source_g1 = true;
    r.source_g2 = true;
    Ok(Attempt::Certified(Box::new(SphereBody {
        model: model.clone(),
        report: r,
    })))
}
#[cfg(test)]
mod tests;
