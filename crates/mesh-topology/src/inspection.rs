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
        let mut components = 0usize;
        for v in 0..v_count {
            if used[v] && parent[v] == v {
                components += 1;
            }
        }
        Ok(components)
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
        let mut v = 0isize;
        for &idx in self.indices {
            if !used_verts[idx] {
                used_verts[idx] = true;
                v += 1;
            }
        }
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

/// Statistics of dihedral angles (angle between adjacent face normals in radians and degrees)
/// across all interior manifold edges of a triangle mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DihedralAngleStats {
    pub interior_edge_count: usize,
    pub min_angle_rad: f64,
    pub max_angle_rad: f64,
    pub mean_angle_rad: f64,
    pub min_angle_deg: f64,
    pub max_angle_deg: f64,
    pub mean_angle_deg: f64,
    /// Count of interior feature edges whose dihedral angle exceeds $30^\circ$ ($\pi / 6\text{ rad}$).
    pub sharp_edge_count: usize,
}

impl MeshView<'_> {
    /// Computes dihedral angle statistics across all interior manifold edges (edges shared by
    /// exactly two triangles).
    pub fn dihedral_angle_stats(&self) -> Result<DihedralAngleStats> {
        self.validate()?;
        let tri_count = self.indices.len() / 3;
        let mut normals = Vec::with_capacity(tri_count);
        for t in self.indices.as_chunks::<3>().0 {
            let a = self.point(t[0])?;
            let b = self.point(t[1])?;
            let c = self.point(t[2])?;
            let n = cross(sub(b, a), sub(c, a));
            let len = norm(n);
            if len > 1e-15 {
                normals.push([n[0] / len, n[1] / len, n[2] / len]);
            } else {
                normals.push([0.0, 0.0, 0.0]);
            }
        }
        // Map each undirected edge `(u.min(v), u.max(v))` to `(first_tri, second_tri, valence)`
        // in a pre-sized HashMap with zero per-edge heap allocations!
        let mut edge_faces: std::collections::HashMap<(usize, usize), (usize, usize, u32)> =
            std::collections::HashMap::with_capacity((tri_count * 3) / 2 + 1);
        for (ti, t) in self.indices.as_chunks::<3>().0.iter().enumerate() {
            for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let key = (u.min(v), u.max(v));
                edge_faces
                    .entry(key)
                    .and_modify(|entry| {
                        if entry.2 == 1 {
                            entry.1 = ti;
                        }
                        entry.2 += 1;
                    })
                    .or_insert((ti, 0, 1));
            }
        }
        let mut count = 0usize;
        let mut min_rad = f64::INFINITY;
        let mut max_rad = 0.0_f64;
        let mut sum_rad = 0.0_f64;
        let mut sharp_count = 0usize;
        let sharp_threshold = std::f64::consts::FRAC_PI_6; // 30 degrees

        for &(f0, f1, valence) in edge_faces.values() {
            if valence == 2 {
                let n0 = normals[f0];
                let n1 = normals[f1];
                if norm(n0) <= 1e-12 || norm(n1) <= 1e-12 {
                    continue;
                }
                let cos_theta = (n0[0] * n1[0] + n0[1] * n1[1] + n0[2] * n1[2]).clamp(-1.0, 1.0);
                let angle = cos_theta.acos();
                count += 1;
                min_rad = min_rad.min(angle);
                max_rad = max_rad.max(angle);
                sum_rad += angle;
                if angle > sharp_threshold + 1e-7 {
                    sharp_count += 1;
                }
            }
        }
        let min_angle_rad = if min_rad.is_finite() { min_rad } else { 0.0 };
        let mean_angle_rad = if count > 0 {
            sum_rad / count as f64
        } else {
            0.0
        };
        Ok(DihedralAngleStats {
            interior_edge_count: count,
            min_angle_rad,
            max_angle_rad: max_rad,
            mean_angle_rad,
            min_angle_deg: min_angle_rad.to_degrees(),
            max_angle_deg: max_rad.to_degrees(),
            mean_angle_deg: mean_angle_rad.to_degrees(),
            sharp_edge_count: sharp_count,
        })
    }
}

/// Computes dihedral angle statistics across all interior manifold edges of `mesh`.
pub fn dihedral_angle_stats(mesh: &MeshView<'_>) -> Result<DihedralAngleStats> {
    mesh.dihedral_angle_stats()
}

impl MeshView<'_> {
    /// Computes vertex edge-valence statistics `(min_valence, max_valence, mean_valence, irregular_vertex_count)`
    /// across all referenced vertices in the triangle mesh. For a triangle mesh, an interior vertex
    /// is regular when its edge valence is `6` (and a boundary vertex is regular when its edge valence is `4`).
    pub fn vertex_valence_stats(&self) -> Result<(usize, usize, f64, usize)> {
        self.validate()?;
        if self.indices.is_empty() {
            return Ok((0, 0, 0.0, 0));
        }
        let v_count = self.positions.len() / 3;
        let tri_count = self.indices.len() / 3;
        let mut edge_uses: std::collections::HashMap<(usize, usize), u32> =
            std::collections::HashMap::with_capacity((tri_count * 3) / 2 + 1);
        for t in self.indices.as_chunks::<3>().0 {
            for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                if u != v {
                    let key = (u.min(v), u.max(v));
                    *edge_uses.entry(key).or_insert(0) += 1;
                }
            }
        }
        let mut valence = vec![0usize; v_count];
        let mut is_boundary = vec![false; v_count];
        for (&(u, v), &uses) in &edge_uses {
            valence[u] += 1;
            valence[v] += 1;
            if uses == 1 {
                is_boundary[u] = true;
                is_boundary[v] = true;
            }
        }
        let mut min_val = usize::MAX;
        let mut max_val = 0usize;
        let mut sum_val = 0usize;
        let mut active_verts = 0usize;
        let mut irregular_count = 0usize;
        for (vi, &val) in valence.iter().enumerate() {
            if val == 0 {
                continue;
            }
            active_verts += 1;
            min_val = min_val.min(val);
            max_val = max_val.max(val);
            sum_val += val;
            let regular_val = if is_boundary[vi] { 4 } else { 6 };
            if val != regular_val {
                irregular_count += 1;
            }
        }
        if active_verts == 0 {
            return Ok((0, 0, 0.0, 0));
        }
        let mean_val = sum_val as f64 / active_verts as f64;
        Ok((min_val, max_val, mean_val, irregular_count))
    }
}

/// Computes vertex edge-valence statistics `(min_valence, max_valence, mean_valence, irregular_vertex_count)`
/// across all referenced vertices in `mesh` (where regular triangle-mesh interior valence is `6`).
pub fn vertex_valence_stats(mesh: &MeshView<'_>) -> Result<(usize, usize, f64, usize)> {
    mesh.vertex_valence_stats()
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

        let stats = dihedral_angle_stats(&one_tet).unwrap();
        assert_eq!(stats.interior_edge_count, 6);
        assert_eq!(stats.sharp_edge_count, 6);
        assert!(stats.min_angle_deg >= 90.0 - 1e-6);
        assert!(stats.max_angle_deg > 120.0);

        // Tetrahedron has 4 interior vertices, each of edge valence 3 (all 4 are irregular since 3 != 6)
        let (min_v, max_v, mean_v, irreg) = vertex_valence_stats(&one_tet).unwrap();
        assert_eq!(min_v, 3);
        assert_eq!(max_v, 3);
        assert!((mean_v - 3.0).abs() < 1e-12);
        assert_eq!(irreg, 4);
    }
}
