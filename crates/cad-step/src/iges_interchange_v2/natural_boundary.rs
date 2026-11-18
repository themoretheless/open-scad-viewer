use super::*;

impl Builder<'_> {
    pub(super) fn natural_outer_loop(
        &mut self,
        de: usize,
        surface: &Surface,
        work: &mut usize,
    ) -> Result<usize> {
        let [u0, u1] = [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ];
        let [v0, v1] = [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ];
        let mut coedges = Vec::with_capacity(4);
        for (axis, parameter, backward, a, b) in [
            (Axis::V, v0, false, [u0, v0], [u1, v0]),
            (Axis::U, u1, false, [u1, v0], [u1, v1]),
            (Axis::V, v1, true, [u1, v1], [u0, v1]),
            (Axis::U, u0, true, [u0, v1], [u0, v0]),
        ] {
            let iso = surface.iso(axis, parameter)?;
            if iso
                .control_points
                .iter()
                .all(|point| point == &iso.control_points[0])
            {
                return Err(trimmed_unsupported(
                    de,
                    "collapsed natural boundary at a surface pole",
                ));
            }
            let opposite = iso.reverse()?;
            let mut existing = None;
            for (index, edge) in self.edges.iter().enumerate() {
                graph_work(work, iso.control_points.len())?;
                let reversed = if edge.curve == iso {
                    false
                } else if edge.curve == opposite {
                    true
                } else {
                    continue;
                };
                if existing.replace((index, reversed)).is_some() {
                    return Err(trimmed_unsupported(de, "ambiguous natural boundary edge"));
                }
            }
            let (edge, reversed) = match existing {
                Some(found) => found,
                None => {
                    if self.edges.len() >= MAX_TOPOLOGY {
                        return Err(trimmed_unsupported(
                            de,
                            "natural boundary exceeds edge budget",
                        ));
                    }
                    let (start, end) = CurveDispatcher::endpoints(&iso)?;
                    let a = self.trimmed_vertex(de, start)?;
                    let b = self.trimmed_vertex(de, end)?;
                    let index = self.edges.len();
                    self.edges.push(Edge {
                        curve: iso,
                        vertices: [a, b],
                        degenerate: false,
                    });
                    self.edge_de.push(de);
                    (index, false)
                }
            };
            let uv = Curve::from_polyline(vec![a.to_vec(), b.to_vec()])?;
            coedges.push(Coedge::new(edge, reversed ^ backward, uv));
        }
        let index = self.loops.len();
        self.loops.push(Loop { coedges });
        self.loop_de.push(de);
        Ok(index)
    }
}
