//! Exact finite-face subdivision candidates. Whole original SS curves are
//! placed inside the retained source domain; existing holes are assigned using
//! winding of entire original endpoint enclosures, never rounded UV guesses.
use super::*;
use num_traits::ToPrimitive;
use nurbs_core::{
    interval_eval::{self, Interval},
    trim_domain::Location,
};
#[derive(Clone, Copy, Debug)]
pub struct ClosedCutLimits {
    pub tolerance_uv: f64,
    pub placement_queries: usize,
    pub placement_domain_cells: usize,
    /// Shared across every placement query and every edge subdivision.
    pub placement_geometry: ss::OriginalSsCurveLimits,
    /// Explicit allowance per seed/hole boundary obligation (at most 65).
    pub per_boundary_winding: ss::OriginalSsWindingLimits,
}
impl Default for ClosedCutLimits {
    fn default() -> Self {
        Self {
            tolerance_uv: 1e-9,
            placement_queries: 4096,
            placement_domain_cells: 1000000,
            placement_geometry: Default::default(),
            per_boundary_winding: Default::default(),
        }
    }
}
pub struct ClosedCutCandidate {
    source: crate::closed_solid::QualifiedClosedSolid,
    face: usize,
    wire: ss::OriginalSsRootWire,
    support: usize,
    seed: [f64; 2],
    orientation: i32,
    limits: ClosedCutLimits,
    model: MixedBrep,
    inside: usize,
    inherited_holes: Vec<usize>,
    placement_queries: usize,
    domain_cells: usize,
}
impl ClosedCutCandidate {
    pub fn model(&self) -> &MixedBrep {
        &self.model
    }
    pub fn source_face(&self) -> usize {
        self.face
    }
    pub fn inside_face(&self) -> usize {
        self.inside
    }
    pub fn inside_inherited_holes(&self) -> &[usize] {
        &self.inherited_holes
    }
    pub fn placement_work(&self) -> [usize; 2] {
        [self.placement_queries, self.domain_cells]
    }
    pub fn material_classified(&self) -> bool {
        false
    }
    pub fn geometry_admitted(&self) -> bool {
        false
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let f = split_closed_contour_candidate(
            &self.source,
            self.face,
            &self.wire,
            self.support,
            self.seed,
            self.limits,
        )?;
        if f.orientation != self.orientation
            || f.inside != self.inside
            || f.inherited_holes != self.inherited_holes
            || f.placement_work() != self.placement_work()
        {
            return Err(unproved(
                "Finite original face subdivision evidence changed",
            ));
        }
        Ok(())
    }
}
fn charge(remaining: &mut ss::OriginalSsCurveLimits, b: &ss::OriginalSsCurveBounds) -> Result<()> {
    let subtract = |a: usize, b: usize| {
        a.checked_sub(b)
            .ok_or_else(|| unproved("Shared finite-face root query work exhausted"))
    };
    remaining.native_rectangles = subtract(remaining.native_rectangles, b.native_rectangles)?;
    remaining.interval_operations = subtract(remaining.interval_operations, b.interval_operations)?;
    remaining.projection.max_work = subtract(remaining.projection.max_work, b.projection_work)?;
    remaining.world_rectangles = subtract(remaining.world_rectangles, b.world_rectangles)?;
    Ok(())
}
fn clock(value: &num_rational::BigRational, low: bool) -> Result<f64> {
    let f = value
        .to_f64()
        .filter(|f| f.is_finite())
        .ok_or_else(|| unproved("Root cut clock overflow"))?;
    let q = num_rational::BigRational::from_float(f).unwrap();
    Ok(if low && q > *value {
        f.next_down()
    } else if !low && q < *value {
        f.next_up()
    } else {
        f
    })
}
fn append_loop(t: &mut MixedTopology, edges: &[usize], support: usize, reverse: bool) -> usize {
    let mut ids = edges.to_vec();
    if reverse {
        ids.reverse();
    }
    let id = t.loops.len();
    t.loops.push(brep_topology::Loop {
        coedges: ids
            .into_iter()
            .map(|edge| {
                brep_topology::Coedge::new(
                    edge,
                    reverse,
                    MixedPcurve::Root(crate::root_model::RootPcurve {
                        world_edge: edge,
                        support,
                        reversed: reverse,
                    }),
                )
            })
            .collect(),
    });
    id
}
pub fn split_closed_contour_candidate(
    source: &crate::closed_solid::QualifiedClosedSolid,
    face: usize,
    wire: &ss::OriginalSsRootWire,
    support: usize,
    inside_seed: [f64; 2],
    limits: ClosedCutLimits,
) -> Result<ClosedCutCandidate> {
    if !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || support >= 2
        || !(1..=65536).contains(&limits.placement_queries)
        || !(1..=8000000).contains(&limits.placement_domain_cells)
    {
        return Err(unproved("Invalid finite-face cut placement work"));
    }
    let original = source.model();
    let surface = &original
        .faces
        .get(face)
        .ok_or_else(|| unproved("Cut source face is absent"))?
        .surface;
    if original.faces[face].holes.len() > 64 {
        return Err(unproved("Finite-face cut hole allowance exhausted"));
    }
    let domain = crate::face_domain::FaceDomain::new(original, face, limits.tolerance_uv)?;
    let mut geometry = limits.placement_geometry;
    let mut queries = 0;
    let mut domain_cells = 0;
    for edge in wire.edges() {
        let s = edge.curve().source();
        let s = if support == 0 { s.first() } else { s.second() };
        if s != surface
            || [edge.start(), edge.end()].iter().any(|p| {
                p.phase() != [0; 4]
                    || matches!(
                        p.kind(),
                        ss::OriginalSsPortKind::CollapsedPole
                            | ss::OriginalSsPortKind::OriginalSourceNode
                    )
            })
        {
            return Err(unproved(
                "Finite planar cut needs exact source projections and resolved UV endpoint joins",
            ));
        }
        let d = edge.parameter_bounds();
        let natural = edge.curve().parameter_domain();
        let range = [
            clock(&d[0], true)?.max(natural[0]),
            clock(&d[1], false)?.min(natural[1]),
        ];
        let mut stack = vec![(range, 0usize)];
        while let Some((range, depth)) = stack.pop() {
            if queries == limits.placement_queries || domain_cells == limits.placement_domain_cells
            {
                return Err(unproved(
                    "Complete finite-face contour placement work exhausted",
                ));
            }
            queries += 1;
            let b = edge.curve().enclose(range, geometry)?;
            charge(&mut geometry, &b)?;
            let uv = [b.uv[2 * support], b.uv[2 * support + 1]];
            let classification = domain.classify(
                uv,
                (limits.placement_domain_cells - domain_cells).min(100000),
            )?;
            domain_cells += classification.cells;
            if classification.location == Location::Inside {
                continue;
            }
            if classification.location == Location::Outside {
                return Err(unproved(
                    "Original cut curve leaves the retained finite source face",
                ));
            }
            let middle = range[0] * 0.5 + range[1] * 0.5;
            if depth == 32 || !(range[0] < middle && middle < range[1]) {
                return Err(unproved("Original cut/rim relation remains unresolved"));
            }
            stack.push(([middle, range[1]], depth + 1));
            stack.push(([range[0], middle], depth + 1));
        }
    }
    let orientation =
        ss::qualify_original_ss_winding(wire, support, inside_seed, limits.per_boundary_winding)?
            .winding();
    if orientation != 1 && orientation != -1 {
        return Err(unproved(
            "Cut orientation needs a unit original winding witness",
        ));
    }
    let mut inside_holes = Vec::new();
    let mut outside_holes = Vec::new();
    for &hole in &original.faces[face].holes {
        let c = &original.loops[hole].coedges[0];
        let p = &c.pcurve;
        let low = c
            .trim()
            .map(|t| t.pcurve_parameter[0])
            .unwrap_or(p.domain()[0]);
        let point = interval_eval::evaluate_interval(p, Interval::point(low))?;
        if point.len() != 2 {
            return Err(unproved(
                "Original hole endpoint has no spatial UV projection",
            ));
        }
        let winding = wire
            .winding_box(
                support,
                [[point[0].lo, point[0].hi], [point[1].lo, point[1].hi]],
                limits.per_boundary_winding,
            )?
            .winding();
        if winding == orientation {
            inside_holes.push(hole);
        } else if winding == 0 {
            outside_holes.push(hole);
        } else {
            return Err(unproved("Original hole/cut nesting is unresolved"));
        }
    }
    let mut t = MixedBrep::from_closed_solid(source)?.topology;
    let mut ids = Vec::new();
    for edge in wire.edges() {
        let mut vertices = [0; 2];
        for (i, port) in [edge.start(), edge.end()].into_iter().enumerate() {
            vertices[i] = if let Some(id) = t.vertices.iter().position(
                |v| matches!(&v.point,MixedVertex::Root(p) if p.same_selector(port.point())),
            ) {
                id
            } else {
                let id = t.vertices.len();
                t.vertices.push(brep_topology::Vertex {
                    point: MixedVertex::Root(port.point().clone()),
                });
                id
            };
        }
        let id = t.edges.len();
        t.edges.push(brep_topology::Edge {
            degenerate: false,
            vertices,
            curve: MixedEdge::Root(edge.clone()),
        });
        ids.push(id);
    }
    let hole = append_loop(&mut t, &ids, support, orientation > 0);
    let outer = append_loop(&mut t, &ids, support, orientation < 0);
    t.faces[face].holes = outside_holes;
    t.faces[face].holes.push(hole);
    let inside = t.faces.len();
    t.faces.push(brep_topology::Face {
        surface: surface.clone(),
        outer,
        holes: inside_holes.clone(),
    });
    for shell in &mut t.shells {
        if let Some(use_) = shell.faces.iter().find(|u| u.face == face).cloned() {
            shell.faces.push(brep_topology::FaceUse {
                face: inside,
                reversed: use_.reversed,
            });
        }
    }
    let model = MixedBrep::from_topology(t)?;
    Ok(ClosedCutCandidate {
        source: source.clone(),
        face,
        wire: wire.clone(),
        support,
        seed: inside_seed,
        orientation,
        limits,
        model,
        inside,
        inherited_holes: inside_holes,
        placement_queries: queries,
        domain_cells,
    })
}

mod classification;
pub use classification::{CutRectangleLimits, CutRectangleReport};
mod material;
pub use material::{CutMaterialLimits, QualifiedCutMaterialRectangle};

mod jordan;
pub use jordan::{QualifiedPairedRootBoundary, QualifiedArrangementMaterialRectangle, CensusArrangementLimits, CensusFaceArrangement, CensusRectangleReport, QualifiedCensusMaterialRectangle, InteriorCensusAnchor, InteriorCensusLimits, QualifiedInteriorSsCensus, qualify_interior_ss_census,InteriorLoopLimits, QualifiedInteriorSsLoop, qualify_paired_interior_loop};
