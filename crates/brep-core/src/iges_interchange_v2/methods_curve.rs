//! iges interchange v2 operations: curve.
use super::*;
impl<'a> Builder<'a> {
pub(super) fn curve(&self, de: usize, dim: usize) -> Result<Curve> {
        CurveDispatcher {
            entities: self.e,
            dim,
            scale: if dim == 3 { self.scale } else { 1. },
            tolerance_mm: self.merge_tolerance_mm,
            work: 0,
        }
        .dispatch(de)
    }
pub(super) fn vertex_list(&mut self, de: usize) -> Result<Vec<usize>> {
        let v = &entity(self.e, de, 502)?.values;
        let n = u(v, 1, "vertex count")?;
        if n > MAX_TOPOLOGY || v.len() != 2 + 3 * n {
            return Err(refuse("Malformed IGES vertex list"));
        }
        let mut out = Vec::new();
        for i in 0..n {
            let index = self.vertices.len();
            self.vertices.push(Vertex {
                point: [
                    f(v, 2 + 3 * i, "vertex X")? * self.scale,
                    f(v, 3 + 3 * i, "vertex Y")? * self.scale,
                    f(v, 4 + 3 * i, "vertex Z")? * self.scale,
                ],
            });
            self.vertex_de.push(de);
            out.push(index);
        }
        Ok(out)
    }
pub(super) fn build(mut self) -> Result<(Model, TopologyMapping)> {
        let mut vertex_lists = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty == 502 {
                vertex_lists.insert(de, self.vertex_list(de)?);
            }
        }
        let mut edge_lists = BTreeMap::<(usize, usize), usize>::new();
        for (&de, e) in self.e {
            if e.directory.ty != 504 {
                continue;
            }
            let v = &e.values;
            let n = u(v, 1, "edge count")?;
            if v.len() != 2 + 5 * n {
                return Err(refuse("Malformed IGES edge list"));
            }
            for i in 0..n {
                let at = 2 + 5 * i;
                let curve = self.curve(u(v, at, "edge curve")?, 3)?;
                let vl0 = u(v, at + 1, "start vertex list")?;
                let vi0 = u(v, at + 2, "start vertex index")?;
                let vl1 = u(v, at + 3, "end vertex list")?;
                let vi1 = u(v, at + 4, "end vertex index")?;
                let a = *vertex_lists
                    .get(&vl0)
                    .and_then(|x| x.get(vi0.checked_sub(1)?))
                    .ok_or_else(|| refuse("Invalid IGES start vertex index"))?;
                let b = *vertex_lists
                    .get(&vl1)
                    .and_then(|x| x.get(vi1.checked_sub(1)?))
                    .ok_or_else(|| refuse("Invalid IGES end vertex index"))?;
                let index = self.edges.len();
                self.edges.push(Edge {
                    degenerate: a == b,
                    vertices: [a, b],
                    curve,
                });
                self.edge_de.push(de);
                edge_lists.insert((de, i + 1), index);
            }
        }
        let mut loop_map = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty != 508 {
                continue;
            }
            let v = &e.values;
            let n = u(v, 1, "loop member count")?;
            let mut at = 2;
            let mut coedges = Vec::new();
            for _ in 0..n {
                if u(v, at, "loop member type")? != 0 {
                    return Err(refuse("Vertex-only IGES loop member is unsupported"));
                }
                let el = u(v, at + 1, "loop edge list")?;
                let ei = u(v, at + 2, "loop edge index")?;
                let reversed = u(v, at + 3, "loop orientation")? == 0;
                let pcount = u(v, at + 4, "loop pcurve count")?;
                if pcount != 1 {
                    return Err(refuse("Each IGES loop edge requires exactly one pcurve"));
                }
                let iso = u(v, at + 5, "loop pcurve isoparametric")?;
                if iso > 1 {
                    return Err(refuse("Invalid IGES isoparametric flag"));
                }
                let pcurve = self.curve(u(v, at + 6, "loop pcurve")?, 2)?;
                let edge = *edge_lists
                    .get(&(el, ei))
                    .ok_or_else(|| refuse("Invalid IGES edge-list reference"))?;
                coedges.push(Coedge::new(edge, reversed, pcurve));
                at += 7;
            }
            if at != v.len() {
                return Err(refuse("Trailing IGES loop parameters"));
            }
            let index = self.loops.len();
            self.loops.push(Loop { coedges });
            self.loop_de.push(de);
            loop_map.insert(de, index);
        }
        let mut face_map = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty != 510 {
                continue;
            }
            let v = &e.values;
            let surface = surface128(self.e, u(v, 1, "face surface")?, self.scale)?;
            let n = u(v, 2, "face loop count")?;
            if n == 0 || v.len() != 4 + n {
                return Err(refuse("Malformed IGES face"));
            }
            let loops = (0..n)
                .map(|i| {
                    loop_map
                        .get(&u(v, 4 + i, "face loop")?)
                        .copied()
                        .ok_or_else(|| refuse("Missing IGES face loop"))
                })
                .collect::<Result<Vec<_>>>()?;
            for loop_index in &loops {
                for coedge in &self.loops[*loop_index].coedges {
                    verify_correspondence(
                        &self.edges[coedge.edge],
                        &coedge.pcurve,
                        &surface,
                        coedge.reversed,
                    )?;
                }
            }
            let index = self.faces.len();
            self.faces.push(Face {
                surface,
                outer: loops[0],
                holes: loops[1..].to_vec(),
            });
            self.face_de.push(de);
            face_map.insert(de, index);
        }
        let mut shell_map = BTreeMap::new();
        for (&de, e) in self.e {
            if e.directory.ty != 514 {
                continue;
            }
            let v = &e.values;
            let n = u(v, 1, "shell face count")?;
            if n == 0 || v.len() != 2 + 2 * n {
                return Err(refuse("Malformed IGES shell"));
            }
            let faces = (0..n)
                .map(|i| {
                    Ok(FaceUse {
                        face: *face_map
                            .get(&u(v, 2 + 2 * i, "shell face")?)
                            .ok_or_else(|| refuse("Missing IGES shell face"))?,
                        reversed: u(v, 3 + 2 * i, "shell orientation")? == 0,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let index = self.shells.len();
            self.shells.push(Shell {
                faces,
                closed: true,
            });
            self.shell_de.push(de);
            shell_map.insert(de, index);
        }
        for (&de, e) in self.e {
            if e.directory.ty != 186 {
                continue;
            }
            let v = &e.values;
            let outer = *shell_map
                .get(&u(v, 1, "solid outer shell")?)
                .ok_or_else(|| refuse("Missing IGES outer shell"))?;
            let n = u(v, 3, "solid void count")?;
            if v.len() != 4 + 2 * n {
                return Err(refuse("Malformed IGES manifold solid B-rep"));
            }
            let inners = (0..n)
                .map(|i| {
                    shell_map
                        .get(&u(v, 4 + 2 * i, "void shell")?)
                        .copied()
                        .ok_or_else(|| refuse("Missing IGES void shell"))
                })
                .collect::<Result<Vec<_>>>()?;
            self.bodies.push(Body {
                outer_shell: outer,
                inner_shells: inners,
            });
            self.body_de.push(de);
        }
        if self.bodies.is_empty() {
            return Err(refuse("IGES contains no manifold solid B-rep (186)"));
        }
        let count = self.vertices.len()
            + self.edges.len()
            + self.loops.len()
            + self.faces.len()
            + self.shells.len()
            + self.bodies.len();
        if count > MAX_TOPOLOGY {
            return Err(refuse("IGES topology budget exceeds 32768 entities"));
        }
        let mut model = Model(
            brep_topology::Model {
                vertices: self.vertices,
                edges: self.edges,
                loops: self.loops,
                faces: self.faces,
                shells: self.shells,
                bodies: self.bodies,
                tolerance_mm: 1e-7,
            },
            TopologyIds::default(),
        );
        model.rebuild_topology_ids();
        model.validate()?;
        let mut map = Vec::new();
        for (kind, des) in [
            (TopoKind::Vertex, &self.vertex_de),
            (TopoKind::Edge, &self.edge_de),
            (TopoKind::Loop, &self.loop_de),
            (TopoKind::Face, &self.face_de),
            (TopoKind::Shell, &self.shell_de),
            (TopoKind::Body, &self.body_de),
        ] {
            map.extend(des.iter().enumerate().map(|(i, de)| (kind, i, *de)));
        }
        Ok((model, map))
    }
}
