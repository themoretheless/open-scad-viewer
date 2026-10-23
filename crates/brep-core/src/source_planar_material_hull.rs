//! Convex enclosure of an original planar material region by owned carriers.
//! A fresh planar chart embedding makes the bounded material image lie in
//! the convex hull of its boundary. Unused original carrier tails enlarge
//! the enclosure; they never hide material.
use crate::{
    source_contour_proposal::SourceRegion,
    source_shared_edge::SharedEdge,
    source_shell_incidence::{Address, Shell},
};
use nurbs_core::{Error, Result, surface_injectivity};
pub struct Certificate {
    region: SourceRegion,
    plane: [[f64; 3]; 3],
    chart: surface_injectivity::Report,
    edges: Vec<SharedEdge>,
    points: Vec<[f64; 3]>,
}
impl Certificate {
    pub fn region(&self) -> &SourceRegion {
        &self.region
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
    pub fn chart(&self) -> &surface_injectivity::Report {
        &self.chart
    }
    pub fn edges(&self) -> &[SharedEdge] {
        &self.edges
    }
    pub fn points(&self) -> &[[f64; 3]] {
        &self.points
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub reason: &'static str,
}
pub fn certify(shell: &Shell, face: usize, max_work: u64) -> Result<Report> {
    let regions = shell
        .regions()
        .ok_or_else(|| Error::new("BREP_SOURCE_MATERIAL_HULL", "Original regions required"))?;
    if face >= regions.len() || !(1..=100_000_000).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_MATERIAL_HULL",
            "Choose a source face and bounded work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        reason: "source-planar-material-hull-unproven",
    };
    let region = &regions[face];
    let s = region.loops()[0][0].surface();
    // This stage bounds embedding work to one original bilinear knot chart.
    if s.periodic_u
        || s.periodic_v
        || s.degree_u != 1
        || s.degree_v != 1
        || s.control_points.len() != 2
        || s.control_points[0].len() != 2
        || s.knots_u.len() != 4
        || s.knots_v.len() != 4
        || s.knots_u[0] != s.knots_u[1]
        || s.knots_u[2] != s.knots_u[3]
        || s.knots_v[0] != s.knots_v[1]
        || s.knots_v[2] != s.knots_v[3]
    {
        return Ok(out);
    }
    let Some(plane) = crate::source_allowed_contact::plane(s, &mut out.exact_work, max_work)?
    else {
        return Ok(out);
    };
    let chart = surface_injectivity::certify_contraction(s, 1)?;
    if !chart.proven {
        return Ok(out);
    }
    let mut edges = Vec::new();
    let mut points = Vec::new();
    for (wire, fragments) in region.loops().iter().enumerate() {
        for (edge, _) in fragments.iter().enumerate() {
            let address = Address { face, wire, edge };
            let Some((index, slot)) = shell
                .uses()
                .iter()
                .enumerate()
                .find_map(|(i, a)| a.iter().position(|a| *a == address).map(|s| (i, s)))
            else {
                return Ok(out);
            };
            let shared = &shell.edges()[index];
            // Only direct full-source composition authority is used here.
            // General mapped restrictions need independent range enclosures.
            if shared.ranges().is_some() || shared.world().weights.iter().any(|w| *w <= 0.) {
                return Ok(out);
            }
            let source = &shared.uses()[slot];
            let domain = source.curve().domain();
            let expected = if source.reversed() {
                [domain[1], domain[0]]
            } else {
                domain
            };
            if !(0..2).all(|end|matches!(source.endpoints()[end],crate::source_boundary_fragment::Endpoint::Parameter(t) if t==expected[end])) {return Ok(out);}
            for p in &shared.world().control_points {
                points.push(p.as_slice().try_into().map_err(|_| {
                    Error::new("BREP_SOURCE_MATERIAL_HULL", "Expected original XYZ carrier")
                })?);
            }
            edges.push(shared.clone());
        }
    }
    out.certificate = Some(Certificate {
        region: region.clone(),
        plane,
        chart,
        edges,
        points,
    });
    out.reason = "source-planar-material-boundary-hull-qualified";
    Ok(out)
}
