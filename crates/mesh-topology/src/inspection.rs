use crate::{EdgeCounts, EdgeUses, MeshView, Result, check};
use math_core::{cross, norm, sub};

#[derive(Debug, Clone)]
pub struct Inspection {
    pub triangle_count: usize,
    pub vertex_count: usize,
    pub boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub orientation_conflicts: usize,
    pub degenerate_triangles: usize,
    pub closed: bool,
    pub signed_volume_mm3: f64,
}
impl MeshView<'_> {
    pub fn inspect(&self) -> Result<Inspection> {
        self.inspect_with_edges().map(|(report, _)| report)
    }
    pub fn inspect_with_edges(&self) -> Result<(Inspection, EdgeUses)> {
        self.validate()?;
        let reference = if self.positions.is_empty() {
            [0.; 3]
        } else {
            self.point(0)?
        };
        let mut volume = 0.;
        let mut volume_compensation = 0.;
        let mut degenerate = 0;
        for t in self.indices.as_chunks::<3>().0 {
            let a = self.point(t[0])?;
            let b = self.point(t[1])?;
            let c = self.point(t[2])?;
            let ab = sub(b, a);
            let ac = sub(c, a);
            let normal = cross(ab, ac);
            if norm(normal) <= f64::EPSILON * (norm(ab) * norm(ac)).max(1.) {
                degenerate += 1;
            }
            let ar = sub(a, reference);
            let br = sub(b, reference);
            let cr = sub(c, reference);
            let bc = cross(br, cr);
            let term = ar[0] * bc[0] + ar[1] * bc[1] + ar[2] * bc[2] - volume_compensation;
            let next = volume + term;
            volume_compensation = (next - volume) - term;
            volume = next;
        }
        check(
            volume.is_finite(),
            "Mesh volume exceeded finite numeric bounds.",
        )?;
        let edges = EdgeUses::new(self.indices);
        let EdgeCounts {
            boundary,
            non_manifold,
            orientation,
        } = edges.counts();
        Ok((
            Inspection {
                triangle_count: self.indices.len() / 3,
                vertex_count: self.positions.len() / 3,
                boundary_edges: boundary,
                non_manifold_edges: non_manifold,
                orientation_conflicts: orientation,
                degenerate_triangles: degenerate,
                closed: !self.indices.is_empty()
                    && boundary == 0
                    && non_manifold == 0
                    && orientation == 0
                    && degenerate == 0,
                signed_volume_mm3: volume / 6.,
            },
            edges,
        ))
    }

    /// Counts connected triangle components (shells) in the mesh via union-find over shared vertices.
    pub fn connected_components_count(&self) -> Result<usize> {
        self.validate()?;
        if self.indices.is_empty() {
            return Ok(0);
        }
        let v_count = self.positions.len() / 3;
        let mut parent: Vec<usize> = (0..v_count).collect();
        let mut used = vec![false; v_count];
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        for tri in self.indices.as_chunks::<3>().0 {
            let [a, b, c] = *tri;
            used[a] = true;
            used[b] = true;
            used[c] = true;
            let ra = find(&mut parent, a);
            let rb = find(&mut parent, b);
            if ra != rb {
                parent[rb] = ra;
            }
            let rc = find(&mut parent, c);
            if ra != rc {
                parent[rc] = ra;
            }
        }
        let mut roots = std::collections::BTreeSet::new();
        for v in 0..v_count {
            if used[v] {
                roots.insert(find(&mut parent, v));
            }
        }
        Ok(roots.len())
    }

    /// Computes the Euler–Poincaré topological genus $g$ for a closed orientable 2-manifold mesh
    /// from $\chi = V - E + F = 2 c - 2 g$, where $c$ is the number of connected closed shells.
    pub fn genus(&self) -> Result<isize> {
        let (report, edges) = self.inspect_with_edges()?;
        check(
            report.closed,
            "Topological genus requires a closed orientable manifold mesh.",
        )?;
        let mut used_verts = vec![false; self.positions.len() / 3];
        for &idx in self.indices {
            used_verts[idx] = true;
        }
        let v = used_verts.into_iter().filter(|&u| u).count() as isize;
        let e = edges.edge_count() as isize;
        let f = report.triangle_count as isize;
        let c = self.connected_components_count()? as isize;
        let chi = v - e + f;
        Ok(c - chi / 2)
    }
}

/// Counts connected triangle components (shells) in `mesh`.
pub fn connected_components_count(mesh: &MeshView<'_>) -> Result<usize> {
    mesh.connected_components_count()
}

/// Computes the topological genus $g$ of a closed orientable manifold `mesh`.
pub fn genus(mesh: &MeshView<'_>) -> Result<isize> {
    mesh.genus()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connected_components_and_genus_for_tetrahedra() {
        let positions = [
            0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, // tet 1
            20.0, 0.0, 0.0, 30.0, 0.0, 0.0, 20.0, 10.0, 0.0, 20.0, 0.0, 10.0, // tet 2
        ];
        let indices_one = [0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3];
        let one_tet = MeshView::new(&positions[..12], &indices_one);
        assert_eq!(connected_components_count(&one_tet).unwrap(), 1);
        assert_eq!(genus(&one_tet).unwrap(), 0);

        let indices_two = [
            0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3, // tet 1
            4, 6, 5, 4, 5, 7, 5, 6, 7, 6, 4, 7, // tet 2
        ];
        let two_tets = MeshView::new(&positions, &indices_two);
        assert_eq!(connected_components_count(&two_tets).unwrap(), 2);
        assert_eq!(genus(&two_tets).unwrap(), 0);
    }
}
