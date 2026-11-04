//! Paired retained patches whose complete cross-source contacts are their
//! literal common root boundary. Outward material orientation remains separate.
use super::*;
pub struct QualifiedPairedRootBoundary {
    census: Arc<QualifiedInteriorSsCensus>,
    signatures: [Vec<bool>; 2],
    limits: [CensusArrangementLimits; 2],
    model: MixedBrep,
    genus: usize,
}
impl QualifiedPairedRootBoundary {
    pub fn model(&self) -> &MixedBrep {
        &self.model
    }
    pub fn genus(&self) -> usize {
        self.genus
    }
    pub fn boundary_embedded(&self) -> bool {
        true
    }
    pub fn material_admitted(&self) -> bool {
        false
    }
    pub fn signatures(&self) -> [&[bool]; 2] {
        [&self.signatures[0], &self.signatures[1]]
    }
    pub fn reverify(&self) -> Result<()> {
        self.census.reverify()?;
        let fresh = self.census.sew_paired_regions(
            self.signatures(),
            self.model.topology.tolerance_mm,
            self.limits,
        )?;
        if fresh.genus != self.genus {
            return Err(unproved("Paired root boundary topology changed"));
        }
        Ok(())
    }
}
fn insert_edge(t: &mut MixedTopology, edge: &ss::OriginalSsRootEdge) -> Result<usize> {
    for (id, old) in t.edges.iter().enumerate() {
        if let MixedEdge::Root(old) = &old.curve {
            if old.start().point().same_selector(edge.start().point())
                && old.end().point().same_selector(edge.end().point())
                && old.curve().same_definition(edge.curve())?
            {
                return Ok(id);
            }
        }
    }
    let mut vertices = [0; 2];
    for (i, port) in [edge.start(), edge.end()].into_iter().enumerate() {
        vertices[i] =
            if let Some(id) = t.vertices.iter().position(
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
    Ok(id)
}
impl QualifiedInteriorSsCensus {
    /// The sealed census owns ALL contacts of the two original supports. A
    /// retained arrangement region has no root component in its interior; its
    /// only contacts are its authored boundary loops. Matching those loops by
    /// exact graph/selectors sews two injective patches into an embedded closed
    /// orientable boundary. Rational rims/holes need separate stitching proof.
    pub fn sew_paired_regions(
        self: &Arc<Self>,
        signatures: [&[bool]; 2],
        tolerance_mm: f64,
        limits: [CensusArrangementLimits; 2],
    ) -> Result<QualifiedPairedRootBoundary> {
        if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
            return Err(unproved("Paired boundary needs positive finite tolerance"));
        }
        let arrangements = [
            self.build_face_arrangement(0, limits[0])?,
            self.build_face_arrangement(1, limits[1])?,
        ];
        let mut t = MixedTopology {
            vertices: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            faces: Vec::new(),
            shells: Vec::new(),
            bodies: Vec::new(),
            tolerance_mm,
        };
        let mut incidence: Vec<Vec<(usize, bool)>> = Vec::new();
        let mut hole_counts = [0; 2];
        for support in 0..2 {
            let source = arrangements[support].model().topology();
            let face = arrangements[support].face_for_signature(signatures[support])?;
            let face = &source.faces[face];
            hole_counts[support] = face.holes.len();
            let mut loops = Vec::new();
            for l in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                let mut coedges = Vec::new();
                for c in &source.loops[l].coedges {
                    let (MixedEdge::Root(edge), MixedPcurve::Root(pcurve)) =
                        (&source.edges[c.edge].curve, &c.pcurve)
                    else {
                        return Err(unproved(
                            "Paired boundary needs explicit rational/root rim stitching",
                        ));
                    };
                    if c.trim().is_some() || pcurve.support != support {
                        return Err(unproved("Foreign paired root projection"));
                    }
                    let id = insert_edge(&mut t, edge)?;
                    while incidence.len() <= id {
                        incidence.push(Vec::new());
                    }
                    incidence[id].push((support, c.reversed));
                    coedges.push(brep_topology::Coedge::new(
                        id,
                        c.reversed,
                        MixedPcurve::Root(crate::root_model::RootPcurve {
                            world_edge: id,
                            support,
                            reversed: c.reversed,
                        }),
                    ));
                }
                let id = t.loops.len();
                t.loops.push(brep_topology::Loop { coedges });
                loops.push(id);
            }
            t.faces.push(brep_topology::Face {
                surface: face.surface.clone(),
                outer: loops[0],
                holes: loops[1..].to_vec(),
            });
        }
        if hole_counts[0] != hole_counts[1] {
            return Err(unproved(
                "Paired patches have different boundary component counts",
            ));
        }
        let mut reverse = None;
        for uses in &incidence {
            if uses.len() != 2 || uses[0].0 != 0 || uses[1].0 != 1 {
                return Err(unproved(
                    "Paired regions have unmatched or repeated original world edges",
                ));
            }
            let needed = uses[0].1 == uses[1].1;
            if reverse.is_some_and(|r| r != needed) {
                return Err(unproved(
                    "Paired root boundary has inconsistent orientation",
                ));
            }
            reverse = Some(needed);
        }
        let reverse =
            reverse.ok_or_else(|| unproved("Paired boundary has no shared root edges"))?;
        t.shells.push(brep_topology::Shell {
            faces: vec![
                brep_topology::FaceUse {
                    face: 0,
                    reversed: false,
                },
                brep_topology::FaceUse {
                    face: 1,
                    reversed: reverse,
                },
            ],
            closed: true,
        });
        t.bodies.push(brep_topology::Body {
            outer_shell: 0,
            inner_shells: Vec::new(),
        });
        let model = MixedBrep::from_topology(t)?;
        Ok(QualifiedPairedRootBoundary {
            census: self.clone(),
            signatures: signatures.map(|s| s.to_vec()),
            limits,
            model,
            genus: hole_counts[0],
        })
    }
}

mod volume;
pub use volume::{RootBoundaryVolumeLimits,RootBoundaryVolumeReport};

mod material;
pub use material::{PairedRootSolidAdmission,QualifiedPairedRootSolid};
