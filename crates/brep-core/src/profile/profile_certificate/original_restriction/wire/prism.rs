//! Rational 3D shell geometry from exact source-preserving profile wires.
//! Closed incidence and continuous boundary agreement are proved. This does
//! not admit trim Jordan domains, embedding, nesting, material or a solid Body.
use super::*;
use brep_topology::{Coedge, Edge, Face, FaceUse, Loop, Shell, Vertex};
use num_traits::{FromPrimitive, Signed};
mod geometry;
pub mod display;
pub mod solid;
use geometry::Kind;
pub use geometry::{verify_geometry, Agreement, Curve3, Pcurve, Surface};
pub use solid::Solid;
pub type Topology = brep_topology::Model<Curve3, Surface, Pcurve, [Q; 3]>;
#[derive(Clone, Debug)]
pub struct ShellGeometry {
    sources: Vec<Wire>,
    height: [Q; 2],
    model: Topology,
    wall_sources: Vec<[usize; 3]>,
    agreement: Agreement,
}
fn point(h: &[Q; 3], z: &Q) -> [Q; 3] {
    [&h[0] / &h[2], &h[1] / &h[2], z.clone()]
}
fn lifted(net: &[[Q; 3]], z: &Q) -> Curve3 {
    Curve3 {
        net: net
            .iter()
            .map(|h| [h[0].clone(), h[1].clone(), z * &h[2], h[2].clone()])
            .collect(),
    }
}
fn line(a: [Q; 2], b: [Q; 2]) -> Pcurve {
    Pcurve {
        net: [a, b]
            .map(|p| [p[0].clone(), p[1].clone(), Q::one()])
            .to_vec(),
    }
}
fn add_loop(model: &mut Topology, coedges: Vec<Coedge<Pcurve>>) -> usize {
    let i = model.loops.len();
    model.loops.push(Loop { coedges });
    i
}
fn add_edge(model: &mut Topology, vertices: [usize; 2], curve: Curve3) -> usize {
    let i = model.edges.len();
    model.edges.push(Edge {
        vertices,
        curve,
        degenerate: false,
    });
    i
}
fn validate(model: &Topology) -> Result<()> {
    model.validate_topology_with_vertices(|p| {
        if p.iter()
            .any(|q| q.abs() > Q::from_integer(1_000_000.into()))
        {
            return Err(refused("Exact shell vertex exceeds coordinate bound"));
        }
        Ok(())
    })
}
const MAX_PROFILES: usize = 64;
const MAX_SPANS: usize = 254;
/// Profiles, heights and span count are within the exact extrusion budget,
/// and every profile span replays as a regular source.
fn verify_sources(sources: &[Wire], height: &[Q; 2]) -> Result<()> {
    let count = sources
        .iter()
        .flat_map(|w| &w.pieces)
        .map(Vec::len)
        .sum::<usize>();
    if sources.is_empty()
        || sources.len() > MAX_PROFILES
        || !(1..=MAX_SPANS).contains(&count)
        || height[0] >= height[1]
        || height
            .iter()
            .any(|q| q.abs() > Q::from_integer(1_000_000.into()))
    {
        return Err(refused(
            "Invalid exact extrusion profiles, heights or resource budget",
        ));
    }
    for w in sources {
        w.reverify()?;
        for span in w.pieces.iter().flatten() {
            smooth::regular(&span.net)?;
        }
    }
    Ok(())
}
/// Axis-aligned bounds of all profile control points, used as the cap chart.
fn cap_bounds(sources: &[Wire]) -> Result<[[Q; 2]; 2]> {
    let first = &sources[0].pieces[0][0].net[0];
    let p = [&first[0] / &first[2], &first[1] / &first[2]];
    let mut bounds = [p.clone(), p];
    for h in sources
        .iter()
        .flat_map(|w| &w.pieces)
        .flatten()
        .flat_map(|s| &s.net)
    {
        for a in 0..2 {
            let v = &h[a] / &h[2];
            bounds[0][a] = bounds[0][a].clone().min(v.clone());
            bounds[1][a] = bounds[1][a].clone().max(v);
        }
    }
    if (0..2).any(|a| bounds[0][a] >= bounds[1][a]) {
        return Err(refused("Degenerate exact cap chart"));
    }
    Ok(bounds)
}
/// The prism topology under construction: walls are added profile by
/// profile, then the two caps close the shell.
struct Prism<'a> {
    model: Topology,
    height: &'a [Q; 2],
    cap_loops: [Vec<usize>; 2],
    wall_sources: Vec<[usize; 3]>,
}
impl Prism<'_> {
    /// One wall face over a profile span, bounded by its two horizontal
    /// edges and the vertical edges at its ends.
    fn wall(&mut self, profile: &[[Q; 3]], horizontal: [usize; 2], vertical: [usize; 2]) {
        let lo = &self.height[0];
        let hi = &self.height[1];
        let outer = add_loop(
            &mut self.model,
            vec![
                Coedge::new(
                    horizontal[0],
                    false,
                    line([Q::zero(), lo.clone()], [Q::one(), lo.clone()]),
                ),
                Coedge::new(
                    vertical[1],
                    false,
                    line([Q::one(), lo.clone()], [Q::one(), hi.clone()]),
                ),
                Coedge::new(
                    horizontal[1],
                    true,
                    line([Q::one(), hi.clone()], [Q::zero(), hi.clone()]),
                ),
                Coedge::new(
                    vertical[0],
                    true,
                    line([Q::zero(), hi.clone()], [Q::zero(), lo.clone()]),
                ),
            ],
        );
        self.model.faces.push(Face {
            surface: Surface {
                kind: Kind::Wall {
                    profile: profile.to_vec(),
                    height: self.height.clone(),
                },
            },
            outer,
            holes: vec![],
        });
    }
    /// Vertices, edges, cap loops and wall faces of one closed profile.
    fn profile(&mut self, wi: usize, wire: &Wire) {
        let spans = wire
            .pieces
            .iter()
            .enumerate()
            .flat_map(|(pi, p)| p.iter().enumerate().map(move |(si, s)| (pi, si, s)))
            .collect::<Vec<_>>();
        let n = spans.len();
        let mut vertices: [Vec<usize>; 2] = [vec![], vec![]];
        for (_, _, s) in &spans {
            for z in 0..2 {
                vertices[z].push(self.model.vertices.len());
                self.model.vertices.push(Vertex {
                    point: point(&s.net[0], &self.height[z]),
                });
            }
        }
        let mut horizontal: [Vec<usize>; 2] = [vec![], vec![]];
        let mut vertical = Vec::new();
        for (i, (_, _, s)) in spans.iter().enumerate() {
            let j = (i + 1) % n;
            for z in 0..2 {
                horizontal[z].push(add_edge(
                    &mut self.model,
                    [vertices[z][i], vertices[z][j]],
                    lifted(&s.net, &self.height[z]),
                ));
            }
            let a = self.model.vertices[vertices[0][i]].point.clone();
            let b = self.model.vertices[vertices[1][i]].point.clone();
            vertical.push(add_edge(
                &mut self.model,
                [vertices[0][i], vertices[1][i]],
                Curve3 {
                    net: [a, b]
                        .map(|p| [p[0].clone(), p[1].clone(), p[2].clone(), Q::one()])
                        .to_vec(),
                },
            ));
        }
        for z in 0..2 {
            let coedges = spans
                .iter()
                .enumerate()
                .map(|(i, (_, _, s))| {
                    Coedge::new(horizontal[z][i], false, Pcurve { net: s.net.clone() })
                })
                .collect();
            self.cap_loops[z].push(add_loop(&mut self.model, coedges));
        }
        for (i, (pi, si, s)) in spans.iter().enumerate() {
            let j = (i + 1) % n;
            self.wall(
                &s.net,
                [horizontal[0][i], horizontal[1][i]],
                [vertical[i], vertical[j]],
            );
            self.wall_sources.push([wi, *pi, *si]);
        }
    }
    /// The two caps close the shell of all wall faces. No Body role is
    /// manufactured from closed incidence alone.
    fn close(&mut self, bounds: [[Q; 2]; 2]) {
        let mut uses = (0..self.model.faces.len())
            .map(|face| FaceUse {
                face,
                reversed: false,
            })
            .collect::<Vec<_>>();
        for z in 0..2 {
            uses.push(FaceUse {
                face: self.model.faces.len(),
                reversed: z == 0,
            });
            self.model.faces.push(Face {
                surface: Surface {
                    kind: Kind::Cap {
                        z: self.height[z].clone(),
                        bounds: bounds.clone(),
                    },
                },
                outer: self.cap_loops[z][0],
                holes: self.cap_loops[z][1..].to_vec(),
            });
        }
        self.model.shells.push(Shell {
            faces: uses,
            closed: true,
        });
    }
}
fn build(
    sources: &[Wire],
    height: &[Q; 2],
    tolerance_mm: f64,
) -> Result<(Topology, Vec<[usize; 3]>, Agreement)> {
    verify_sources(sources, height)?;
    let mut prism = Prism {
        model: Topology {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm,
        },
        height,
        cap_loops: [vec![], vec![]],
        wall_sources: Vec::new(),
    };
    let bounds = cap_bounds(sources)?;
    for (wi, wire) in sources.iter().enumerate() {
        prism.profile(wi, wire);
    }
    prism.close(bounds);
    validate(&prism.model)?;
    let agreement = verify_geometry(&prism.model)?;
    Ok((prism.model, prism.wall_sources, agreement))
}
pub fn extrude(
    sources: &[Wire],
    height: [ExactRatio; 2],
    tolerance_mm: f64,
) -> Result<ShellGeometry> {
    let height = [decode_ratio(&height[0])?, decode_ratio(&height[1])?];
    let (model, wall_sources, agreement) = build(sources, &height, tolerance_mm)?;
    Ok(ShellGeometry {
        sources: sources.to_vec(),
        height,
        model,
        wall_sources,
        agreement,
    })
}
pub fn extrude_binary64(
    sources: &[Wire],
    height: [f64; 2],
    tolerance_mm: f64,
) -> Result<ShellGeometry> {
    let mut exact = [Q::zero(), Q::zero()];
    for i in 0..2 {
        exact[i] = Q::from_f64(height[i]).ok_or_else(|| refused("Nonfinite exact shell height"))?;
    }
    extrude(sources, exact.each_ref().map(ratio), tolerance_mm)
}
impl ShellGeometry {
    /// Simple, mutually disjoint exact boundary loops; containment, material
    /// roles and solid admission remain separate.
    pub fn qualify_profile_jordan(&self, limits: jordan::Limits) -> Result<jordan::JordanWires> {
        self.reverify()?;
        jordan::qualify(&self.sources, limits)
    }
    pub fn model(&self) -> &Topology {
        &self.model
    }
    pub fn source_profiles(&self) -> &[Wire] {
        &self.sources
    }
    pub fn height(&self) -> [ExactRatio; 2] {
        self.height.each_ref().map(ratio)
    }
    pub fn wall_sources(&self) -> &[[usize; 3]] {
        &self.wall_sources
    }
    pub fn agreement(&self) -> &Agreement {
        &self.agreement
    }
    pub fn reverify(&self) -> Result<()> {
        let (model, walls, agreement) =
            build(&self.sources, &self.height, self.model.tolerance_mm)?;
        if model != self.model || walls != self.wall_sources || agreement != self.agreement {
            return Err(refused("Exact source extrusion replay changed"));
        }
        Ok(())
    }
}
