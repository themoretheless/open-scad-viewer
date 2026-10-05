//! Closed oriented incidence of original surface-composed wires.
//! This is not a certificate of embedded faces, material orientation or volume.
use crate::{
    source_shared_edge::{self, SharedEdge},
    source_world_wire::Wire,
};
use nurbs_core::{curve::Curve, Error, Result};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address {
    pub face: usize,
    pub wire: usize,
    pub edge: usize,
}
pub struct Pair {
    pub uses: [Address; 2],
    pub world: Curve,
    pub world_reversed: [bool; 2],
    pub cutters: [Option<Curve>; 2],
}
pub struct Shell {
    faces: Vec<Vec<Wire>>,
    edges: Vec<SharedEdge>,
    uses: Vec<[Address; 2]>,
    vertices: Vec<Vec<Vec<[usize; 2]>>>,
}
impl Shell {
    pub fn faces(&self) -> &[Vec<Wire>] {
        &self.faces
    }
    pub fn edges(&self) -> &[SharedEdge] {
        &self.edges
    }
    pub fn uses(&self) -> &[[Address; 2]] {
        &self.uses
    }
    pub fn vertices(&self) -> &[Vec<Vec<[usize; 2]>>] {
        &self.vertices
    }
}
pub struct ChartFace {
    pub face: usize,
    pub contraction: Option<nurbs_core::surface_injectivity::Report>,
    pub linear: Option<nurbs_core::surface_linear_monotonicity::Report>,
}
impl ChartFace {
    pub fn injectivity_proven(&self) -> bool {
        self.contraction.as_ref().is_some_and(|r| r.proven)
            || self.linear.as_ref().is_some_and(|r| r.certified)
    }
}
pub struct ChartReport {
    pub all_injective: bool,
    pub spans: usize,
    pub linear_cells: usize,
    pub faces: Vec<ChartFace>,
}
impl Shell {
    /// Prove whole original charts injective, hence any retained source subset.
    /// Does not classify contacts between different faces or certify a volume.
    pub fn inspect_face_charts(
        &self,
        max_spans: usize,
        max_linear_cells: usize,
    ) -> Result<ChartReport> {
        if !(1..=100000).contains(&max_spans) || max_linear_cells > 100000 {
            return Err(error("Choose bounded source chart work"));
        }
        let mut out = ChartReport {
            all_injective: true,
            spans: 0,
            linear_cells: 0,
            faces: Vec::new(),
        };
        for (face, wires) in self.faces.iter().enumerate() {
            let surface = wires[0].edges()[0].surface();
            let contraction = if out.spans < max_spans {
                Some(nurbs_core::surface_injectivity::certify_contraction(
                    surface,
                    max_spans - out.spans,
                )?)
            } else {
                None
            };
            out.spans += contraction.as_ref().map_or(0, |r| r.spans);
            let linear = if !contraction.as_ref().is_some_and(|r| r.proven)
                && out.linear_cells < max_linear_cells
            {
                Some(nurbs_core::surface_linear_monotonicity::inspect_candidate(
                    surface,
                    max_linear_cells - out.linear_cells,
                )?)
            } else {
                None
            };
            out.linear_cells += linear.as_ref().map_or(0, |r| r.cells);
            let result = ChartFace {
                face,
                contraction,
                linear,
            };
            out.all_injective &= result.injectivity_proven();
            out.faces.push(result);
        }
        Ok(out)
    }
}
pub struct Report {
    pub shell: Option<Shell>,
    pub work_used: u64,
    pub root_checks: usize,
    pub uncertain_pair: Option<usize>,
    pub reason: &'static str,
}
fn error(message: &str) -> Error {
    Error::new("BREP_SOURCE_SHELL", message)
}
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}
fn join(parent: &mut [usize], a: usize, b: usize) {
    let a = root(parent, a);
    let b = root(parent, b);
    parent[b] = a;
}
/// Recheck every canonical pair. Each directed use must appear exactly once.
/// Source joins own local vertices; exact opposite pairs own cross-face vertices.
pub fn assemble(faces: &[Vec<Wire>], pairs: &[Pair], max_work: u64) -> Result<Report> {
    if faces.len() < 2 || faces.len() > 4096 || max_work == 0 || max_work > 100_000_000 {
        return Err(error("Choose bounded faces and exact shell work"));
    }
    let mut offsets = Vec::new();
    let mut total = 0;
    for face in faces {
        if face.is_empty() || face.len() > 16 {
            return Err(error("Face needs bounded original wires"));
        }
        let surface = face[0].edges()[0].surface();
        let mut starts = Vec::new();
        for wire in face {
            if wire.edges().iter().any(|e| e.surface() != surface) {
                return Err(error("Face wires must share the original surface"));
            }
            starts.push(total);
            total += wire.edges().len();
            if total > 65536 {
                return Err(error("Source shell incidence work limit"));
            }
        }
        offsets.push(starts);
    }
    if pairs.len().checked_mul(2) != Some(total) {
        return Err(error("Every directed source edge requires one partner"));
    }
    let lookup = |a: Address| -> Result<usize> {
        let wire = faces
            .get(a.face)
            .and_then(|f| f.get(a.wire))
            .ok_or_else(|| error("Unknown source wire address"))?;
        if a.edge >= wire.edges().len() {
            return Err(error("Unknown source edge address"));
        }
        Ok(offsets[a.face][a.wire] + a.edge)
    };
    let mut seen = vec![false; total];
    for p in pairs {
        if p.uses[0].face == p.uses[1].face {
            return Err(error("Pair must join distinct faces"));
        }
        for &a in &p.uses {
            let i = lookup(a)?;
            if seen[i] {
                return Err(error("Source edge use paired more than once"));
            }
            seen[i] = true;
        }
    }
    let mut out = Report {
        shell: None,
        work_used: 0,
        root_checks: 0,
        uncertain_pair: None,
        reason: "source-shell-pair-unqualified",
    };
    let mut parent: Vec<_> = (0..total).collect();
    let mut face_parent: Vec<_> = (0..faces.len()).collect();
    let mut edges = Vec::new();
    for (i, p) in pairs.iter().enumerate() {
        if out.work_used == max_work {
            out.uncertain_pair = Some(i);
            out.reason = "source-shell-work-limit";
            return Ok(out);
        }
        let sources = p.uses.map(|a| &faces[a.face][a.wire].edges()[a.edge]);
        let report = source_shared_edge::qualify_with_cutters(
            &p.world,
            sources,
            p.world_reversed,
            [p.cutters[0].as_ref(), p.cutters[1].as_ref()],
            max_work - out.work_used,
        )?;
        out.work_used += report.work_used;
        out.root_checks += report.root_checks;
        let Some(edge) = report.edge else {
            out.uncertain_pair = Some(i);
            out.reason = report.reason;
            return Ok(out);
        };
        let endpoint = |a: Address, end: usize| {
            offsets[a.face][a.wire] + faces[a.face][a.wire].vertices()[a.edge][end]
        };
        join(&mut parent, endpoint(p.uses[0], 0), endpoint(p.uses[1], 1));
        join(&mut parent, endpoint(p.uses[0], 1), endpoint(p.uses[1], 0));
        join(&mut face_parent, p.uses[0].face, p.uses[1].face);
        edges.push(edge);
    }
    let first = root(&mut face_parent, 0);
    if (1..faces.len()).any(|i| root(&mut face_parent, i) != first) {
        out.reason = "source-shell-disconnected";
        return Ok(out);
    }
    let mut ids = BTreeMap::new();
    let vertices = faces
        .iter()
        .enumerate()
        .map(|(f, face)| {
            face.iter()
                .enumerate()
                .map(|(w, wire)| {
                    wire.vertices()
                        .iter()
                        .map(|v| {
                            v.map(|i| {
                                let owner = root(&mut parent, offsets[f][w] + i);
                                let next = ids.len();
                                *ids.entry(owner).or_insert(next)
                            })
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    out.shell = Some(Shell {
        faces: faces.to_vec(),
        edges,
        uses: pairs.iter().map(|p| p.uses).collect(),
        vertices,
    });
    out.reason = "source-closed-oriented-incidence-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_boundary_fragment::{Endpoint, Fragment};
    use nurbs_core::surface::Surface;
    fn tetrahedron() -> (Vec<Vec<Wire>>, Vec<Pair>) {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let triangles = [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];
        let mut pending = BTreeMap::new();
        let mut pairs = Vec::new();
        let mut faces = Vec::new();
        for (face, t) in triangles.iter().enumerate() {
            let [a, b, c] = t.map(|i| points[i]);
            let fourth: Vec<_> = (0..3).map(|i| b[i] + c[i] - a[i]).collect();
            let s = Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: vec![0., 0., 1., 1.],
                control_points: vec![vec![a.to_vec(), c.to_vec()], vec![b.to_vec(), fourth]],
                weights: vec![vec![1.; 2]; 2],
                periodic_u: false,
                periodic_v: false,
            };
            let uv = [[0., 0.], [1., 0.], [0., 1.]];
            let mut fragments = Vec::new();
            for edge in 0..3 {
                let next = (edge + 1) % 3;
                let pc = Curve::from_polyline(vec![uv[edge].to_vec(), uv[next].to_vec()]).unwrap();
                fragments.push(
                    Fragment::new(&s, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.))
                        .unwrap(),
                );
                let address = Address {
                    face,
                    wire: 0,
                    edge,
                };
                let key = (t[edge].min(t[next]), t[edge].max(t[next]));
                if let Some((old, world)) = pending.remove(&key) {
                    pairs.push(Pair {
                        uses: [old, address],
                        world,
                        world_reversed: [false, true],
                        cutters: [None, None],
                    });
                } else {
                    pending.insert(
                        key,
                        (
                            address,
                            Curve::from_polyline(vec![
                                points[t[edge]].to_vec(),
                                points[t[next]].to_vec(),
                            ])
                            .unwrap(),
                        ),
                    );
                }
            }
            faces.push(vec![Wire::new(&fragments).unwrap()]);
        }
        assert!(pending.is_empty());
        (faces, pairs)
    }
    #[test]
    fn tetrahedron_pairs_produce_shared_vertex_owners_and_refuse_missing_or_duplicate_uses() {
        let (faces, mut pairs) = tetrahedron();
        let report = assemble(&faces, &pairs, 100_000_000).unwrap();
        assert!(
            report.shell.is_some(),
            "{} pair {:?}",
            report.reason,
            report.uncertain_pair
        );
        let shell = report.shell.unwrap();
        assert_eq!(shell.edges().len(), 6);
        let owners: std::collections::BTreeSet<_> = shell
            .vertices()
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .copied()
            .collect();
        assert_eq!(owners.len(), 4);
        for pair in shell.uses() {
            let vertex = |a: Address| shell.vertices()[a.face][a.wire][a.edge];
            assert_eq!(vertex(pair[0]), [vertex(pair[1])[1], vertex(pair[1])[0]]);
        }
        assert!(assemble(&faces, &pairs[..5], 100_000_000).is_err());
        pairs[1].uses[0] = pairs[0].uses[0];
        assert!(assemble(&faces, &pairs, 100_000_000).is_err());
    }
    #[test]
    fn root_partition_of_shared_edge_preserves_closed_shell_and_original_definitions() {
        use crate::source_boundary_fragment::Role;
        let (mut faces, mut pairs) = tetrahedron();
        let old = pairs.remove(0);
        for (i, address) in old.uses.iter().enumerate() {
            let edge = &faces[address.face][address.wire].edges()[address.edge];
            let c = edge.curve();
            let t = if i == 0 { 0.25 } else { 0.75 };
            let a = &c.control_points[0];
            let b = &c.control_points[1];
            let point = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
            let normal = [-(b[1] - a[1]), b[0] - a[0]];
            let cutter = Curve::from_polyline(vec![
                vec![point[0] - normal[0], point[1] - normal[1]],
                vec![point[0] + normal[0], point[1] + normal[1]],
            ])
            .unwrap();
            let p =
                crate::source_contact_point::qualify(edge.surface(), c, &cutter, [[0., 1.]; 2], 16)
                    .unwrap()
                    .point
                    .unwrap();
            let parts = edge.split_at(&p, Role::Boundary).unwrap();
            assert_eq!(parts[0].curve(), c);
            assert_eq!(parts[1].curve(), c);
            let mut edges = faces[address.face][address.wire].edges().to_vec();
            edges.splice(address.edge..address.edge + 1, parts);
            faces[address.face][address.wire] = Wire::new(&edges).unwrap();
        }
        for pair in &mut pairs {
            for address in &mut pair.uses {
                for split in old.uses {
                    if address.face == split.face
                        && address.wire == split.wire
                        && address.edge > split.edge
                    {
                        address.edge += 1;
                    }
                }
            }
        }
        let [a, b] = old.uses;
        pairs.push(Pair {
            uses: [
                a,
                Address {
                    edge: b.edge + 1,
                    ..b
                },
            ],
            world: old.world.clone(),
            world_reversed: old.world_reversed,
            cutters: [None, None],
        });
        pairs.push(Pair {
            uses: [
                Address {
                    edge: a.edge + 1,
                    ..a
                },
                b,
            ],
            world: old.world,
            world_reversed: old.world_reversed,
            cutters: [None, None],
        });
        let r = assemble(&faces, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_some(), "{} {:?}", r.reason, r.uncertain_pair);
        let shell = r.shell.unwrap();
        assert_eq!(shell.edges().len(), 7);
        let owners: std::collections::BTreeSet<_> = shell
            .vertices()
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .copied()
            .collect();
        assert_eq!(owners.len(), 5);
        for pair in shell.uses() {
            let vertex = |a: Address| shell.vertices()[a.face][a.wire][a.edge];
            assert_eq!(vertex(pair[0]), [vertex(pair[1])[1], vertex(pair[1])[0]]);
        }
    }
    #[test]
    fn chart_audit_catches_folded_geometry_even_when_exact_boundary_incidence_closes() {
        let (mut faces, pairs) = tetrahedron();
        let shell = assemble(&faces, &pairs, 100_000_000)
            .unwrap()
            .shell
            .unwrap();
        let charts = shell.inspect_face_charts(4, 0).unwrap();
        assert!(charts.all_injective);
        assert_eq!(charts.spans, 4);
        let stopped = shell.inspect_face_charts(1, 0).unwrap();
        assert!(!stopped.all_injective);
        assert_eq!(stopped.spans, 1);
        assert_eq!(stopped.faces.len(), 4);
        assert!(stopped.faces[1].contraction.is_none());
        let s = faces[0][0].edges()[0].surface();
        let a = &s.control_points[0][0];
        let b = &s.control_points[1][0];
        let c = &s.control_points[0][1];
        // Affine face plus exact polynomial bubble 16*u*v*(1-u-v)
        // in its u direction. All three original triangle boundaries stay
        // unchanged while an interior directional derivative changes sign.
        let controls = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let u = i as f64 / 2.;
                        let v = j as f64 / 2.;
                        let delta = 16.
                            * (u * v - if i == 2 { v } else { 0. } - if j == 2 { u } else { 0. });
                        (0..3)
                            .map(|k| a[k] + (u + delta) * (b[k] - a[k]) + v * (c[k] - a[k]))
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let folded = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: controls,
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        assert_eq!(
            folded.evaluate(0.375, 0.25).unwrap().point,
            folded.evaluate(0.625, 0.25).unwrap().point
        );
        let edges: Vec<_> = faces[0][0]
            .edges()
            .iter()
            .map(|e| {
                Fragment::new(
                    &folded,
                    e.curve(),
                    e.endpoints()[0].clone(),
                    e.endpoints()[1].clone(),
                )
                .unwrap()
            })
            .collect();
        faces[0][0] = Wire::new(&edges).unwrap();
        let r = assemble(&faces, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_some(), "{} {:?}", r.reason, r.uncertain_pair);
        let charts = r.shell.unwrap().inspect_face_charts(4, 0).unwrap();
        assert!(!charts.all_injective);
        assert!(!charts.faces[0].injectivity_proven());
        assert!(charts.faces[1..].iter().all(|f| f.injectivity_proven()));
    }
    #[test]
    fn disconnected_closed_components_and_exhausted_work_are_not_a_shell() {
        let (mut faces, mut pairs) = tetrahedron();
        let stopped = assemble(&faces, &pairs, 1).unwrap();
        assert!(stopped.shell.is_none() && stopped.uncertain_pair.is_some());
        assert!(stopped.work_used <= 1);
        let (other, mut more) = tetrahedron();
        for pair in &mut more {
            for address in &mut pair.uses {
                address.face += faces.len();
            }
        }
        faces.extend(other);
        pairs.extend(more);
        let r = assemble(&faces, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_none());
        assert_eq!(r.reason, "source-shell-disconnected");
    }
    #[test]
    fn exact_geometry_mismatch_never_produces_partial_shell() {
        let (faces, mut pairs) = tetrahedron();
        pairs[0].world.control_points[0][0] += 1e-12;
        let r = assemble(&faces, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_none());
        assert_eq!(r.uncertain_pair, Some(0));
    }
}
