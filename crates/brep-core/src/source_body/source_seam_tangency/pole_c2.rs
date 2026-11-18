//! C2 world graph disks from full exact seam coverage and owned pole jets.
use super::{Boundary, Carrier, Class, PoleStrata};
use nurbs_core::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};
/// A local owned source-star graph, not every body portion over this projection.
pub struct C2PoleNeighborhood {
    vertex: usize,
    axis: usize,
    point: [f64; 3],
    radius: f64,
    faces: Vec<usize>,
    seams: Vec<usize>,
    jet_bounds: [[f64; 2]; 5],
}
impl C2PoleNeighborhood {
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
    /// gx,gy,Hxx,Hxy,Hyy at the pole in cyclic world graph coordinates.
    /// These are diagnostic enclosures; exact equality comes from owned jets.
    pub fn coefficient_bounds(&self) -> [[f64; 2]; 5] {
        self.jet_bounds
    }
}
/// Cannot be assembled from standalone reports or caller smoothness flags.
/// Borrowing the full private boundary retains same-body geometry, topology,
/// exact meridian identities and original common pole jets for every disk.
pub struct C2PoleAtlas<'b, 's, E: Carrier> {
    boundary: &'b Boundary<'s, E>,
    neighborhoods: Vec<C2PoleNeighborhood>,
}
impl<'b, 's, E: Carrier> C2PoleAtlas<'b, 's, E> {
    pub fn boundary(&self) -> &'b Boundary<'s, E> {
        self.boundary
    }
    pub fn neighborhoods(&self) -> &[C2PoleNeighborhood] {
        &self.neighborhoods
    }
}
pub struct C2PoleReport<'b, 's, E: Carrier> {
    pub certificate: Option<C2PoleAtlas<'b, 's, E>>,
    pub steps: usize,
    pub completed_neighborhoods: usize,
    pub uncertain_vertex: Option<usize>,
    pub uncertain_edge: Option<usize>,
    pub reason: &'static str,
}
fn step<'b, 's, E: Carrier>(r: &mut C2PoleReport<'b, 's, E>, max: usize) -> bool {
    if r.steps == max {
        r.reason = "pole-c2-incidence-work-limit";
        false
    } else {
        r.steps += 1;
        true
    }
}
/// Derive C2 on EVERY original pole star's reported open projected disk.
/// The private graph atlas proves one sheet and complete native vertex-link
/// coverage; exact pole jets give uniform facewise Hessian limits. Every edge
/// of that owned link must additionally retain an exact full-carrier G2 token.
/// C0, tolerance-only, incomplete and exhausted requests yield no partial atlas.
/// Face graph derivatives glue exactly on the punctured disk, then their common
/// uniform gradient/Hessian limits remove the pole singularity geometrically.
/// No regular original polar parameterization or whole-body C2 is claimed.
pub fn qualify_c2_pole_neighborhoods<'b, 's, E: Carrier>(
    boundary: &'b Boundary<'s, E>,
    max_steps: usize,
) -> Result<C2PoleReport<'b, 's, E>> {
    if !(1..=1000000).contains(&max_steps) {
        return Err(Error::new(
            "BREP_SOURCE_POLE_C2",
            "Choose bounded native pole incidence work",
        ));
    }
    let mut out = C2PoleReport {
        certificate: None,
        steps: 0,
        completed_neighborhoods: 0,
        uncertain_vertex: None,
        uncertain_edge: None,
        reason: "pole-c2-curvature-atlas-required",
    };
    let PoleStrata::FacewiseCurvature(curvature) = boundary.poles() else {
        return Ok(out);
    };
    let body = boundary.body();
    let ordinary = boundary.ordinary();
    if !std::ptr::eq(ordinary.body(), body)
        || !std::ptr::eq(
            curvature.graphs().charts().transverse().atlas().body(),
            body,
        )
    {
        out.reason = "pole-c2-same-body-ownership-unproven";
        return Ok(out);
    }
    let stars = curvature.graphs().stars();
    if stars.is_empty() {
        out.reason = "pole-c2-no-owned-stars";
        return Ok(out);
    }
    let mut vertices = BTreeSet::new();
    for star in stars {
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        if !vertices.insert(star.vertex()) {
            out.reason = "pole-c2-duplicate-star";
            return Ok(out);
        }
    }
    let mut links = BTreeMap::new();
    for link in &body.geometry().topology().links {
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        if vertices.contains(&link.vertex) && links.insert(link.vertex, link).is_some() {
            out.reason = "pole-c2-owned-link-unproven";
            return Ok(out);
        }
    }
    let mut exact = BTreeSet::new();
    for (index, seam) in ordinary.polar_curvature_seams() {
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        if seam.is_exact()
            && seam.normal_upper() == 0.
            && seam.curvature_upper() == 0.
            && matches!(
                ordinary.classes().get(*index),
                Some(Class::Curvature { .. })
            )
        {
            if !exact.insert(*index) {
                out.reason = "pole-c2-duplicate-exact-carrier";
                return Ok(out);
            }
        }
    }
    let shell = body.geometry().shell();
    let mut jets = BTreeMap::new();
    for (address, jet) in curvature.jets() {
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        let vertex = shell.vertices()[address.face][address.wire][address.edge][0];
        jets.entry(vertex).or_insert(jet);
    }
    let mut neighborhoods = Vec::new();
    for star in stars {
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        out.uncertain_vertex = Some(star.vertex());
        let (Some(link), Some(jet)) = (links.get(&star.vertex()), jets.get(&star.vertex())) else {
            out.reason = "pole-c2-owned-link-or-jet-unproven";
            return Ok(out);
        };
        if jet.point() != star.point()
            || jet.dropped_axis() != star.dropped_axis()
            || !star.projected_radius_lower().is_finite()
            || star.projected_radius_lower() <= 0.
        {
            out.reason = "pole-c2-original-graph-jet-unproven";
            return Ok(out);
        }
        let mut faces = BTreeSet::new();
        for &face in star.faces() {
            if !step(&mut out, max_steps) {
                return Ok(out);
            }
            faces.insert(face);
        }
        let mut seams = BTreeSet::new();
        for node in &link.cycle {
            if !step(&mut out, max_steps) {
                return Ok(out);
            }
            seams.insert(node.edge);
        }
        // The graph factory has already proved this complete link equals its
        // paired natural meridians, including repeated face/corner incidence.
        // Check its unchanged source ownership defensively before gluing jets.
        if faces.is_empty() || seams.len() != faces.len() {
            out.reason = "pole-c2-complete-natural-fan-unproven";
            return Ok(out);
        }
        for &edge in &seams {
            if !step(&mut out, max_steps) {
                return Ok(out);
            }
            out.uncertain_edge = Some(edge);
            let Some(uses) = shell.uses().get(edge) else {
                out.reason = "pole-c2-meridian-ownership-unproven";
                return Ok(out);
            };
            if uses.iter().any(|a| {
                !faces.contains(&a.face)
                    || !shell.vertices()[a.face][a.wire][a.edge].contains(&star.vertex())
            }) {
                out.reason = "pole-c2-meridian-ownership-unproven";
                return Ok(out);
            }
            if !exact.contains(&edge) {
                out.reason = "pole-c2-exact-meridian-unproven";
                return Ok(out);
            }
        }
        neighborhoods.push(C2PoleNeighborhood {
            vertex: star.vertex(),
            axis: star.dropped_axis(),
            point: star.point(),
            radius: star.projected_radius_lower(),
            faces: faces.into_iter().collect(),
            seams: seams.into_iter().collect(),
            jet_bounds: jet.coefficient_bounds(),
        });
        out.completed_neighborhoods += 1;
        out.uncertain_edge = None;
    }
    out.certificate = Some(C2PoleAtlas {
        boundary,
        neighborhoods,
    });
    out.uncertain_vertex = None;
    out.reason = "original-owned-pole-c2-graph-neighborhoods-qualified";
    Ok(out)
}
